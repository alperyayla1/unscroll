; Unshorts installer hooks.

!macro NSIS_HOOK_POSTINSTALL
  ; Sets up the background enforcer, the extension bridge and applies the block.
  nsExec::Exec '"$INSTDIR\unshorts.exe" --setup'
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  ; A running focus lock can't be bypassed by uninstalling.
  nsExec::Exec '"$INSTDIR\unshorts.exe" --uninstall-check'
  Pop $0
  IntCmp $0 0 unshorts_unlocked
    MessageBox MB_ICONSTOP|MB_OK "A focus lock is running. Unshorts can be uninstalled after the lock ends."
    Quit
  unshorts_unlocked:
  nsExec::Exec '"$INSTDIR\unshorts.exe" --cleanup'
!macroend
