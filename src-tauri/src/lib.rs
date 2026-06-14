//! LANSwitch desktop app (unprivileged).
//!
//! Responsibilities:
//!   * enumerate interfaces + presets (read-only, in-process),
//!   * build a NIC-first tray menu (interface -> presets),
//!   * send apply requests to the privileged helper,
//!   * host a settings window for editing presets.
//!
//! VERSION NOTE: the tray/menu calls below target Tauri 2.x. If your installed
//! Tauri differs, reconcile the `TrayIconBuilder` / `Menu` / `Submenu` calls
//! with the current docs — the surrounding logic stays the same.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use tauri::{
    menu::{Menu, MenuItem, Submenu},
    tray::TrayIconBuilder,
    App, AppHandle, Emitter, Manager, State, WindowEvent,
};
#[cfg(target_os = "windows")]
use tauri::tray::{MouseButton, MouseButtonState, TrayIconEvent};
use tauri_plugin_opener::OpenerExt;

const BUY_ME_A_COFFEE_URL: &str = "https://buymeacoffee.com/ekconsult";

use lanswitch_core::{
    discover::list_interfaces,
    presets::{load_or_seed, save},
    types::{Interface, Mode, Preset, PresetsFile},
    validate::parse_subnet,
    DEFAULT_PRESETS_JSON,
};

mod helper_client;
use helper_client::apply_via_helper;

/// Shared app state.
struct AppState {
    presets_path: PathBuf,
    /// Maps a generated tray menu-item id -> (interface name, preset id).
    menu_actions: Mutex<HashMap<String, (String, String)>>,
}

// ---------------------------------------------------------------------------
// Commands callable from the settings UI (JS) via `invoke`.
// ---------------------------------------------------------------------------

#[tauri::command]
fn cmd_list_interfaces() -> Vec<Interface> {
    list_interfaces()
}

