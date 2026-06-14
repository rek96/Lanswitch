# Bundled helper binaries (generated at build time)

Tauri `externalBin` expects platform-specific filenames here, for example:

- `lanswitch-helper-x86_64-pc-windows-msvc.exe`
- `lanswitch-helper-aarch64-apple-darwin`
- `lanswitch-helper-x86_64-apple-darwin`

Run `scripts/prepare-binaries.ps1` (Windows) or `scripts/prepare-binaries.sh` (macOS)
before `cargo tauri build`. Release scripts do this automatically.

These files are gitignored — do not commit built binaries.
