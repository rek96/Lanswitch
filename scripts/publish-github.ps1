# Publish to GitHub Releases
#
# Builds Windows installers, pushes the repo, and uploads release assets.
# Requires: gh auth login, git, Rust, cargo-tauri
#
# Usage:
#   .\scripts\publish-github.ps1
#   .\scripts\publish-github.ps1 -Repo rek96/Lanswitch -SkipBuild

param(
  [string]$Repo = "rek96/Lanswitch",
  [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"
$Root = Split-Path $PSScriptRoot -Parent
Set-Location $Root

$Version = (Get-Content "src-tauri/tauri.conf.json" -Raw | Select-String '"version"\s*:\s*"([^"]+)"').Matches[0].Groups[1].Value
$Tag = "v$Version"

$gh = Get-Command gh -ErrorAction SilentlyContinue
if (-not $gh) {
  $ghPath = "C:\Program Files\GitHub CLI\gh.exe"
  if (Test-Path $ghPath) { $gh = $ghPath } else { throw "GitHub CLI (gh) not found. Install: winget install GitHub.cli" }
}

& $gh auth status 2>&1 | Out-Null
if ($LASTEXITCODE -ne 0) {
  throw "Not logged into GitHub. Run: gh auth login"
}

if (-not $SkipBuild) {
  & "$PSScriptRoot\build-release.ps1"
}

$setup = Get-ChildItem "target\release\bundle\nsis\LANSwitch_*-setup.exe" | Select-Object -First 1
$msi = Get-ChildItem "target\release\bundle\msi\LANSwitch_*_x64_en-US.msi" | Select-Object -First 1
if (-not $setup) { throw "NSIS installer not found. Run build-release.ps1 first." }

# Ensure remote exists and code is pushed
$remoteUrl = "https://github.com/$Repo.git"
$hasOrigin = $false
try {
  git remote get-url origin 2>$null | Out-Null
  if ($LASTEXITCODE -eq 0) { $hasOrigin = $true }
} catch {}

if (-not $hasOrigin) {
  git remote add origin $remoteUrl
}

git branch -M main
$pushOk = $false
git push -u origin main 2>$null
if ($LASTEXITCODE -eq 0) { $pushOk = $true }

if (-not $pushOk) {
  & $gh repo create $Repo --public --source=. --remote=origin --push --description "Quick LAN preset switching from the system tray — by EK Consult"
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
} else {
  git push origin main
}

# Create or update release
$releaseExists = & $gh release view $Tag 2>$null
if ($LASTEXITCODE -eq 0) {
  Write-Host "Release $Tag exists — uploading assets..."
  & $gh release upload $Tag $setup.FullName $msi.FullName --clobber
} else {
  & $gh release create $Tag `
    $setup.FullName `
    $msi.FullName `
    --title "LANSwitch $Version" `
    --notes "## Install (Windows)`n`n1. Download **$($setup.Name)**`n2. Run the installer (one UAC prompt)`n3. Use the system tray icon to switch LAN presets`n`nCreated by [EK Consult](https://buymeacoffee.com/ekconsult).`n`n> SmartScreen may warn until the installer is code-signed. See docs/DISTRIBUTION.md."
}

Write-Host ""
Write-Host "Published: https://github.com/$Repo/releases/tag/$Tag"
Write-Host "Latest:    https://github.com/$Repo/releases/latest"
