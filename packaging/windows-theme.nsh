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
