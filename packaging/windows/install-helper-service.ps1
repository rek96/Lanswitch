# Installs the LANSwitch privileged helper as a Windows service running as
# LocalSystem. The NSIS installer calls this elevated after copying files.
#
# Manual dev use:
#   powershell -ExecutionPolicy Bypass -File install-helper-service.ps1 -InstallDir "C:\Program Files\LANSwitch"

param(
  [string]$InstallDir = $PSScriptRoot
)

$ErrorActionPreference = "Stop"

$ServiceName = "LANSwitchHelper"
$DisplayName = "LANSwitch Privileged Helper"

function Find-HelperBinary([string]$Dir) {
  $names = @(
    "lanswitch-helper.exe",
    "lanswitch-helper-x86_64-pc-windows-msvc.exe",
    "lanswitch-helper-aarch64-pc-windows-msvc.exe"
  )
  foreach ($name in $names) {
    $path = Join-Path $Dir $name
    if (Test-Path $path) { return $path }
  }
  return $null
}

$BinaryPath = Find-HelperBinary $InstallDir
if (-not $BinaryPath) {
  throw "Helper binary not found under '$InstallDir'. Build with: cargo build --release -p lanswitch-helper"
}

$existing = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
if ($existing) {
  Write-Host "Service already exists — stopping and removing first."
  Stop-Service $ServiceName -ErrorAction SilentlyContinue
  sc.exe delete $ServiceName | Out-Null
  Start-Sleep -Seconds 1
}

New-Service -Name $ServiceName `
            -DisplayName $DisplayName `
            -BinaryPathName "`"$BinaryPath`"" `
            -StartupType Automatic | Out-Null

Start-Service $ServiceName
Write-Host "Installed and started '$ServiceName' -> $BinaryPath"
