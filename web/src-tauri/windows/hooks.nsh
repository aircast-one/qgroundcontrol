!macro NSIS_HOOK_POSTINSTALL
  nsExec::Exec 'netsh advfirewall firewall delete rule name="Aircast video and telemetry" program="$INSTDIR\aircast-desktop.exe"'
  nsExec::Exec 'netsh advfirewall firewall add rule name="Aircast video and telemetry" dir=in action=allow program="$INSTDIR\aircast-desktop.exe" enable=yes'
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  nsExec::Exec 'netsh advfirewall firewall delete rule name="Aircast video and telemetry" program="$INSTDIR\aircast-desktop.exe"'
!macroend
