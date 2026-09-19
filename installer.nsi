; Vortex Atoms AI - Installer
; NSIS 3.x required (a portable copy is kept under %LOCALAPPDATA%\nsis). Build:
;   & "$env:LOCALAPPDATA\nsis\nsis-3.10\makensis.exe" installer.nsi
; Output: dist\VortexAtomsAI-Setup.exe
;
; User-level install (no admin/UAC). Model lands in the user cache that the
; engine already looks at, so no download happens on first run.

Unicode true
Name "Vortex Atoms AI"
OutFile "dist\VortexAtomsAI-Setup.exe"
InstallDir "$LOCALAPPDATA\VortexAtomsAI"
RequestExecutionLevel user
SetCompressor /SOLID lzma

!include "MUI2.nsh"

!define APPEXE "vortex_api.exe"
!define APPJSON "vortex.json"
!define STARTSCR "start-server.ps1"
!define STOPSCR "stop-server.ps1"
!define README  "README.txt"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_LANGUAGE "English"
!insertmacro MUI_LANGUAGE "Arabic"

Section "Install" SEC_APP
  SetOutPath "$INSTDIR"
  File "dist\vortex-atoms-ai-app\${APPEXE}"
  File "dist\vortex-atoms-ai-app\${APPJSON}"
  File "dist\vortex-atoms-ai-app\${STARTSCR}"
  File "dist\vortex-atoms-ai-app\${STOPSCR}"
  File "dist\vortex-atoms-ai-app\${README}"

  SetOutPath "$INSTDIR\knowledge"
  File /r "dist\vortex-atoms-ai-app\knowledge\*"

  SetOutPath "$INSTDIR\models"
  File "dist\vortex-atoms-ai-app\models\qwen2.5-0.5b-instruct-q4_k_m.gguf"
  File "dist\vortex-atoms-ai-app\models\tokenizer.json"

  ; Seed the engine's model cache (LOCALAPPDATA\vortex_atoms_ai\models),
  ; mirroring what start-server.ps1 does anyway, so the server never downloads.
  SetOutPath "$INSTDIR"
  CreateDirectory "$LOCALAPPDATA\vortex_atoms_ai\models"
  CopyFiles /SILENT "$INSTDIR\models\*.*" "$LOCALAPPDATA\vortex_atoms_ai\models"

  WriteUninstaller "$INSTDIR\Uninstall.exe"
SectionEnd

Section "Start Menu shortcuts" SEC_SHORTCUTS
  CreateDirectory "$SMPROGRAMS\Vortex Atoms AI"
  CreateShortCut "$SMPROGRAMS\Vortex Atoms AI\Vortex Atoms AI Server.lnk" "$INSTDIR\${APPEXE}" "--port 8080 --tray"
  CreateShortCut "$SMPROGRAMS\Vortex Atoms AI\Start server (script).lnk" "$INSTDIR\${STARTSCR}"
  CreateShortCut "$DESKTOP\Vortex Atoms AI.lnk" "$INSTDIR\${APPEXE}" "--port 8080 --tray"
SectionEnd

Section "Uninstall"
  Delete "$INSTDIR\Uninstall.exe"
  RMDir /r "$INSTDIR"
  Delete "$DESKTOP\Vortex Atoms AI.lnk"
  Delete "$SMPROGRAMS\Vortex Atoms AI\Vortex Atoms AI Server.lnk"
  Delete "$SMPROGRAMS\Vortex Atoms AI\Start server (script).lnk"
  RMDir "$SMPROGRAMS\Vortex Atoms AI"
SectionEnd