; Установщик Camunda Support для текущего пользователя: та же папка и ярлыки, что у прежней Tauri-версии.
Unicode true
!include "MUI2.nsh"

!ifndef VERSION
  !error "Передайте версию: makensis /DVERSION=1.2.3"
!endif

Name "Camunda Support"
OutFile "..\..\target\release\bundle\nsis\Camunda Support_${VERSION}_x64-setup.exe"
InstallDir "$LOCALAPPDATA\Camunda Support"
RequestExecutionLevel user
SetCompressor /SOLID lzma

!define MUI_ICON "..\..\assets\icons\icon.ico"
!define MUI_UNICON "..\..\assets\icons\icon.ico"
!define MUI_FINISHPAGE_RUN "$INSTDIR\Camunda Support.exe"
!define UNINSTALL_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\Camunda Support"

!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "Russian"

VIProductVersion "${VERSION}.0"
VIAddVersionKey /LANG=${LANG_RUSSIAN} "ProductName" "Camunda Support"
VIAddVersionKey /LANG=${LANG_RUSSIAN} "FileDescription" "Camunda Support"
VIAddVersionKey /LANG=${LANG_RUSSIAN} "FileVersion" "${VERSION}"
VIAddVersionKey /LANG=${LANG_RUSSIAN} "ProductVersion" "${VERSION}"
VIAddVersionKey /LANG=${LANG_RUSSIAN} "LegalCopyright" "malinatrash"

Section "Install"
  nsExec::Exec 'taskkill /F /IM "Camunda Support.exe"'
  nsExec::Exec 'taskkill /F /IM "camunda-support.exe"'
  SetOutPath "$INSTDIR"
  File "/oname=Camunda Support.exe" "..\..\target\release\camunda-support.exe"
  Delete "$INSTDIR\camunda-support.exe"
  WriteUninstaller "$INSTDIR\uninstall.exe"

  CreateShortcut "$SMPROGRAMS\Camunda Support.lnk" "$INSTDIR\Camunda Support.exe"
  CreateShortcut "$DESKTOP\Camunda Support.lnk" "$INSTDIR\Camunda Support.exe"

  WriteRegStr HKCU "Software\Classes\camunda-support" "" "URL:Camunda Support"
  WriteRegStr HKCU "Software\Classes\camunda-support" "URL Protocol" ""
  WriteRegStr HKCU "Software\Classes\camunda-support\DefaultIcon" "" "$INSTDIR\Camunda Support.exe,0"
  WriteRegStr HKCU "Software\Classes\camunda-support\shell\open\command" "" '"$INSTDIR\Camunda Support.exe" "%1"'

  WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayName" "Camunda Support"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "Publisher" "malinatrash"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayIcon" "$INSTDIR\Camunda Support.exe"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegDWORD HKCU "${UNINSTALL_KEY}" "NoModify" 1
  WriteRegDWORD HKCU "${UNINSTALL_KEY}" "NoRepair" 1
SectionEnd

Section "Uninstall"
  nsExec::Exec 'taskkill /F /IM "Camunda Support.exe"'
  Delete "$INSTDIR\Camunda Support.exe"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"
  Delete "$SMPROGRAMS\Camunda Support.lnk"
  Delete "$DESKTOP\Camunda Support.lnk"
  DeleteRegKey HKCU "Software\Classes\camunda-support"
  DeleteRegKey HKCU "${UNINSTALL_KEY}"
SectionEnd
