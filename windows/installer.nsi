Unicode true

!include "MUI2.nsh"
!include "LogicLib.nsh"
!include "FileFunc.nsh"
!include "x64.nsh"

!define PRODUCT "AircastQGCNative"
!define DISPLAY "Aircast QGC (native)"
!define SHORTCUT "Aircast QGC"
!define EXE "AircastQGC.exe"
!define ICON "AircastQGC.ico"
!define UNINSTALLER "Uninstall.exe"
!define UNINSTALL_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCT}"
!define DEEP_LINK_KEY "Software\Classes\aircast-qgc"
!define BRANDING "${__FILEDIR__}\..\deploy\windows"

!if "${ARCH}" == "arm64"
    !define MACHINE 43620
    !define OTHER_ARCH "x64"
!else if "${ARCH}" == "x64"
    !define MACHINE 34404
    !define OTHER_ARCH "arm64"
!else
    !error "ARCH must be arm64 or x64"
!endif

Name "${DISPLAY}"
Caption "${DISPLAY} ${VERSION} (${ARCH}) Setup"
BrandingText "${DISPLAY} ${VERSION} ${ARCH}"
OutFile "${OUTFILE}"
InstallDir "$LOCALAPPDATA\Programs\${PRODUCT}"
InstallDirRegKey HKCU "${UNINSTALL_KEY}" "InstallLocation"
RequestExecutionLevel user
SetCompressor /SOLID /FINAL lzma

!define MUI_ICON "${BRANDING}\WindowsQGC.ico"
!define MUI_UNICON "${BRANDING}\WindowsQGC.ico"
!define MUI_HEADERIMAGE
!define MUI_HEADERIMAGE_BITMAP "${BRANDING}\installheader.bmp"
!define MUI_COMPONENTSPAGE_NODESC

!insertmacro MUI_PAGE_COMPONENTS
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"

!macro CLOSE_RUNNING UN
Function ${UN}Locked
    StrCpy $0 0
    ${If} ${FileExists} "$INSTDIR\${EXE}"
        ClearErrors
        FileOpen $1 "$INSTDIR\${EXE}" a
        ${If} ${Errors}
            StrCpy $0 1
        ${Else}
            FileClose $1
        ${EndIf}
    ${EndIf}
FunctionEnd

Function ${UN}CloseRunning
    Call ${UN}Locked
    ${If} $0 == 0
        Return
    ${EndIf}
    ${IfNot} ${Cmd} `MessageBox MB_OKCANCEL|MB_ICONEXCLAMATION "${DISPLAY} is running from $INSTDIR.$\r$\n$\r$\nSetup has to close it to continue. Land any vehicle it is flying first.$\r$\n$\r$\nClose ${DISPLAY} now?" /SD IDCANCEL IDOK`
        Abort "${DISPLAY} is still running."
    ${EndIf}
    System::Call 'kernel32::SetEnvironmentVariable(t "AIRCAST_QGC_EXE", t "$INSTDIR\${EXE}")i'
    nsExec::Exec `powershell.exe -NoProfile -NonInteractive -Command "Get-Process AircastQGC -ErrorAction SilentlyContinue | Where-Object Path -eq $$env:AIRCAST_QGC_EXE | ForEach-Object { $$_.Kill(); $$_.WaitForExit(10000) }"`
    Pop $0
    Call ${UN}Locked
    ${If} $0 != 0
        MessageBox MB_OK|MB_ICONSTOP "Setup could not close ${DISPLAY}. Close it yourself and run setup again." /SD IDOK
        Abort "${DISPLAY} is still running."
    ${EndIf}
FunctionEnd
!macroend
!insertmacro CLOSE_RUNNING ""
!insertmacro CLOSE_RUNNING "un."

Function .onInit
    ${IfNot} ${IsNativeMachineArchitecture} ${MACHINE}
        MessageBox MB_OK|MB_ICONSTOP "This is the ${ARCH} installer of ${DISPLAY}, and this PC does not run ${ARCH} Windows.$\r$\n$\r$\nDownload the windows-${OTHER_ARCH} installer instead." /SD IDOK
        Abort
    ${EndIf}
FunctionEnd

Section "${DISPLAY}" SecApp
    SectionIn RO
    Call CloseRunning
    RMDir /r "$INSTDIR"
    SetOutPath "$INSTDIR"
    File /r "${PAYLOAD}\*.*"
    File "/oname=${ICON}" "${BRANDING}\WindowsQGC.ico"
    WriteUninstaller "$INSTDIR\${UNINSTALLER}"

    CreateShortCut "$SMPROGRAMS\${SHORTCUT}.lnk" "$INSTDIR\${EXE}" "" "$INSTDIR\${ICON}"
    Delete "$DESKTOP\${SHORTCUT}.lnk"

    ${GetSize} "$INSTDIR" "/S=0K" $0 $1 $2
    WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayName" "${DISPLAY}"
    WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayVersion" "${VERSION}"
    WriteRegStr HKCU "${UNINSTALL_KEY}" "Publisher" "Aircast"
    WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayIcon" "$INSTDIR\${ICON}"
    WriteRegStr HKCU "${UNINSTALL_KEY}" "InstallLocation" "$INSTDIR"
    WriteRegStr HKCU "${UNINSTALL_KEY}" "UninstallString" '"$INSTDIR\${UNINSTALLER}"'
    WriteRegStr HKCU "${UNINSTALL_KEY}" "QuietUninstallString" '"$INSTDIR\${UNINSTALLER}" /S'
    WriteRegDWORD HKCU "${UNINSTALL_KEY}" "EstimatedSize" $0
    WriteRegDWORD HKCU "${UNINSTALL_KEY}" "NoModify" 1
    WriteRegDWORD HKCU "${UNINSTALL_KEY}" "NoRepair" 1
SectionEnd

Section "Desktop shortcut" SecDesktop
    CreateShortCut "$DESKTOP\${SHORTCUT}.lnk" "$INSTDIR\${EXE}" "" "$INSTDIR\${ICON}"
SectionEnd

Section "Uninstall"
    Call un.CloseRunning
    Delete "$SMPROGRAMS\${SHORTCUT}.lnk"
    Delete "$DESKTOP\${SHORTCUT}.lnk"
    RMDir /r "$INSTDIR"
    ReadRegStr $0 HKCU "${DEEP_LINK_KEY}\shell\open\command" ""
    ${If} $0 == `"$INSTDIR\${EXE}" "%1"`
        DeleteRegKey HKCU "${DEEP_LINK_KEY}"
    ${EndIf}
    DeleteRegKey HKCU "${UNINSTALL_KEY}"
SectionEnd
