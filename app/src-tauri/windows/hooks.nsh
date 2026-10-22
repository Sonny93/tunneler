; Installer hooks (tauri-bundler 2.10 NSIS template). tunneler.exe is a console
; program: nsExec runs it hidden and logs its output. A failure is logged and
; never blocks the installation or the uninstallation.

!define TUNNELER_EXIT_NO_DAEMON 3

!macro TunnelerRun ARGUMENTS
  nsExec::ExecToLog '"$INSTDIR\tunneler.exe" ${ARGUMENTS}'
  Pop $0
  ${If} $0 != 0
  ${AndIf} $0 != ${TUNNELER_EXIT_NO_DAEMON}
    DetailPrint "tunneler.exe ${ARGUMENTS} failed (result: $0)"
  ${EndIf}
!macroend

!macro NSIS_HOOK_PREINSTALL
  Push $0
  ${If} ${FileExists} "$INSTDIR\tunneler.exe"
    !insertmacro TunnelerRun "quit --yes"
  ${EndIf}
  Pop $0
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  Push $0
  ${If} ${FileExists} "$INSTDIR\tunneler.exe"
    ${If} $UpdateMode = 1
      !insertmacro TunnelerRun "quit --yes"
    ${ElseIf} $DeleteAppDataCheckboxState = 1
      !insertmacro TunnelerRun "purge --yes"
    ${Else}
      !insertmacro TunnelerRun "purge --yes --keep-config"
    ${EndIf}
  ${EndIf}
  Pop $0
!macroend
