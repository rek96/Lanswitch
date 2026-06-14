// LANSwitch settings UI. Talks to the Rust backend via the global Tauri API
// (withGlobalTauri = true).
const invoke = window.__TAURI__.core.invoke;
const listen = window.__TAURI__.event && window.__TAURI__.event.listen;
const openUrl = window.__TAURI__.opener && window.__TAURI__.opener.openUrl;

const DNS_PRESETS = {
  "": [],
  cloudflare: ["1.1.1.1", "1.0.0.1"],
  google: ["8.8.8.8", "8.8.4.4"],
  quad9: ["9.9.9.9", "149.112.112.112"],
  // "custom" => read the two custom inputs
};

let state = { version: 1, presets: [] };
let interfaces = [];

const $ = (sel) => document.querySelector(sel);

// --------------------------------------------------------------------------
// Load
// --------------------------------------------------------------------------
async function load() {
  try {
    state = await invoke("cmd_get_presets");
  } catch (e) {
    toast("Couldn't load presets: " + e, "err");
    state = { version: 1, presets: [] };
  }
  renderPresets();
  await rescan();
}

async function rescan() {
  try {
    interfaces = await invoke("cmd_list_interfaces");
  } catch (e) {
    interfaces = [];
    toast("Couldn't list interfaces: " + e, "err");
  }
  renderInterfaces();
  populateCustomInterfaces();
}

// --------------------------------------------------------------------------
// Presets editor
// --------------------------------------------------------------------------
function renderPresets() {
  const wrap = $("#presets");
  wrap.innerHTML = "";
  state.presets.forEach((p, i) => wrap.appendChild(presetRow(p, i)));
}

function presetRow(p, i) {
  const item = document.createElement("div");
  item.className = "preset-item";

  const row = document.createElement("div");
  row.className = "preset-row";

  // color dot (acts as a color picker)
  const color = document.createElement("input");
  color.type = "color";
  color.className = "dot";
  color.value = p.color || "#6B7280";
  color.oninput = () => { p.color = color.value; };

  const name = textInput(p.name || "", (v) => (p.name = v), "Name");

  const mode = document.createElement("select");
  ["static", "dhcp"].forEach((m) => {
    const o = document.createElement("option");
    o.value = m; o.textContent = m === "static" ? "Static" : "DHCP";
    if (p.mode === m) o.selected = true;
    mode.appendChild(o);
  });
  mode.onchange = () => { p.mode = mode.value; renderPresets(); };

  const ip = textInput(p.ip || "", (v) => (p.ip = v), "192.168.0.245");
  ip.disabled = p.mode !== "static";

  const prefixWrap = document.createElement("div");
  prefixWrap.className = "prefix-wrap";
  const slash = document.createElement("span");
  slash.textContent = "/";
  const prefix = document.createElement("input");
  prefix.type = "number"; prefix.min = "0"; prefix.max = "32";
  prefix.value = p.prefix ?? 24;
  prefix.disabled = p.mode !== "static";
  prefix.oninput = () => { p.prefix = parseInt(prefix.value || "24", 10); };
  prefixWrap.append(slash, prefix);

  // DNS disclosure (static only — DHCP gets DNS automatically)
  const dnsPanel = document.createElement("div");
  dnsPanel.className = "preset-dns hidden";
  dnsPanel.appendChild(buildDnsEditor(p));

  const dnsToggle = document.createElement("button");
  dnsToggle.className = "small dns-toggle";
  dnsToggle.textContent = dnsSummaryLabel(p);
  dnsToggle.disabled = p.mode !== "static";
  dnsToggle.onclick = () => {
    dnsPanel.classList.toggle("hidden");
  };

  const del = document.createElement("button");
  del.className = "icon-danger"; del.textContent = "✕"; del.title = "Delete preset";
  del.onclick = () => { state.presets.splice(i, 1); renderPresets(); };

  row.append(color, name, mode, ip, prefixWrap, dnsToggle, del);
  item.append(row, dnsPanel);
  return item;
}

