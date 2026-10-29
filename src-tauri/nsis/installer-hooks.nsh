; Desktop shortcuts require an explicit choice. Tauri's default NSIS template
; checks the finish-page checkbox and creates a shortcut in silent/passive mode.
; Keep the Start Menu shortcut unchanged and preserve an existing desktop link
; across upgrades.
!define MUI_FINISHPAGE_SHOWREADME_NOTCHECKED

Var DesktopShortcutExisted
Var DesktopShortcutBackup
Var DesktopShortcutBackupFile
Var DesktopShortcutOriginalNoShortcutMode

!macro NSIS_HOOK_PREINSTALL
  StrCpy $DesktopShortcutExisted 0
  StrCpy $DesktopShortcutBackup ""
  StrCpy $DesktopShortcutBackupFile ""
  ${If} $PassiveMode = 1
  ${OrIf} ${Silent}
    ${GetOptions} $CMDLINE "/DESKTOPSHORTCUT" $R0
    ${If} ${Errors}
      ; The template creates a desktop link automatically in silent/passive mode.
      ; Suppress its normal path, then restore Start Menu behavior below.
      StrCpy $DesktopShortcutOriginalNoShortcutMode $NoShortcutMode
      StrCpy $NoShortcutMode 1
      IfFileExists "$DESKTOP\${PRODUCTNAME}.lnk" 0 desktop_backup_done
        StrCpy $DesktopShortcutExisted 1
        ; A sidecar directory survives an interrupted install so the original
        ; link remains recoverable. CopyFiles takes a destination directory.
        StrCpy $DesktopShortcutBackup "$DESKTOP\.what-changed-desktop-backup"
        StrCpy $DesktopShortcutBackupFile "$DesktopShortcutBackup\${PRODUCTNAME}.lnk"
        IfFileExists "$DesktopShortcutBackupFile" 0 +2
          Abort "Desktop shortcut backup already exists: $DesktopShortcutBackupFile"
        ClearErrors
        CreateDirectory "$DesktopShortcutBackup"
        IfErrors 0 +2
          Abort "Could not create the desktop shortcut backup directory. Installation stopped before changing it."
        ClearErrors
        CopyFiles /SILENT "$DESKTOP\${PRODUCTNAME}.lnk" "$DesktopShortcutBackup"
        IfErrors 0 +2
          Abort "Could not back up the existing desktop shortcut. Installation stopped before changing it."
    ${EndIf}
  ${EndIf}
  desktop_backup_done:
!macroend

!macro NSIS_HOOK_POSTINSTALL
  ${If} $PassiveMode = 1
  ${OrIf} ${Silent}
    ; /DESKTOPSHORTCUT is the explicit opt-in for non-interactive installers.
    ${GetOptions} $CMDLINE "/DESKTOPSHORTCUT" $R0
    ${If} ${Errors}
      ${If} $DesktopShortcutExisted = 1
        ClearErrors
        CopyFiles /SILENT "$DesktopShortcutBackupFile" "$DESKTOP"
        IfErrors 0 +2
          Abort "Could not restore the original desktop shortcut. Its backup remains at $DesktopShortcutBackupFile"
        Delete "$DesktopShortcutBackupFile"
        RMDir "$DesktopShortcutBackup"
      ${ElseIf} $WixMode = 1
        ; Wix migration bypasses the template's NoShortcutMode guard.
        Delete "$DESKTOP\${PRODUCTNAME}.lnk"
      ${EndIf}
      StrCpy $NoShortcutMode $DesktopShortcutOriginalNoShortcutMode
      ${If} $NoShortcutMode != 1
      ${AndIf} $WixMode != 1
        Call CreateOrUpdateStartMenuShortcut
      ${EndIf}
    ${EndIf}
  ${EndIf}
!macroend