#[tauri::command]
fn cmd_get_presets(state: State<AppState>) -> Result<PresetsFile, String> {
    load_or_seed(&state.presets_path, DEFAULT_PRESETS_JSON).map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_save_presets(state: State<AppState>, file: PresetsFile) -> Result<(), String> {
    save(&state.presets_path, &file).map_err(|e| e.to_string())
}

/// Apply a preset (by id) to an interface. Looks the preset up fresh so we
/// always send the current saved definition to the helper.
#[tauri::command]
fn cmd_apply(state: State<AppState>, preset_id: String, interface: String) -> Result<(), String> {
    let file =
        load_or_seed(&state.presets_path, DEFAULT_PRESETS_JSON).map_err(|e| e.to_string())?;
    let preset = file
        .presets
        .into_iter()
        .find(|p| p.id == preset_id)
        .ok_or_else(|| format!("preset \"{preset_id}\" not found"))?;
    apply_via_helper(&preset, &interface)
}

/// Apply an ad-hoc, unsaved config (IP / subnet / gateway / DNS) to an
/// interface. `subnet` accepts a prefix ("24", "/24") or a dotted mask
/// ("255.255.255.0"). The helper still re-validates everything.
#[tauri::command]
fn cmd_apply_custom(
    interface: String,
    ip: String,
    subnet: String,
    gateway: Option<String>,
    dns: Vec<String>,
    dns_clear: Option<bool>,
) -> Result<(), String> {
    let prefix = parse_subnet(&subnet).map_err(|e| e.to_string())?;
    let preset = Preset {
        id: "custom".into(),
        name: "Custom".into(),
        color: String::new(),
        mode: Mode::Static,
        ip: Some(ip),
        prefix: Some(prefix),
        gateway: gateway.filter(|s| !s.is_empty()),
        dns: dns.into_iter().filter(|s| !s.is_empty()).collect(),
        dns_clear: dns_clear.unwrap_or(false),
    };
    apply_via_helper(&preset, &interface)
}

/// Rebuild the tray after presets change.
#[tauri::command]
fn cmd_refresh_tray(app: AppHandle) -> Result<(), String> {
    rebuild_tray(&app).map_err(|e| e.to_string())
}

/// Hide the settings window — the tray app keeps running.
#[tauri::command]
fn cmd_hide_settings(app: AppHandle) -> Result<(), String> {
    app.get_webview_window("settings")
        .ok_or_else(|| "settings window not found".into())
        .and_then(|w| w.hide().map_err(|e| e.to_string()))
}

// ---------------------------------------------------------------------------
// Tray (NIC-first).
// ---------------------------------------------------------------------------

fn load_presets(app: &AppHandle) -> Vec<Preset> {
    let state = app.state::<AppState>();
    load_or_seed(&state.presets_path, DEFAULT_PRESETS_JSON)
        .map(|f| f.presets)
        .unwrap_or_default()
}

/// Prefix preset rows with a mode hint — tray menus are text-only on most OSes.
fn preset_menu_label(p: &Preset) -> String {
    let glyph = match p.mode {
        Mode::Static => "●",
        Mode::Dhcp => "↻",
    };
    format!("{glyph}  {}", p.name)
}

/// Build the menu: one submenu per connected interface, listing presets.
fn build_menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let interfaces = list_interfaces();
    let presets = load_presets(app);

    let state = app.state::<AppState>();
    let mut actions = state.menu_actions.lock().unwrap();
    actions.clear();
    let mut counter = 0usize;

    let menu = Menu::new(app)?;

    // Header block — disabled items act as a lightweight "title bar" in native menus.
    let title = MenuItem::with_id(app, "title", "LANSwitch", false, None::<&str>)?;
    let subtitle =
        MenuItem::with_id(app, "subtitle", "Switch LAN presets per interface", false, None::<&str>)?;
    menu.append(&title)?;
    menu.append(&subtitle)?;
    menu.append(&tauri::menu::PredefinedMenuItem::separator(app)?)?;

    if interfaces.is_empty() {
        let none = MenuItem::with_id(app, "noiface", "No network interfaces found", false, None::<&str>)?;
        menu.append(&none)?;
    }

    for iface in &interfaces {
        let label = match &iface.current_ip {
            Some(ip) => format!("{}   ·   {}", iface.name, ip),
            None => format!("{}   ·   (no IP)", iface.name),
        };

        let sub = Submenu::new(app, label, true)?;
        for p in &presets {
            let id = format!("act{counter}");
            counter += 1;
            actions.insert(id.clone(), (iface.name.clone(), p.id.clone()));
            let item = MenuItem::with_id(app, &id, &preset_menu_label(p), true, None::<&str>)?;
            sub.append(&item)?;
        }
        // Per-interface "Custom IP…" opens the settings window targeted at this
        // interface (tray menus can't take text input).
        let custom_id = format!("custom{counter}");
        counter += 1;
        actions.insert(custom_id.clone(), (iface.name.clone(), "__custom__".into()));
        let sub_sep = tauri::menu::PredefinedMenuItem::separator(app)?;
        sub.append(&sub_sep)?;
        let custom_item =
            MenuItem::with_id(app, &custom_id, "✎  Custom IP…", true, None::<&str>)?;
        sub.append(&custom_item)?;
        menu.append(&sub)?;
    }

    let sep_actions = tauri::menu::PredefinedMenuItem::separator(app)?;
    let sep_about = tauri::menu::PredefinedMenuItem::separator(app)?;
    let sep_quit = tauri::menu::PredefinedMenuItem::separator(app)?;
    menu.append(&sep_actions)?;
    let settings = MenuItem::with_id(app, "settings", "⚙  Settings…", true, None::<&str>)?;
    let refresh = MenuItem::with_id(app, "refresh", "↻  Refresh interfaces", true, None::<&str>)?;
    menu.append(&settings)?;
    menu.append(&refresh)?;
    menu.append(&sep_about)?;
    let creator = MenuItem::with_id(app, "creator", "by EK Consult", false, None::<&str>)?;
    let support = MenuItem::with_id(
        app,
        "support",
        "☕  Buy me a coffee",
        true,
        None::<&str>,
    )?;
    menu.append(&creator)?;
    menu.append(&support)?;
    menu.append(&sep_quit)?;
    let quit = MenuItem::with_id(app, "quit", "Quit LANSwitch", true, None::<&str>)?;
    menu.append(&quit)?;

    Ok(menu)
}

fn rebuild_tray(app: &AppHandle) -> tauri::Result<()> {
    if let Some(tray) = app.tray_by_id("main") {
        let menu = build_menu(app)?;
        tray.set_menu(Some(menu))?;
    }
    Ok(())
}

fn open_support_link(app: &AppHandle) {
    let _ = app
        .opener()
        .open_url(BUY_ME_A_COFFEE_URL, None::<&str>);
}

