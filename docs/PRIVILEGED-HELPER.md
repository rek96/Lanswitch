# The privileged helper: install, sign, notarize

The helper is the only part that runs with elevated rights. It applies IP
changes and nothing else. This doc covers getting it installed and trusted on
each OS — the part that bites teams shipping to non-technical users.

---

## macOS (13+) — SMAppService daemon

On macOS 13+ the modern, Apple-blessed way to install a privileged background
service is **`SMAppService.daemon(plistName:)`**. It replaces the old
`SMJobBless` dance.

### Bundle layout

The daemon plist and binary ship *inside* your app bundle:

```
LANSwitch.app/
├── Contents/MacOS/lanswitch            # the tray app
├── Contents/MacOS/lanswitch-helper     # the privileged binary
└── Contents/Library/LaunchDaemons/
    └── com.lanswitch.helper.plist      # from packaging/macos/
```

Use the provided `packaging/macos/com.lanswitch.helper.plist`. Its `Label` must
match the id you register and the `BundleProgram` must point at the helper
binary inside the bundle.

### Registering (one-time, asks for admin)

From the app (Swift side of a small plugin, or a build step), register the
daemon:

```swift
import ServiceManagement

let service = SMAppService.daemon(plistName: "com.lanswitch.helper.plist")
do {
    try service.register()        // prompts the user to approve in System Settings
} catch {
    // surface "Login Items & Extensions" so the user can enable it
}
```

The user approves once under **System Settings → General → Login Items &
Extensions**. After that the daemon runs as root at boot, opens the local
socket, and the tray app connects to it.

### Signing & notarization (required)

SMAppService will refuse to load an unsigned or improperly signed daemon.

1. Sign the helper and the app with your **Developer ID Application** cert.
2. The daemon's plist `Label` and the app's bundle id should share a team
   prefix; keep entitlements minimal.
3. Notarize the whole `.app` (`xcrun notarytool submit … --wait`) and staple
   (`xcrun stapler staple LANSwitch.app`).

Without notarization, Gatekeeper blocks the helper on other people's Macs.

### Removing

`try service.unregister()` from the app, or the user toggles it off in Login
Items & Extensions.

---

## Windows 10/11 — LocalSystem service

The helper runs as a Windows **service** under the LocalSystem account, which
has the rights to change IP configuration. Your installer registers it once;
afterwards there's no per-action UAC prompt.

### Install (one-time, elevated)

Your installer (MSI/NSIS) should run, elevated:

```powershell
packaging\windows\install-helper-service.ps1
```

That script creates and starts the `LANSwitchHelper` service pointing at
`lanswitch-helper.exe`. Edit `$BinaryPath` in the script to wherever your
installer drops the binary (e.g. `C:\Program Files\LANSwitch\`).

### Signing (recommended)

Sign `lanswitch-helper.exe`, the tray `lanswitch.exe`, and the installer with an
**Authenticode** code-signing certificate (ideally EV to skip SmartScreen
reputation warnings). Unsigned installers that register a LocalSystem service
will scare users and may be blocked by policy.

### Removing

```powershell
packaging\windows\uninstall-helper-service.ps1
```

---

## The IPC contract (both platforms)

The app connects to a namespaced local socket named `lanswitch-helper.sock`
(`interprocess` crate → Unix domain socket on macOS, named pipe on Windows). One
request, one response, newline-delimited JSON.

Request (`HelperRequest`):

```json
{ "preset": { "id": "coda", "name": "Coda Audio", "mode": "static",
              "ip": "192.168.0.245", "prefix": 24 },
  "interface": "USB 10/100/1000 LAN" }
```

Response (`HelperResponse`):

```json
{ "ok": true }
{ "ok": false, "error": "interface \"Wi-Fi\" is not currently present" }
```

The helper re-validates the request against the live interface list before
doing anything, so a malformed or forged request can't make it run an arbitrary
command.

### Hardening (optional)

For a stricter threat model, add a random token generated at install time,
stored somewhere only your app and helper can read, and include it in
`HelperRequest`. Reject mismatches in `helper/src/main.rs::process`. On Windows
you can additionally restrict the named pipe's ACL; on macOS, tighten the socket
file permissions or move to XPC with a code-signing requirement check.
