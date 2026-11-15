; Vessel's Windows installer: per user, no administrator, a Start menu
; entry, an entry in "Installed apps" and an uninstaller.
;
; [Decision Log]
; - Purpose: the owner installs a build with one double-click and finds
;   it in the Start menu; removes it from Settings like any app.
; - Constraints: built on Linux (makensis), no code-signing certificate
;   yet, no administrator rights expected; the app updates itself by
;   replacing its own executable, so the install directory must be
;   writable by the user.
; - Alternatives: WiX/MSI (Windows-only toolchain to build, per-machine by
;   default); Inno Setup (Zed's; Windows-only compiler, runs under wine at
;   best); MSIX (needs a signing certificate to install at all).
; - Choice: NSIS, as Tauri's default Windows bundle is: it builds on Linux
;   from the distribution's package, installs per user into
;   %LOCALAPPDATA%\Programs\Vessel (the place per-user apps use), and
;   writes the uninstall entry under HKCU.
; - Cost: the "Installed apps" version is the one installed; an update the
;   app applies itself does not change it (spikes/release/updater, the
;   install module, says why). SmartScreen warns about an unsigned
;   installer until the owner has a certificate.
;
; Built by spikes/release/build.sh:
;   makensis -DLABEL=<label> -DSTAGE=<dir with Vessel.exe, LICENSE.md,
;            README.txt and licenses\> -DICONS=<tools/make-icons.py's output>
;            [-DCOMPANIONS=1] -DOUTFILE=<setup.exe> installer.nsi
; COMPANIONS (ADR 0094): the command line (bin\vessel.exe) and the node
; (bin\vesseld.exe) are in STAGE and go in a bin folder beside the app.

; A 64-bit installer: the app is x64 only, so nothing is lost on Windows,
; and it runs under a wine without 32-bit support, where it was tested.
Target amd64-unicode
SetCompressor /SOLID lzma
RequestExecutionLevel user
ManifestDPIAware true

!ifndef LABEL
  !error "LABEL is required"
!endif
!ifndef STAGE
  !error "STAGE is required"
!endif
!ifndef OUTFILE
  !error "OUTFILE is required"
!endif
!ifndef ICONS
  !error "ICONS is required (the directory tools/make-icons.py wrote)"
!endif

!define NAME "Vessel"
!define PUBLISHER "Team-WICKED"
!define EXE "Vessel.exe"
!define UNINSTALL_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\Vessel"

Name "${NAME} ${LABEL}"
OutFile "${OUTFILE}"
InstallDir "$LOCALAPPDATA\Programs\${NAME}"
InstallDirRegKey HKCU "${UNINSTALL_KEY}" "InstallLocation"
BrandingText "${NAME} ${LABEL}"

!include "MUI2.nsh"
!include "LogicLib.nsh"
!include "StrFunc.nsh"
!include "WinMessages.nsh"
; StrFunc's replace, for the uninstaller (ADR 0094: the command line's
; folder in the user's PATH). Its comparison ignores case, as Windows'
; paths do.
${UnStrRep}

!define MUI_ABORTWARNING
; The mark (docs/design/brand/): the app's icon for the installer and the
; uninstaller, the mark in the brand colour on the pages' white header, and
; in white on the brand colour beside the finish page. The shortcut and
; the "Installed apps" entry take the icon embedded in Vessel.exe.
!define MUI_ICON "${ICONS}\vessel.ico"
!define MUI_UNICON "${ICONS}\vessel.ico"
!define MUI_HEADERIMAGE
!define MUI_HEADERIMAGE_RIGHT
!define MUI_HEADERIMAGE_BITMAP "${ICONS}\installer-header.bmp"
!define MUI_HEADERIMAGE_UNBITMAP "${ICONS}\installer-header.bmp"
!define MUI_WELCOMEFINISHPAGE_BITMAP "${ICONS}\installer-side.bmp"
!define MUI_FINISHPAGE_RUN "$INSTDIR\${EXE}"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

; Korean first: the owner's language is the installer's default; Windows
; in another language gets English.
!insertmacro MUI_LANGUAGE "Korean"
!insertmacro MUI_LANGUAGE "English"

VIProductVersion "0.0.0.0"
VIAddVersionKey /LANG=${LANG_ENGLISH} "ProductName" "${NAME}"
VIAddVersionKey /LANG=${LANG_ENGLISH} "ProductVersion" "${LABEL}"
VIAddVersionKey /LANG=${LANG_ENGLISH} "CompanyName" "${PUBLISHER}"
VIAddVersionKey /LANG=${LANG_ENGLISH} "FileDescription" "${NAME} installer"
VIAddVersionKey /LANG=${LANG_ENGLISH} "FileVersion" "${LABEL}"
VIAddVersionKey /LANG=${LANG_ENGLISH} "LegalCopyright" "${PUBLISHER}"

