!macro CLIPLEAF_REMOVE_LEGACY_INSTALL KEY RUN_NAME
  ReadRegStr $R8 SHCTX "${KEY}" ""
  ${If} $R8 != ""
  ${AndIf} ${FileExists} "$R8\uninstall.exe"
    ReadRegStr $R7 HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "${RUN_NAME}"
    ${If} $R7 != ""
      StrCpy $R9 "1"
    ${EndIf}
    ExecWait '"$R8\uninstall.exe" /S _?=$R8'
    Delete "$R8\uninstall.exe"
    RMDir "$R8"
  ${EndIf}
  DeleteRegKey SHCTX "${KEY}"
!macroend

!macro NSIS_HOOK_PREINSTALL
  StrCpy $R9 ""
  !insertmacro CLIPLEAF_REMOVE_LEGACY_INSTALL "Software\clipman\ClipMan" "ClipMan"
  !insertmacro CLIPLEAF_REMOVE_LEGACY_INSTALL "Software\clipman\ClipLeaf" "ClipLeaf"
  DeleteRegKey /ifempty SHCTX "Software\clipman"
  SetOutPath $INSTDIR
  ${If} $R9 != ""
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "${PRODUCTNAME}" '"$INSTDIR\${MAINBINARYNAME}.exe" --minimized'
  ${EndIf}
!macroend
