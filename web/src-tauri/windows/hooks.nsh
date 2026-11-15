!define NATIVE_UNINSTALL_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\AircastQGCNative"

!macro NSIS_HOOK_POSTINSTALL
  nsExec::ExecToLog 'netsh advfirewall firewall delete rule name="Aircast video and telemetry" program="$INSTDIR\aircast-desktop.exe"'
  Pop $0
  nsExec::ExecToLog 'netsh advfirewall firewall add rule name="Aircast video and telemetry" dir=in action=allow program="$INSTDIR\aircast-desktop.exe" enable=yes'
  Pop $0
  ${If} $0 != 0
    DetailPrint "Windows did not take the firewall rule for Aircast (code $0). Windows will ask to allow Aircast the first time it receives video or telemetry."
  ${EndIf}
  ReadRegStr $1 HKCU "${NATIVE_UNINSTALL_KEY}" "QuietUninstallString"
  ${If} $1 != ""
    DetailPrint "Removing the earlier Aircast QGC (native) app. Its settings, maps and files stay."
    ExecWait $1 $2
    ${If} $2 != 0
      DetailPrint "The earlier Aircast QGC (native) app could not be removed (code $2). Remove it from Settings > Apps."
    ${EndIf}
  ${EndIf}
  ${If} ${Silent}
    Exec '"$INSTDIR\aircast-desktop.exe"'
  ${EndIf}
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  nsExec::ExecToLog 'netsh advfirewall firewall delete rule name="Aircast video and telemetry" program="$INSTDIR\aircast-desktop.exe"'
  Pop $0
!macroend
