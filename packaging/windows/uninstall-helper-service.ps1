# Removes the LANSwitch privileged helper service. Run elevated.
$ServiceName = "LANSwitchHelper"
Stop-Service $ServiceName -ErrorAction SilentlyContinue
sc.exe delete $ServiceName | Out-Null
Write-Host "Removed '$ServiceName'."
