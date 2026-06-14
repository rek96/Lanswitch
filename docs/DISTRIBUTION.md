# Distributing LANSwitch

**Recommendation: ship a signed installer, not a portable zip.**

LANSwitch is a tray app with a **privileged helper** (Windows service / macOS daemon)
that must be installed once. A portable folder leaves users to run PowerShell as admin
manually — fine for you in dev, painful for crews on show sites.

| Format | Good for | Helper install | Autostart / uninstall | SmartScreen / Gatekeeper |
|--------|----------|----------------|---------------------|--------------------------|
| **NSIS `.exe` installer** (Windows) | End users | Automatic (installer hook) | Yes | OK when signed |
| **`.dmg`** (macOS) | End users | Manual SMAppService step¹ | Drag-to-Applications | Requires sign + notarize |
| Portable zip | Dev / IT who know the scripts | Manual | No | Unsigned = warnings |

¹ macOS 13+ expects `SMAppService` registration from code signed with your Developer ID.
See [PRIVILEGED-HELPER.md](./PRIVILEGED-HELPER.md). Windows is fully automated via NSIS.

---

## One-command release build

### Windows

```powershell
.\scripts\build-release.ps1
```

Outputs (unsigned unless you configure signing):

```
target\release\bundle\nsis\LANSwitch_0.1.0_x64-setup.exe
target\release\bundle\msi\LANSwitch_0.1.0_x64_en-US.msi
```

### macOS

```bash
./scripts/build-release.sh
```

Outputs:

```
target/release/bundle/dmg/LANSwitch_0.1.0_aarch64.dmg
target/release/bundle/macos/LANSwitch.app
```

---

## What the Windows installer does

1. Installs the tray app to `Program Files\LANSwitch\`
2. Bundles `lanswitch-helper.exe` (privileged sidecar)
3. **Post-install:** registers `LANSwitchHelper` Windows service (LocalSystem)
4. **Uninstall:** removes the service, then the app files

Scripts live in `packaging/windows/`; NSIS hooks call them from
`$INSTDIR\resources\`.

---

## Code signing checklist

Unsigned builds run locally but scare end users (SmartScreen / Gatekeeper).

### Windows (Authenticode)

1. Buy an **OV or EV code-signing certificate** (SSL certs do not work).
2. Import the `.pfx` into `Cert:\CurrentUser\My` (see
   [Tauri Windows signing](https://v2.tauri.app/distribute/sign/windows/)).
3. Set environment variables **or** edit `src-tauri/tauri.conf.json`:

```json
"windows": {
  "certificateThumbprint": "YOUR_CERT_THUMBPRINT",
  "digestAlgorithm": "sha256",
  "timestampUrl": "http://timestamp.digicert.com"
}
```

4. Re-run `.\scripts\build-release.ps1` — Tauri signs the app and installer via `signtool`.

Sign these artifacts in order if doing it manually:

- `lanswitch-helper.exe`
- `lanswitch.exe`
- `LANSwitch_*-setup.exe` / `.msi`

EV certificates reduce SmartScreen reputation warnings for new publishers.

### macOS (Developer ID + notarization)

1. Enrol in Apple Developer Program.
2. Create **Developer ID Application** certificate.
3. Set before building:

```bash
export APPLE_SIGNING_IDENTITY="Developer ID Application: EK Consult (TEAMID)"
export APPLE_API_ISSUER="..."
export APPLE_API_KEY="..."
export APPLE_API_KEY_PATH="..."
```

4. Build with `./scripts/build-release.sh`.
5. Notarize and staple:

```bash
xcrun notarytool submit "target/release/bundle/dmg/LANSwitch_*.dmg" --wait
xcrun stapler staple "target/release/bundle/macos/LANSwitch.app"
```

The helper daemon **must** be signed with the same team or SMAppService refuses to load it.

---

## Sharing with your team

### Easiest (Windows)

1. Build signed `LANSwitch_*-setup.exe`.
2. Upload to SharePoint / Dropbox / GitHub Releases.
3. User double-clicks → UAC once for install → tray icon appears → presets work.

### Easiest (macOS)

1. Build signed + notarized `.dmg`.
2. User opens DMG, drags to Applications.
3. First launch: approve in **System Settings → Login Items & Extensions** when prompted
   (once SMAppService registration is wired — see PRIVILEGED-HELPER.md).

### GitHub Releases (optional)

Push a tag; the workflow in `.github/workflows/release.yml` builds artifacts when you
add signing secrets (`WINDOWS_CERTIFICATE`, `WINDOWS_CERTIFICATE_PASSWORD`, Apple vars).

---

## Dev-only portable workflow

If you must skip the installer during development:

```powershell
# Elevated PowerShell — start helper manually
cargo build --release -p lanswitch-helper
.\target\release\lanswitch-helper.exe

# Normal shell — tray app
cargo tauri dev
```

Do **not** ship this to non-technical users.

---

## Version bumps

Before each release:

1. Bump `version` in `src-tauri/tauri.conf.json` and `src-tauri/Cargo.toml`.
2. Rebuild — NSIS uses the version to decide upgrade vs reinstall behaviour.
3. Tag: `git tag v0.1.0 && git push origin v0.1.0`

---

## Support link

Creator: **EK Consult** · [Buy me a coffee](https://buymeacoffee.com/ekconsult)