// Short label for the DNS toggle button, reflecting the preset's current DNS.
function dnsSummaryLabel(p) {
  if (p.dns_clear) return "DNS: auto";
  if (p.dns && p.dns.length) {
    const key = dnsKeyFor(p.dns);
    return "DNS: " + (key ? capitalize(key) : "custom");
  }
  return "DNS: —";
}
function capitalize(s) { return s.charAt(0).toUpperCase() + s.slice(1); }

// Which named DNS preset (if any) matches a server list.
function dnsKeyFor(dns) {
  for (const [k, v] of Object.entries(DNS_PRESETS)) {
    if (k && JSON.stringify(v) === JSON.stringify(dns)) return k;
  }
  return null;
}

// A reusable DNS editor bound to an object with `.dns` and `.dns_clear`.
function buildDnsEditor(obj) {
  const wrap = document.createElement("div");
  wrap.className = "dns-editor";

  const label = document.createElement("label");
  label.textContent = "DNS servers";

  const sel = document.createElement("select");
  [
    ["", "Leave unchanged"],
    ["clear", "Clear (set automatic)"],
    ["cloudflare", "Cloudflare — 1.1.1.1 / 1.0.0.1"],
    ["google", "Google — 8.8.8.8 / 8.8.4.4"],
    ["quad9", "Quad9 — 9.9.9.9 / 149.112.112.112"],
    ["custom", "Custom…"],
  ].forEach(([v, text]) => {
    const o = document.createElement("option");
    o.value = v; o.textContent = text;
    sel.appendChild(o);
  });

  const customRow = document.createElement("div");
  customRow.className = "row";
  const d1 = textInput((obj.dns && obj.dns[0]) || "", () => write(), "Primary DNS");
  const d2 = textInput((obj.dns && obj.dns[1]) || "", () => write(), "Secondary DNS (optional)");
  customRow.append(d1, d2);

  // initial selection
  let initial = "";
  if (obj.dns_clear) initial = "clear";
  else if (obj.dns && obj.dns.length) initial = dnsKeyFor(obj.dns) || "custom";
  sel.value = initial;
  customRow.classList.toggle("hidden", initial !== "custom");

  function write() {
    const v = sel.value;
    if (v === "clear") { obj.dns = []; obj.dns_clear = true; }
    else if (v === "custom") {
      obj.dns = [d1.value, d2.value].map((s) => s.trim()).filter(Boolean);
      obj.dns_clear = false;
    } else if (v === "") { obj.dns = []; obj.dns_clear = false; }
    else { obj.dns = DNS_PRESETS[v].slice(); obj.dns_clear = false; }
    customRow.classList.toggle("hidden", v !== "custom");
  }
  sel.onchange = write;

  wrap.append(label, sel, customRow);
  return wrap;
}

function textInput(value, onInput, placeholder) {
  const el = document.createElement("input");
  el.type = "text"; el.value = value; el.placeholder = placeholder || "";
  el.oninput = () => onInput(el.value.trim());
  return el;
}

function addPreset() {
  const id = "preset-" + Math.random().toString(36).slice(2, 7);
  state.presets.push({ id, name: "New preset", color: "#3B82F6", mode: "static", ip: "192.168.0.245", prefix: 24 });
  renderPresets();
}

async function savePresets() {
  // basic id hygiene: ensure ids are unique and present
  const seen = new Set();
  for (const p of state.presets) {
    if (!p.id || seen.has(p.id)) p.id = "preset-" + Math.random().toString(36).slice(2, 7);
    seen.add(p.id);
  }
  try {
    await invoke("cmd_save_presets", { file: state });
    await invoke("cmd_refresh_tray");
    toast("Saved", "ok");
  } catch (e) {
    toast("Save failed: " + e, "err");
  }
}

