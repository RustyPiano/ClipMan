!macro NSIS_HOOK_PREINSTALL
  ReadRegStr $R8 SHCTX "${MANUKEY}\ClipMan" ""
  ${If} $R8 != ""
  ${AndIf} ${FileExists} "$R8\uninstall.exe"
    ReadRegStr $R9 HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "ClipMan"
    ExecWait '"$R8\uninstall.exe" /S _?=$R8'
    Delete "$R8\uninstall.exe"
    RMDir "$R8"
    DeleteRegKey SHCTX "${MANUKEY}\ClipMan"
    ${If} $R9 != ""
      WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "${PRODUCTNAME}" '"$INSTDIR\${MAINBINARYNAME}.exe" --minimized'
    ${EndIf}
  ${EndIf}
!macroend