fn on_menu_event(app: &AppHandle, id: &str) {
    match id {
        "settings" => show_settings(app),
        "refresh" => {
            let _ = rebuild_tray(app);
        }
        "support" => open_support_link(app),
        "quit" => app.exit(0),
        other => {
            // An apply action: look up (interface, preset).
            let lookup = {
                let state = app.state::<AppState>();
                let map = state.menu_actions.lock().unwrap();
                map.get(other).cloned()
            };
            if let Some((interface, preset_id)) = lookup {
                // "Custom IP…" — open settings targeted at this interface.
                if preset_id == "__custom__" {
                    show_settings(app);
                    if let Some(win) = app.get_webview_window("settings") {
                        let _ = win.emit("custom-apply", interface);
                    }
                    return;
                }

                let state = app.state::<AppState>();
                let before = current_ip_of(&interface);
                let result = load_or_seed(&state.presets_path, DEFAULT_PRESETS_JSON)
                    .map_err(|e| e.to_string())
                    .and_then(|f| {
                        f.presets
                            .into_iter()
                            .find(|p| p.id == preset_id)
                            .ok_or_else(|| "preset not found".to_string())
                    })
                    .and_then(|preset| apply_via_helper(&preset, &interface));

                match result {
                    Ok(()) => {
                        let after = current_ip_of(&interface);
                        notify(
                            app,
                            &format!("LANSwitch · {interface}"),
                            &format!("{before}  →  {after}"),
                        );
                        emit_change(app, &interface, &before, &after, true, None);
                    }
                    Err(e) => {
                        notify(app, "LANSwitch — failed", &e);
                        emit_change(app, &interface, &before, &before, false, Some(&e));
                    }
                }
                let _ = rebuild_tray(app); // refresh shown IPs
            }
        }
    }
}

/// Current IPv4 of an interface by friendly name, or "no IP".
fn current_ip_of(interface: &str) -> String {
    list_interfaces()
        .into_iter()
        .find(|i| i.name == interface)
        .and_then(|i| i.current_ip)
        .unwrap_or_else(|| "no IP".into())
}

fn notify(app: &AppHandle, title: &str, body: &str) {
    use tauri_plugin_notification::NotificationExt;
    let _ = app.notification().builder().title(title).body(body).show();
    eprintln!("[notify] {title}: {body}");
}

/// Tell an open settings window what changed, so it can show a banner.
fn emit_change(
    app: &AppHandle,
    interface: &str,
    before: &str,
    after: &str,
    ok: bool,
    error: Option<&str>,
) {
    if let Some(win) = app.get_webview_window("settings") {
        let _ = win.emit(
            "applied",
            serde_json::json!({
                "interface": interface,
                "before": before,
                "after": after,
                "ok": ok,
                "error": error,
            }),
        );
    }
}

fn show_settings(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("settings") {
        let _ = win.show();
        let _ = win.set_focus();
    }
}

/// Closing the settings window hides it — the tray app keeps running.
fn hide_settings_window(window: &tauri::Window) {
    let _ = window.hide();
}

fn tray_icon() -> tauri::image::Image<'static> {
    tauri::include_image!("icons/32x32.png")
}

// ---------------------------------------------------------------------------
// Setup / entry point.
// ---------------------------------------------------------------------------

fn resolve_presets_path(app: &App) -> PathBuf {
    let dir = app
        .path()
        .app_config_dir()
        .unwrap_or_else(|_| PathBuf::from("."));
    dir.join("presets.json")
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .setup(|app| {
            let presets_path = resolve_presets_path(app);
            // Seed presets on first run.
            let _ = load_or_seed(&presets_path, DEFAULT_PRESETS_JSON);

            app.manage(AppState {
                presets_path,
                menu_actions: Mutex::new(HashMap::new()),
            });

            let handle = app.handle().clone();
            let menu = build_menu(&handle)?;

            let tray = TrayIconBuilder::with_id("main")
                .icon(tray_icon())
                .tooltip("LANSwitch — by EK Consult")
                .menu(&menu)
                .on_menu_event(|app, event| {
                    on_menu_event(app, event.id().as_ref());
                });

            #[cfg(target_os = "windows")]
            let tray = tray.on_tray_icon_event(|tray, event| {
                if let TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } = event
                {
                    show_settings(tray.app_handle());
                }
            });

            tray.build(app)?;

            // Enable start-at-login (user chose this default).
            use tauri_plugin_autostart::ManagerExt;
            let _ = app.autolaunch().enable();

            // Hide the settings window until explicitly opened.
            if let Some(win) = app.get_webview_window("settings") {
                let _ = win.hide();
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                hide_settings_window(window);
            }
        })
        .invoke_handler(tauri::generate_handler![
            cmd_list_interfaces,
            cmd_get_presets,
            cmd_save_presets,
            cmd_apply,
            cmd_apply_custom,
            cmd_refresh_tray,
            cmd_hide_settings,
        ])
        .build(tauri::generate_context!())
        .expect("error while building LANSwitch")
        // Keep the event loop alive even when all windows are hidden.
        // The only exit path is the "Quit" tray item, which calls app.exit(0).
        .run(|_app, event| {
            if let tauri::RunEvent::ExitRequested { api, .. } = event {
                api.prevent_exit();
            }
        });
}
