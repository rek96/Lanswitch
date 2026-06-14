# Copies the release helper binary into src-tauri/binaries/ with the target-triple
# suffix required by Tauri externalBin bundling.
param(
  [ValidateSet("release", "debug")]
  [string]$Profile = "release"
)

$ErrorActionPreference = "Stop"
$Root = Split-Path $PSScriptRoot -Parent
Set-Location $Root

$tripleLine = (rustc -vV | Select-String "^host:").Line
if (-not $tripleLine) { throw "Could not detect Rust host triple." }
$triple = $tripleLine -replace "^host:\s*", ""

Write-Host "Building lanswitch-helper ($Profile) for $triple..."
cargo build -p lanswitch-helper --profile $Profile
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$src = Join-Path $Root "target\$Profile\lanswitch-helper.exe"
if (-not (Test-Path $src)) {
  $src = Join-Path $Root "target\$Profile\lanswitch-helper"
}
if (-not (Test-Path $src)) {
  throw "Helper binary not found after build."
}

$destDir = Join-Path $Root "src-tauri\binaries"
New-Item -ItemType Directory -Force -Path $destDir | Out-Null

$dest = Join-Path $destDir "lanswitch-helper-$triple.exe"
if ($src -notmatch "\.exe$") {
  $dest = Join-Path $destDir "lanswitch-helper-$triple"
}

Copy-Item -Force $src $dest
Write-Host "Prepared $dest"
