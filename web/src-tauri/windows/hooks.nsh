!macro NSIS_HOOK_POSTINSTALL
  nsExec::ExecToLog 'netsh advfirewall firewall delete rule name="Aircast video and telemetry" program="$INSTDIR\aircast-desktop.exe"'
  Pop $0
  nsExec::ExecToLog 'netsh advfirewall firewall add rule name="Aircast video and telemetry" dir=in action=allow program="$INSTDIR\aircast-desktop.exe" enable=yes'
  Pop $0
  ${If} $0 != 0
    DetailPrint "Windows did not take the firewall rule for Aircast (code $0). Windows will ask to allow Aircast the first time it receives video or telemetry."
  ${EndIf}
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  nsExec::ExecToLog 'netsh advfirewall firewall delete rule name="Aircast video and telemetry" program="$INSTDIR\aircast-desktop.exe"'
  Pop $0
!macroend
