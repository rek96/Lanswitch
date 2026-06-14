# Build signed-ready Windows installers for LANSwitch.
# Configure certificateThumbprint in tauri.conf.json (or env) before shipping.
param(
  [switch]$SkipHelper
)

$ErrorActionPreference = "Stop"
$Root = Split-Path $PSScriptRoot -Parent
Set-Location $Root

if (-not $SkipHelper) {
  & "$PSScriptRoot\prepare-binaries.ps1" -Profile release
}

Write-Host "Building LANSwitch installers..."
cargo tauri build
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$bundle = Join-Path $Root "target\release\bundle"
Write-Host ""
Write-Host "Done. Installers:"
Get-ChildItem -Path $bundle -Recurse -Include "*setup.exe", "*.msi" -ErrorAction SilentlyContinue |
  ForEach-Object { Write-Host "  $($_.FullName)" }

Write-Host ""
Write-Host "Share the NSIS setup.exe with end users."
Write-Host "See docs/DISTRIBUTION.md for code signing."
