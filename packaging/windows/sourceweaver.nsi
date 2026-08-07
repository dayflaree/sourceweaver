# -*- coding: utf-8 -*-

!ifndef VERSION
  !define VERSION "dev"
!endif

!ifndef PACKAGE_DIR
  !error "PACKAGE_DIR must point at the staged Windows package directory"
!endif

!ifndef OUTPUT_EXE
  !define OUTPUT_EXE "sourceweaver-${VERSION}-windows-x86_64-setup.exe"
!endif

!ifndef ROOT_DIR
  !define ROOT_DIR "."
!endif

Unicode true
Name "Source Weaver ${VERSION}"
OutFile "${OUTPUT_EXE}"
InstallDir "$LOCALAPPDATA\Programs\Source Weaver"
RequestExecutionLevel user

SetCompressor /SOLID lzma
Icon "${ROOT_DIR}\packaging\windows\sourceweaver.ico"
UninstallIcon "${ROOT_DIR}\packaging\windows\sourceweaver.ico"
VIProductVersion "0.0.0.0"
VIFileVersion "0.0.0.0"

VIAddVersionKey /LANG=1033 "ProductName" "Source Weaver"
VIAddVersionKey /LANG=1033 "CompanyName" "Source Weaver contributors"
VIAddVersionKey /LANG=1033 "FileDescription" "Source Weaver installer"
VIAddVersionKey /LANG=1033 "LegalCopyright" "MIT licensed"
VIAddVersionKey /LANG=1033 "OriginalFilename" "sourceweaver-setup.exe"

!include "MUI2.nsh"

!define MUI_ABORTWARNING
!define MUI_ICON "${ROOT_DIR}\packaging\windows\sourceweaver.ico"
!define MUI_UNICON "${ROOT_DIR}\packaging\windows\sourceweaver.ico"

!insertmacro MUI_PAGE_LICENSE "${ROOT_DIR}\LICENSE"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "English"

Section "Source Weaver" SecMain
  SectionIn RO
  SetShellVarContext current

  SetOutPath "$INSTDIR"
  File /r "${PACKAGE_DIR}\*.*"

  CreateDirectory "$SMPROGRAMS\Source Weaver"
  CreateShortcut "$SMPROGRAMS\Source Weaver\Source Weaver.lnk" \
    "$INSTDIR\sourceweaver-desktop.exe" \
    "" \
    "$INSTDIR\assets\sourceweaver.ico" \
    0
  CreateShortcut "$SMPROGRAMS\Source Weaver\Source Weaver CLI Help.lnk" \
    "$INSTDIR\sourceweaver.exe" \
    "--help" \
    "$INSTDIR\assets\sourceweaver.ico" \
    0
  CreateShortcut "$DESKTOP\Source Weaver.lnk" \
    "$INSTDIR\sourceweaver-desktop.exe" \
    "" \
    "$INSTDIR\assets\sourceweaver.ico" \
    0

  WriteUninstaller "$INSTDIR\Uninstall Source Weaver.exe"

  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Source Weaver" "DisplayName" "Source Weaver"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Source Weaver" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Source Weaver" "DisplayIcon" "$INSTDIR\assets\sourceweaver.ico"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Source Weaver" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Source Weaver" "Publisher" "Source Weaver contributors"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Source Weaver" "UninstallString" '"$INSTDIR\Uninstall Source Weaver.exe"'
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Source Weaver" "QuietUninstallString" '"$INSTDIR\Uninstall Source Weaver.exe" /S'
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Source Weaver" "NoModify" 1
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Source Weaver" "NoRepair" 1
SectionEnd

Section "Uninstall"
  SetShellVarContext current

  Delete "$DESKTOP\Source Weaver.lnk"
  Delete "$SMPROGRAMS\Source Weaver\Source Weaver.lnk"
  Delete "$SMPROGRAMS\Source Weaver\Source Weaver CLI Help.lnk"
  RMDir "$SMPROGRAMS\Source Weaver"

  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Source Weaver"

  Delete "$INSTDIR\Uninstall Source Weaver.exe"
  RMDir /r "$INSTDIR"
SectionEnd