// --------------------------------------------------------------------------
// Interfaces + quick apply
// --------------------------------------------------------------------------
function renderInterfaces() {
  const wrap = $("#interfaces");
  wrap.innerHTML = "";
  if (!interfaces.length) {
    wrap.innerHTML = '<p class="hint">No interfaces found.</p>';
    return;
  }
  interfaces.forEach((iface) => wrap.appendChild(interfaceRow(iface)));
}

function interfaceRow(iface) {
  const row = document.createElement("div");
  row.className = "iface-row";

  const left = document.createElement("div");
  const name = document.createElement("div");
  name.className = "iface-name";
  name.textContent = iface.name;
  const meta = document.createElement("div");
  meta.className = "iface-meta";
  meta.textContent = `${iface.kind || "Network"} · ${iface.current_ip || "no IP"}`;
  left.append(name, meta);

  const badge = document.createElement("span");
  badge.className = "badge" + (iface.connected ? " up" : "");
  badge.textContent = iface.connected ? "connected" : "down";

  const select = document.createElement("select");
  const ph = document.createElement("option");
  ph.value = ""; ph.textContent = "Apply preset…"; ph.disabled = true; ph.selected = true;
  select.appendChild(ph);
  state.presets.forEach((p) => {
    const o = document.createElement("option");
    o.value = p.id; o.textContent = p.name;
    select.appendChild(o);
  });
  select.onchange = async () => {
    const presetId = select.value;
    select.value = "";
    if (!presetId) return;
    const before = ipOf(iface.name);
    try {
      await invoke("cmd_apply", { presetId, interface: iface.name });
      await rescan();
      showChange({ interface: iface.name, before, after: ipOf(iface.name), ok: true });
    } catch (e) {
      showChange({ interface: iface.name, before, after: before, ok: false, error: String(e) });
    }
  };

  row.append(left, badge, select);
  return row;
}

// --------------------------------------------------------------------------
// Quick custom apply
// --------------------------------------------------------------------------
function populateCustomInterfaces() {
  const sel = $("#customIface");
  const prev = sel.value;
  sel.innerHTML = "";
  interfaces.forEach((iface) => {
    const o = document.createElement("option");
    o.value = iface.name;
    o.textContent = iface.current_ip ? `${iface.name} (${iface.current_ip})` : iface.name;
    sel.appendChild(o);
  });
  if (prev && interfaces.some((i) => i.name === prev)) sel.value = prev;
}

function resolveSubnet() {
  const sel = $("#customSubnet").value;
  return sel === "custom" ? $("#customSubnetText").value.trim() : sel;
}

function dnsSelection() {
  const sel = $("#customDns").value;
  if (sel === "clear") return { dns: [], dnsClear: true };
  if (sel === "custom") {
    const dns = [$("#customDns1").value, $("#customDns2").value]
      .map((s) => s.trim())
      .filter(Boolean);
    return { dns, dnsClear: false };
  }
  return { dns: DNS_PRESETS[sel] || [], dnsClear: false };
}

function readCustom() {
  const { dns, dnsClear } = dnsSelection();
  return {
    interface: $("#customIface").value,
    ip: $("#customIp").value.trim(),
    subnet: resolveSubnet(),
    gateway: $("#customGw").value.trim() || null,
    dns,
    dnsClear,
  };
}

// Current known IP for an interface name (from the last scan).
function ipOf(name) {
  const f = interfaces.find((i) => i.name === name);
  return (f && f.current_ip) || "no IP";
}

// "What changed" banner shown after any apply (in-window or from the tray).
function showChange(detail) {
  const el = $("#change-banner");
  const { interface: iface, before, after, ok, error } = detail;
  el.classList.remove("hidden", "ok", "bad");
  if (ok) {
    el.classList.add("ok");
    const same = before === after;
    el.innerHTML =
      `<span class="ci">${iface}</span> ` +
      `<span class="bi">${before}</span> <span class="arrow">→</span> ` +
      `<span class="ai">${after}</span>` +
      (same ? ' <span class="note">(unchanged)</span>' : "");
  } else {
    el.classList.add("bad");
    el.innerHTML = `<span class="ci">${iface}</span> not changed — ${error || "failed"}`;
  }
  clearTimeout(showChange._t);
  showChange._t = setTimeout(() => el.classList.add("hidden"), 6000);
}

