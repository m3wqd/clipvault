Unicode true
!include "MUI2.nsh"
!include "LogicLib.nsh"
!include "x64.nsh"
!ifndef APP_EXE
 !error "Pass /DAPP_EXE=/absolute/path/clipvault.exe"
!endif
!ifndef SETUP_OUT
 !error "Pass /DSETUP_OUT=/absolute/path/ClipVault-0.2.0-x64-setup.exe"
!endif
!ifndef WEBVIEW_BOOTSTRAPPER
 !error "Pass /DWEBVIEW_BOOTSTRAPPER=/absolute/path/MicrosoftEdgeWebview2Setup.exe"
!endif
Name "ClipVault"
OutFile "${SETUP_OUT}"
InstallDir "$LOCALAPPDATA\Programs\ClipVault"
InstallDirRegKey HKCU "Software\ClipVault" "InstallDir"
RequestExecutionLevel user
SetCompressor /SOLID lzma
VIProductVersion "0.2.0.0"
VIAddVersionKey /LANG=1033 "ProductName" "ClipVault"
VIAddVersionKey /LANG=1033 "FileDescription" "ClipVault per-user installer"
VIAddVersionKey /LANG=1033 "FileVersion" "0.2.0"
VIAddVersionKey /LANG=1033 "LegalCopyright" "ClipVault contributors"
!define MUI_ABORTWARNING
!define MUI_FINISHPAGE_RUN "$INSTDIR\ClipVault.exe"
!define MUI_FINISHPAGE_RUN_NOTCHECKED
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "../../LICENSE"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_COMPONENTS
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"
!insertmacro MUI_LANGUAGE "Russian"
Function .onInit
 ${IfNot} ${RunningX64}
  MessageBox MB_OK|MB_ICONSTOP "ClipVault requires 64-bit Windows."
  Abort
 ${EndIf}
 !insertmacro MUI_LANGDLL_DISPLAY
 FindWindow $0 "" "ClipVault"
 ${If} $0 != 0
  MessageBox MB_OK|MB_ICONEXCLAMATION "Quit ClipVault from its tray menu before installing or updating."
  Abort
 ${EndIf}
FunctionEnd
Section "ClipVault (required)" Core
 SectionIn RO
 SetShellVarContext current
 SetRegView 64
 ; WebView2 bootstrapper is Microsoft-signed and installs the runtime when needed.
 InitPluginsDir
 SetOutPath "$PLUGINSDIR"
 File /oname=MicrosoftEdgeWebview2Setup.exe "${WEBVIEW_BOOTSTRAPPER}"
 SetRegView 32
 ReadRegStr $1 HKLM "Software\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" "pv"
 ${If} $1 == ""
 ${OrIf} $1 == "0.0.0.0"
  ReadRegStr $1 HKCU "Software\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" "pv"
 ${EndIf}
 SetRegView 64
 ${If} $1 == ""
 ${OrIf} $1 == "0.0.0.0"
  DetailPrint "Installing Microsoft WebView2 Runtime (internet required)..."
  ExecWait '"$PLUGINSDIR\MicrosoftEdgeWebview2Setup.exe" /silent /install' $0
  ${If} $0 != 0
   MessageBox MB_OK|MB_ICONSTOP "WebView2 setup returned $0. Install Microsoft Edge WebView2 Runtime, then retry ClipVault setup."
   Abort
  ${EndIf}
 ${EndIf}
 SetOutPath "$INSTDIR"
 File /oname=ClipVault.exe "${APP_EXE}"
 File "../../LICENSE"
 WriteUninstaller "$INSTDIR\Uninstall.exe"
 WriteRegStr HKCU "Software\ClipVault" "InstallDir" "$INSTDIR"
 WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\ClipVault" "DisplayName" "ClipVault"
 WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\ClipVault" "DisplayVersion" "0.2.0"
 WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\ClipVault" "Publisher" "ClipVault contributors"
 WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\ClipVault" "InstallLocation" "$INSTDIR"
 WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\ClipVault" "UninstallString" '$\"$INSTDIR\Uninstall.exe$\"'
 WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\ClipVault" "NoModify" 1
 WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\ClipVault" "NoRepair" 1
 CreateDirectory "$SMPROGRAMS\ClipVault"
 CreateShortcut "$SMPROGRAMS\ClipVault\ClipVault.lnk" "$INSTDIR\ClipVault.exe"
 CreateShortcut "$SMPROGRAMS\ClipVault\Uninstall.lnk" "$INSTDIR\Uninstall.exe"
SectionEnd
Section /o "Desktop shortcut" Desktop
 CreateShortcut "$DESKTOP\ClipVault.lnk" "$INSTDIR\ClipVault.exe"
SectionEnd
Function un.onInit
 FindWindow $0 "" "ClipVault"
 ${If} $0 != 0
  MessageBox MB_OK|MB_ICONEXCLAMATION "Quit ClipVault from its tray menu before uninstalling."
  Abort
 ${EndIf}
FunctionEnd
Section "Uninstall"
 SetShellVarContext current
 SetRegView 64
 Delete "$INSTDIR\ClipVault.exe"
 Delete "$INSTDIR\LICENSE"
 Delete "$INSTDIR\Uninstall.exe"
 RMDir "$INSTDIR"
 Delete "$DESKTOP\ClipVault.lnk"
 Delete "$SMPROGRAMS\ClipVault\ClipVault.lnk"
 Delete "$SMPROGRAMS\ClipVault\Uninstall.lnk"
 RMDir "$SMPROGRAMS\ClipVault"
 DeleteRegKey HKCU "Software\ClipVault"
 DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\ClipVault"
 DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "ClipVault"
 ; User history and settings are intentionally preserved.
SectionEnd
