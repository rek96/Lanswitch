; NSIS hooks for the LANSwitch Windows installer.
; Registers / removes the privileged helper Windows service.
; Requires installMode "perMachine" so hooks run elevated.

!macro NSIS_HOOK_PREINSTALL
  DetailPrint "Stopping LANSwitch helper service (if present)..."
  nsExec::ExecToLog 'powershell -NoProfile -ExecutionPolicy Bypass -File "$INSTDIR\resources\uninstall-helper-service.ps1"'
  Sleep 1000
!macroend

!macro NSIS_HOOK_POSTINSTALL
  DetailPrint "Installing LANSwitch privileged helper service..."
  nsExec::ExecToLog 'powershell -NoProfile -ExecutionPolicy Bypass -File "$INSTDIR\resources\install-helper-service.ps1" -InstallDir "$INSTDIR"'
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  DetailPrint "Removing LANSwitch privileged helper service..."
  nsExec::ExecToLog 'powershell -NoProfile -ExecutionPolicy Bypass -File "$INSTDIR\resources\uninstall-helper-service.ps1"'
!macroend