// --- IP math (mirrors the Rust validation, for the live preview) ---
function parseIpv4(s) {
  const parts = (s || "").trim().split(".");
  if (parts.length !== 4) return null;
  const o = parts.map((p) => (/^\d{1,3}$/.test(p) ? parseInt(p, 10) : -1));
  if (o.some((n) => n < 0 || n > 255)) return null;
  return o;
}
function maskToPrefix(octets) {
  const bits = ((octets[0] << 24) | (octets[1] << 16) | (octets[2] << 8) | octets[3]) >>> 0;
  let ones = 0, seenZero = false;
  for (let i = 31; i >= 0; i--) {
    if ((bits >>> i) & 1) {
      if (seenZero) return null; // non-contiguous mask
      ones++;
    } else seenZero = true;
  }
  return ones;
}
function subnetToPrefix(input) {
  const s = (input || "").trim().replace(/^\//, "");
  if (s === "") return null;
  if (s.includes(".")) {
    const o = parseIpv4(s);
    return o ? maskToPrefix(o) : null;
  }
  if (!/^\d{1,2}$/.test(s)) return null;
  const p = parseInt(s, 10);
  return p >= 0 && p <= 32 ? p : null;
}
function prefixToMask(p) {
  const bits = p === 0 ? 0 : (0xffffffff << (32 - p)) >>> 0;
  return [24, 16, 8, 0].map((sh) => (bits >>> sh) & 255).join(".");
}

function updatePreview() {
  const el = $("#customPreview");
  const ip = $("#customIp").value.trim();
  const prefix = subnetToPrefix(resolveSubnet());
  const ipOk = parseIpv4(ip) !== null;

  if (!ip || resolveSubnet() === "") {
    el.className = "preview muted";
    el.textContent = "Enter an IP and subnet to preview…";
    return;
  }
  if (!ipOk) return setPreview("bad", `Invalid IP address: ${ip || "(empty)"}`);
  if (prefix === null) return setPreview("bad", `Invalid subnet: ${resolveSubnet()}`);

  const gw = $("#customGw").value.trim();
  if (gw && !parseIpv4(gw)) return setPreview("bad", `Invalid gateway: ${gw}`);

  const { dns, dnsClear } = dnsSelection();
  let dnsText = "DNS unchanged";
  if (dnsClear) dnsText = "DNS automatic";
  else if (dns.length) dnsText = "DNS " + dns.join(", ");
  const badDns = dns.find((d) => !parseIpv4(d));
  if (badDns) return setPreview("bad", `Invalid DNS server: ${badDns}`);

  const gwText = gw ? `  ·  gw ${gw}` : "  ·  no gateway";
  setPreview("ok", `${ip}/${prefix}  ·  mask ${prefixToMask(prefix)}${gwText}  ·  ${dnsText}`);
}
function setPreview(cls, text) {
  const el = $("#customPreview");
  el.className = "preview " + cls;
  el.textContent = text;
}

async function customApply() {
  const cfg = readCustom();
  if (!cfg.interface) return toast("Pick an interface first", "err");
  if (!cfg.ip) return toast("Enter an IP address", "err");
  if (!cfg.subnet) return toast("Enter a subnet", "err");
  const before = ipOf(cfg.interface);
  try {
    await invoke("cmd_apply_custom", cfg);
    await rescan();
    showChange({ interface: cfg.interface, before, after: ipOf(cfg.interface), ok: true });
  } catch (e) {
    showChange({ interface: cfg.interface, before, after: before, ok: false, error: String(e) });
  }
}

function customSaveAsPreset() {
  const cfg = readCustom();
  if (!cfg.ip || !cfg.subnet) return toast("Fill in IP and subnet first", "err");
  const prefix = subnetToPrefix(cfg.subnet);
  if (prefix === null) return toast("Subnet isn't valid", "err");
  state.presets.push({
    id: "preset-" + Math.random().toString(36).slice(2, 7),
    name: "Custom " + cfg.ip,
    color: "#3B82F6",
    mode: "static",
    ip: cfg.ip,
    prefix,
    gateway: cfg.gateway || undefined,
    dns: cfg.dns,
    dns_clear: cfg.dnsClear || undefined,
  });
  renderPresets();
  toast("Added to presets — remember to Save", "ok");
}

function wireCustom() {
  $("#customSubnet").onchange = () => {
    $("#customSubnetText").classList.toggle("hidden", $("#customSubnet").value !== "custom");
    updatePreview();
  };
  $("#customDns").onchange = () => {
    $("#customDnsInputs").classList.toggle("hidden", $("#customDns").value !== "custom");
    updatePreview();
  };
  // Live preview on every relevant input.
  ["customIp", "customSubnetText", "customGw", "customDns1", "customDns2"].forEach((id) => {
    document.getElementById(id).addEventListener("input", updatePreview);
  });
  $("#customApply").onclick = customApply;
  $("#customSave").onclick = customSaveAsPreset;
  updatePreview();

  // Tray "Custom IP…" opens this window targeted at a specific interface.
  if (listen) {
    listen("custom-apply", (event) => {
      const name = event.payload;
      populateCustomInterfaces();
      const sel = $("#customIface");
      if ([...sel.options].some((o) => o.value === name)) sel.value = name;
      const panel = $("#custom-panel");
      panel.scrollIntoView({ behavior: "smooth", block: "start" });
      panel.classList.remove("flash");
      void panel.offsetWidth; // restart animation
      panel.classList.add("flash");
      $("#customIp").focus();
    });
    // Applies triggered from the tray report their before/after here.
    listen("applied", (event) => {
      showChange(event.payload);
      rescan();
    });
  }
}

// --------------------------------------------------------------------------
// Import / export
// --------------------------------------------------------------------------
function openModal(title, text, onApply) {
  $("#modal-title").textContent = title;
  $("#modal-text").value = text;
  $("#modal").classList.remove("hidden");
  $("#modal-apply").onclick = () => { onApply($("#modal-text").value); };
}
function closeModal() { $("#modal").classList.add("hidden"); }

// --------------------------------------------------------------------------
// Toast
// --------------------------------------------------------------------------
let toastTimer;
function toast(msg, kind) {
  const t = $("#toast");
  t.textContent = msg;
  t.className = "toast " + (kind || "");
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => t.classList.add("hidden"), 2600);
}

// --------------------------------------------------------------------------
// Wire up
// --------------------------------------------------------------------------
window.addEventListener("DOMContentLoaded", () => {
  $("#add").onclick = addPreset;
  $("#save").onclick = savePresets;
  $("#rescan").onclick = rescan;
  $("#hide").onclick = () => invoke("cmd_hide_settings").catch((e) => toast(String(e), "err"));
  wireCustom();
  $("#modal-cancel").onclick = closeModal;
  $("#export").onclick = () =>
    openModal("Export presets (copy this)", JSON.stringify(state, null, 2), () => closeModal());
  $("#import").onclick = () =>
    openModal("Import presets (paste JSON, then Apply)", "", (text) => {
      try {
        const parsed = JSON.parse(text);
        if (!parsed.presets || !Array.isArray(parsed.presets)) throw new Error("missing 'presets' array");
        state = parsed;
        renderPresets();
        closeModal();
        toast("Imported — remember to Save", "ok");
      } catch (e) {
        toast("Invalid JSON: " + e.message, "err");
      }
    });
  const support = $("#support-link");
  if (support && openUrl) {
    support.addEventListener("click", (e) => {
      e.preventDefault();
      openUrl("https://buymeacoffee.com/ekconsult");
    });
  }
  load();
});
