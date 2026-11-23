!define MACUI_INSTALLER_PROCESS_HELPER "${__FILEDIR__}\installer-processes.ps1"
!define MACUI_INSTALLER_RUNTIME_ROOT "${__FILEDIR__}\..\installer-runtime"

!macro NSIS_HOOK_PREINSTALL
  ; Only stop processes owned by this exact installation directory.
  InitPluginsDir
  !include "${MACUI_INSTALLER_RUNTIME_ROOT}\runtime.nsh"
  SetRegView 32
  ReadRegDWORD $0 HKLM "SOFTWARE\Microsoft\VisualStudio\14.0\VC\Runtimes\x64" "Installed"
  StrCpy $2 1
  ${If} $0 = 1
    ReadRegStr $1 HKLM "SOFTWARE\Microsoft\VisualStudio\14.0\VC\Runtimes\x64" "Version"
    StrCpy $1 $1 "" 1
    ${VersionCompare} "${MACUI_VC_VERSION}" "$1" $2
  ${EndIf}
  SetRegView 64
  ${If} $2 = 1
    ReadRegDWORD $0 HKLM "SOFTWARE\Microsoft\VisualStudio\14.0\VC\Runtimes\x64" "Installed"
    ${If} $0 = 1
      ReadRegStr $1 HKLM "SOFTWARE\Microsoft\VisualStudio\14.0\VC\Runtimes\x64" "Version"
      StrCpy $1 $1 "" 1
      ${VersionCompare} "${MACUI_VC_VERSION}" "$1" $2
    ${EndIf}
  ${EndIf}
  ${If} $2 = 1
    File /oname=$PLUGINSDIR\vc_redist.x64.exe "${MACUI_INSTALLER_RUNTIME_ROOT}\vc_redist.x64.exe"
    DetailPrint '$(macuiRuntimeInstalling)'
    ExecWait '"$PLUGINSDIR\vc_redist.x64.exe" /install /quiet /norestart /log "$TEMP\MACUI-VC-Runtime.log"' $0
    ${If} $0 = 3010
      SetRebootFlag true
    ${ElseIf} $0 != 0
      Abort "$(macuiRuntimeFailed)"
    ${EndIf}
  ${EndIf}
  File /oname=$PLUGINSDIR\macui-installer-processes.ps1 "${MACUI_INSTALLER_PROCESS_HELPER}"
  DetailPrint 'Stopping the existing MAC UI installation...'
  nsExec::ExecToLog '"$SYSDIR\WindowsPowerShell\v1.0\powershell.exe" -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "$PLUGINSDIR\macui-installer-processes.ps1" -InstallDirectory "$INSTDIR"'
  Pop $0
  ${If} $0 != 0
    Abort "Unable to stop the existing MAC UI installation. Close it and retry."
  ${EndIf}

  ; Remove only known previous binaries from this installation after they stop.
  Delete "$INSTDIR\mac-ui.exe"
  Delete "$INSTDIR\seelen-ui.exe"
  Delete "$INSTDIR\slu.exe"
  Delete "$INSTDIR\slu-service.exe"
  Delete "$INSTDIR\sluhk.dll"
  Delete "$INSTDIR\seelen_ui.pdb"

  DetailPrint 'Cleaning static folder from previous installation...'
  RMDir /r "$INSTDIR\static"

  DetailPrint 'Cleaning webview2 runtime from previous installation...'
  RMDir /r "$INSTDIR\runtime"

  File /a /oname=macui-hook.dll "${__FILEDIR__}\..\..\macui_hook.dll"
  File /a "${__FILEDIR__}\..\..\SHA256SUMS"
  File /a "${__FILEDIR__}\..\..\SHA256SUMS.sig"

  ; Gate File at compile time: a runtime If still embeds debug symbols.
  !searchreplace MAC_UI_VERSION_WITHOUT_NIGHTLY "${VERSION}" "nightly" ""
  !if "${MAC_UI_VERSION_WITHOUT_NIGHTLY}" S!= "${VERSION}"
    File /a "${__FILEDIR__}\..\..\macui.pdb"
  !endif
  !undef MAC_UI_VERSION_WITHOUT_NIGHTLY
!macroend

!macro NSIS_HOOK_POSTINSTALL
  DetailPrint 'Exec: macui-service.exe install'
  nsExec::Exec '"$INSTDIR\macui-service.exe" install'
  Pop $0
  ${If} $0 != 0
    Abort "Unable to register the MAC UI service. Run the installer again to repair this installation."
  ${EndIf}
  !insertmacro UPDATEFILEASSOC
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  InitPluginsDir
  File /oname=$PLUGINSDIR\macui-installer-processes.ps1 "${MACUI_INSTALLER_PROCESS_HELPER}"
  nsExec::ExecToLog '"$SYSDIR\WindowsPowerShell\v1.0\powershell.exe" -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "$PLUGINSDIR\macui-installer-processes.ps1" -InstallDirectory "$INSTDIR"'
  Pop $0
  ${If} $0 != 0
    Abort "Unable to stop MAC UI. Finish active file operations and retry."
  ${EndIf}
  DetailPrint 'Exec: macui-service.exe stop'
  nsExec::Exec '"$INSTDIR\macui-service.exe" stop'
  Pop $0
  DetailPrint 'Exec: macui-service.exe uninstall'
  nsExec::Exec '"$INSTDIR\macui-service.exe" uninstall'
  Pop $0
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  Delete "$INSTDIR\macui-hook.dll"
  Delete "$INSTDIR\SHA256SUMS"
  Delete "$INSTDIR\SHA256SUMS.sig"
  Delete "$INSTDIR\macui.pdb"
  !insertmacro UPDATEFILEASSOC
!macroend
