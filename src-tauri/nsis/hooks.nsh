; Unscroll installer hooks.

!macro NSIS_HOOK_POSTINSTALL
  ; Sets up the background enforcer, the extension bridge and applies the block.
  nsExec::Exec '"$INSTDIR\unscroll.exe" --setup'
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  ; A running focus lock can't be bypassed by uninstalling.
  nsExec::Exec '"$INSTDIR\unscroll.exe" --uninstall-check'
  Pop $0
  IntCmp $0 0 unscroll_unlocked
    MessageBox MB_ICONSTOP|MB_OK "A focus lock is running. Unscroll can be uninstalled after the lock ends."
    Quit
  unscroll_unlocked:
  nsExec::Exec '"$INSTDIR\unscroll.exe" --cleanup'
!macroend
