# LANSwitch

**Quick LAN preset switching from the system tray — for AV/production crews.**

Created by **[EK Consult](https://buymeacoffee.com/ekconsult)**.

Pick an interface by its **friendly name** (Wi-Fi, USB Ethernet dongle, built-in
port), pick a preset (Coda Audio, dLive, Lighting /8, Art-Net, DHCP…), done.

## Download & install

**Latest release:** [github.com/rek96/Lanswitch/releases/latest](https://github.com/rek96/Lanswitch/releases/latest)

| Platform | Download | Notes |
|----------|----------|-------|
| **Windows 10/11** | [`LANSwitch_*-setup.exe`](https://github.com/rek96/Lanswitch/releases/latest) | Recommended — installs app + privileged helper service |
| Windows (MSI) | `LANSwitch_*_x64_en-US.msi` | Same install, enterprise-friendly |
| macOS 13+ | [`.dmg` on Releases](https://github.com/rek96/Lanswitch/releases/latest) | Apple Silicon (arm64); helper requires approval — see [docs/PRIVILEGED-HELPER.md](docs/PRIVILEGED-HELPER.md) |

### Windows (3 steps)

1. Download **`LANSwitch_*-setup.exe`** from [Releases](https://github.com/rek96/Lanswitch/releases/latest).
2. Run the installer (one UAC prompt — registers the background helper service).
3. Find **LANSwitch** in the system tray → right-click an interface → pick a preset.

Closing the settings window **hides** the app to the tray. Use **Quit LANSwitch** in the tray menu to exit.

> Windows may show a SmartScreen warning until the installer is signed with a
> code-signing certificate. See [docs/DISTRIBUTION.md](docs/DISTRIBUTION.md).

---

## Features

- **Framework:** Tauri 2 (Rust + a small HTML/JS settings window)
- **Platforms:** macOS 13+ and Windows 10/11
- **Elevation:** a privileged background helper does the actual IP changes; the
  tray app runs unprivileged and talks to it over a local socket. The user
  authorizes the helper **once**, not on every change.
- **Tray layout:** NIC-first — choose an interface, then a preset (or **Custom
  IP…** for a one-off address).
- **Presets:** stored as JSON, seeded on first run, editable in the settings
  window (no JSON required) with import/export for sharing between machines.
- **Quick custom apply:** type a one-off IP / subnet / gateway and apply it now,
  with selectable DNS (Cloudflare / Google / Quad9 / custom / **clear to
  automatic** / leave unchanged) and a **live CIDR preview** that validates your
  input before you apply (e.g. `192.168.0.245/24 · mask 255.255.255.0`). Subnet
  accepts a prefix (`24`, `/24`) or a dotted mask (`255.255.255.0`). Optionally
  save it as a preset.

## Layout

```
lanswitch/
├── Cargo.toml                 # workspace (core + helper + src-tauri)
├── core/                      # shared logic (the heart)
│   └── src/
│       ├── types.rs           # Preset, Interface, Helper{Request,Response}
│       ├── validate.rs        # strict validation + prefix→mask
│       ├── commands.rs        # builds argv for networksetup / netsh (no shell)
│       ├── discover.rs        # read-only interface listing (no elevation)
│       └── presets.rs         # load/seed/save presets.json
├── helper/                    # PRIVILEGED binary (root / LocalSystem)
│   └── src/main.rs            # IPC listener → re-validate → execute
├── src-tauri/                 # the unprivileged Tauri app
│   └── src/
│       ├── lib.rs             # commands + NIC-first tray + autostart
│       ├── main.rs
│       └── helper_client.rs   # sends apply requests to the helper
├── ui/                        # static settings window
│   ├── settings.html / .js / styles.css
│   └── presets.default.json   # your AV networks, .245 host
├── packaging/
│   ├── macos/com.lanswitch.helper.plist
│   └── windows/install-helper-service.ps1 (+ uninstall)
└── docs/PRIVILEGED-HELPER.md  # install + signing + notarization
```

## How it works

1. **Discovery (no admin):** the app lists interfaces by friendly name and reads
   their current IP directly — `networksetup` on macOS, `Get-NetAdapter` /
   `Get-NetIPAddress` on Windows.
2. **Apply (admin):** the app sends the chosen preset + interface (or an ad-hoc
   custom config) to the helper. The helper **re-enumerates live interfaces,
   re-validates everything**, builds an explicit sequence of argv commands
   (never a shell string) — address first, then DNS if provided — and runs them,
   stopping on the first failure. Tools used: `networksetup -setmanual /
   -setdnsservers` on macOS, `netsh interface ip set address / set dns` on
   Windows.
3. **Presets:** plain JSON in the OS app-config dir, seeded from
   `ui/presets.default.json` on first run.

Default presets (host `.245`, no gateway — correct for flat AV LANs):

| Preset            | Address          | Prefix |
|-------------------|------------------|--------|
| Coda Audio        | 192.168.0.245    | /24    |
| dLive             | 192.168.1.245    | /24    |
| Video             | 192.168.10.245   | /24    |
| Lighting (/24)    | 10.0.0.245       | /24    |
| Lighting (/8)     | 10.0.0.245       | /8     |
| Art-Net (/24)     | 2.0.0.245        | /24    |
| Art-Net (/8)      | 2.0.0.245        | /8     |
| Automatic (DHCP)  | —                | —      |

## Build & run (dev)

Prereqs: Rust toolchain, Node (for the Tauri CLI), and the Tauri 2 system deps
for your OS.

```bash
# from the repo root
cargo build --release -p lanswitch-helper   # the privileged binary
cargo install tauri-cli --version "^2"       # if you don't have it
cargo tauri dev                              # runs the tray app
```

The helper must be installed/running before "apply" works — see
`docs/PRIVILEGED-HELPER.md`. During development you can also just run the helper
manually in an elevated terminal:

```bash
# macOS (elevated)
sudo ./target/release/lanswitch-helper
# Windows (elevated PowerShell)
.\target\release\lanswitch-helper.exe
```

## Before you ship

See **[docs/DISTRIBUTION.md](docs/DISTRIBUTION.md)** for the full release guide.

**Use a signed installer, not a portable zip.** The privileged helper must be
registered once (automated on Windows via NSIS; manual SMAppService approval on
macOS).

- **Windows:** run `.\scripts\build-release.ps1` → share `LANSwitch_*-setup.exe`.
  Sign with an Authenticode cert (`certificateThumbprint` in `tauri.conf.json`).
- **macOS:** run `./scripts/build-release.sh` → notarize the `.dmg` before sharing.
- **Sign & notarize.** On macOS the helper daemon must be signed and notarized
  or it won't load via SMAppService. On Windows, sign the installer/service to
  avoid SmartScreen. Details in `docs/PRIVILEGED-HELPER.md`.
- **Confirm versions.** Pin the `2`-versioned crates and reconcile the tray API.
- **Icons.** App and tray icons live under `src-tauri/icons/`; regenerate from `app-icon.png` with `cargo tauri icon ./app-icon.png -o src-tauri/icons`.

## Security notes

- Only *applying* a change is privileged; discovery is read-only and unprivileged.
- The helper never interpolates input into a shell. It validates the IP (parsed),
  the prefix (0–32), and the interface (must be in the live list) before acting.
- The local socket is reachable by other local processes. For a single-user AV
  laptop that's an acceptable threat model; if you need more, add a shared-secret
  token to `HelperRequest` and check it in `helper/src/main.rs`. (Stub-friendly.)