Section "Vessel"
  SetOutPath "$INSTDIR"
  File "${STAGE}\${EXE}"
  File "${STAGE}\LICENSE.md"
  File "${STAGE}\README.txt"
!ifdef COMPANIONS
  ; ADR 0094: the command line and the node, replaced with the app by its
  ; updates (spikes/release/updater, the install module).
  SetOutPath "$INSTDIR\bin"
  File "${STAGE}\bin\vessel.exe"
  File "${STAGE}\bin\vesseld.exe"
  SetOutPath "$INSTDIR"
!endif
  ; The licences of what the app embeds: the fonts' (SIL OFL 1.1 asks
  ; each copy to carry it) and any agent mark's that comes with one;
  ; build.sh's licenses() fills the folder.
  SetOutPath "$INSTDIR\licenses"
  File "${STAGE}\licenses\*.txt"
  SetOutPath "$INSTDIR"
  WriteUninstaller "$INSTDIR\uninstall.exe"

  CreateShortcut "$SMPROGRAMS\${NAME}.lnk" "$INSTDIR\${EXE}" "" "$INSTDIR\${EXE}" 0

  WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayName" "${NAME}"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayVersion" "${LABEL}"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "Publisher" "${PUBLISHER}"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayIcon" "$INSTDIR\${EXE}"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegStr HKCU "${UNINSTALL_KEY}" "QuietUninstallString" '"$INSTDIR\uninstall.exe" /S'
  WriteRegDWORD HKCU "${UNINSTALL_KEY}" "NoModify" 1
  WriteRegDWORD HKCU "${UNINSTALL_KEY}" "NoRepair" 1
SectionEnd

;; The app's "install the command line" (W19) adds exactly "<app>\bin" to
;; the user's PATH (HKCU\Environment, REG_EXPAND_SZ); its uninstaller takes
;; exactly that entry out again, keeping every other entry and the value's
;; type, and tells running programs the environment changed.
;; NSIS strings hold NSIS_MAX_STRLEN - 1 characters (1023 in the build's
;; makensis): a longer PATH would come back cut short, and writing it back
;; would lose the rest. It is left as it is then; an entry for a folder
;; that is gone does nothing.
Function un.RemoveBinFromPath
  ClearErrors
  ReadRegStr $0 HKCU "Environment" "Path"
  ${If} ${Errors}
    Return
  ${EndIf}
  StrLen $1 $0
  IntOp $2 ${NSIS_MAX_STRLEN} - 1
  ${If} $1 >= $2
    Return
  ${EndIf}
  ; Separators on both ends, so the entry matches whole wherever it is.
  StrCpy $3 ";$0;"
  ${UnStrRep} $3 $3 ";$INSTDIR\bin;" ";"
  StrCpy $3 $3 "" 1
  StrCpy $3 $3 -1
  StrLen $4 $3
  ${If} $4 == $1
    Return
  ${EndIf}
  ${If} $3 == ""
    DeleteRegValue HKCU "Environment" "Path"
  ${Else}
    WriteRegExpandStr HKCU "Environment" "Path" $3
  ${EndIf}
  SendMessage ${HWND_BROADCAST} ${WM_SETTINGCHANGE} 0 "STR:Environment" /TIMEOUT=5000
FunctionEnd

Section "Uninstall"
  Delete "$SMPROGRAMS\${NAME}.lnk"
  Delete "$INSTDIR\${EXE}"
  Delete "$INSTDIR\LICENSE.md"
  Delete "$INSTDIR\README.txt"
  ; Every text the installer put there (not just the fonts'), so the
  ; folder empties and RMDir removes it.
  Delete "$INSTDIR\licenses\*.txt"
  RMDir "$INSTDIR\licenses"
  ; What the app's own updates leave beside it: a build moved aside while
  ; it ran, and a download cut short.
  Delete "$INSTDIR\${EXE}.old-*"
  ; The companions (ADR 0094), whether this installer put them there or
  ; an update of the app did later; a node still running stays, and so
  ; does the folder.
  Delete "$INSTDIR\bin\vessel.exe"
  Delete "$INSTDIR\bin\vesseld.exe"
  Delete "$INSTDIR\bin\vessel.exe.old-*"
  Delete "$INSTDIR\bin\vesseld.exe.old-*"
  RMDir "$INSTDIR\bin"
  Call un.RemoveBinFromPath
  Delete "$INSTDIR\.vessel-update-*"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"
  DeleteRegKey HKCU "${UNINSTALL_KEY}"
SectionEnd
