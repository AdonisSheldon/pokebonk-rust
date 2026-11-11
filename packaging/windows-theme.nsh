; Loaded before Tauri's pages; retain the stock install/update/uninstall logic.
; https://v2.tauri.app/distribute/windows-installer/#extending-the-installer
!define MUI_BGCOLOR "F5F1E8"
!define MUI_TEXTCOLOR "303048"
!define MUI_HEADERIMAGE_RIGHT
!define MUI_INSTFILESPAGE_COLORS "303048 F5F1E8"
!define MUI_DIRECTORYPAGE_TEXT_TOP " "
!define MUI_PAGE_HEADER_TEXT "PokéBonk"
!define MUI_PAGE_HEADER_SUBTEXT " "
!define MUI_FINISHPAGE_TITLE "$(^Completed)"
!define MUI_FINISHPAGE_TEXT " "
!define MUI_INSTFILESPAGE_FINISHHEADER_SUBTEXT " "
Caption "PokéBonk"
SpaceTexts none
AutoCloseWindow true

; The directory page already contains the only first-install choice.
!macroundef MUI_PAGE_WELCOME
!macro MUI_PAGE_WELCOME
  !undef MUI_PAGE_CUSTOMFUNCTION_PRE
!macroend

!macroundef MUI_PAGE_DIRECTORY
!macro MUI_PAGE_DIRECTORY
  !define MUI_PAGE_CUSTOMFUNCTION_SHOW PokeBonkDirectoryShow
  !insertmacro MUI_PAGE_INIT
  !insertmacro MUI_PAGEDECLARATION_DIRECTORY
!macroend

; Dialog units keep the controls aligned with both language fonts and DPI scaling.
!macro PokeBonkLiftDirectoryControl ID
  GetDlgItem $2 $0 ${ID}
  System::Call 'user32::GetWindowRect(p r2, p r1)'
  System::Call 'user32::MapWindowPoints(p 0, p r0, p r1, i 2)'
  System::Call '*$1(i.r3, i.r4)'
  IntOp $4 $4 - $5
  ; Preserve native size, stacking and focus (NOSIZE | NOZORDER | NOACTIVATE).
  System::Call 'user32::SetWindowPos(p r2, p 0, i r3, i r4, i 0, i 0, i 0x15)'
!macroend

Function PokeBonkDirectoryShow
  System::Store S
  FindWindow $0 "#32770" "" $HWNDPARENT
  SetCtlColors $HWNDPARENT "${MUI_TEXTCOLOR}" "${MUI_BGCOLOR}"
  SetCtlColors $0 "${MUI_TEXTCOLOR}" "${MUI_BGCOLOR}"
  GetDlgItem $2 $0 1020
  SetCtlColors $2 "${MUI_TEXTCOLOR}" "${MUI_BGCOLOR}"
  GetDlgItem $2 $HWNDPARENT 1028
  SetCtlColors $2 "${MUI_TEXTCOLOR}" "${MUI_BGCOLOR}"
  GetDlgItem $2 $HWNDPARENT 1256
  SetCtlColors $2 "${MUI_TEXTCOLOR}" "${MUI_BGCOLOR}"
  ; The removed paragraph would otherwise overlap the raised directory controls.
  GetDlgItem $2 $0 1006
  ShowWindow $2 0
  System::Alloc 16
  Pop $1
  System::Call '*$1(i 0, i 0, i 0, i 50)'
  System::Call 'user32::MapDialogRect(p r0, p r1)'
  System::Call '*$1(i, i, i, i.r5)'
  !insertmacro PokeBonkLiftDirectoryControl 1020
  !insertmacro PokeBonkLiftDirectoryControl 1019
  !insertmacro PokeBonkLiftDirectoryControl 1001
  System::Free $1
  System::Store L
FunctionEnd
