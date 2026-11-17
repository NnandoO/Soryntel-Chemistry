!include "MUI2.nsh"

!define APP_NAME "Soryntel Chemistry"
!define APP_VERSION "0.1.0"
!define PUBLISHER "Soryntel Chemistry Team"
!define EXE_NAME "soryntel-desktop.exe"

Name "${APP_NAME} ${APP_VERSION}"
OutFile "..\..\dist\Soryntel-Chemistry-${APP_VERSION}-win64-setup.exe"
InstallDir "$PROGRAMFILES64\Soryntel Chemistry"
InstallDirRegKey HKLM "Software\SoryntelChemistry" "InstallDir"
RequestExecutionLevel admin
Unicode True
SetCompressor /SOLID lzma

!define MUI_ICON "..\soryntel-icon.ico"
!define MUI_UNICON "..\soryntel-icon.ico"
!define MUI_ABORTWARNING

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!define MUI_FINISHPAGE_RUN "$INSTDIR\${EXE_NAME}"
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "Spanish"
!insertmacro MUI_LANGUAGE "English"

Section "Soryntel Chemistry" SecMain
    SetOutPath "$INSTDIR"
    File "..\..\target\x86_64-pc-windows-gnu\release\${EXE_NAME}"
    File "..\soryntel-icon.ico"
    CreateDirectory "$INSTDIR\Databases"

    WriteUninstaller "$INSTDIR\uninstall.exe"

    ; Start Menu shortcuts
    CreateDirectory "$SMPROGRAMS\${APP_NAME}"
    CreateShortcut "$SMPROGRAMS\${APP_NAME}\${APP_NAME}.lnk" "$INSTDIR\${EXE_NAME}" "" "$INSTDIR\soryntel-icon.ico" 0
    CreateShortcut "$SMPROGRAMS\${APP_NAME}\Desinstalar ${APP_NAME}.lnk" "$INSTDIR\uninstall.exe"

    ; Desktop shortcut
    CreateShortcut "$DESKTOP\${APP_NAME}.lnk" "$INSTDIR\${EXE_NAME}" "" "$INSTDIR\soryntel-icon.ico" 0

    ; Add/Remove Programs metadata
    WriteRegStr HKLM "Software\SoryntelChemistry" "InstallDir" "$INSTDIR"
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\SoryntelChemistry" "DisplayName" "${APP_NAME}"
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\SoryntelChemistry" "DisplayVersion" "${APP_VERSION}"
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\SoryntelChemistry" "Publisher" "${PUBLISHER}"
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\SoryntelChemistry" "UninstallString" '"$INSTDIR\uninstall.exe"'
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\SoryntelChemistry" "DisplayIcon" "$INSTDIR\soryntel-icon.ico"
SectionEnd

Section "Uninstall"
    Delete "$DESKTOP\${APP_NAME}.lnk"
    RMDir /r "$SMPROGRAMS\${APP_NAME}"
    Delete "$INSTDIR\${EXE_NAME}"
    Delete "$INSTDIR\soryntel-icon.ico"
    Delete "$INSTDIR\uninstall.exe"
    RMDir "$INSTDIR\Databases"
    RMDir "$INSTDIR"
    DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\SoryntelChemistry"
    DeleteRegKey HKLM "Software\SoryntelChemistry"
SectionEnd
