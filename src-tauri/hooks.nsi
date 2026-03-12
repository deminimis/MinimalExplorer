!macro CustomInstall
  ; Register "Open in Minimal Explorer" for all folders
  WriteRegStr HKCR "Folder\shell\Open in Minimal Explorer" "" "Open in Minimal Explorer"
  WriteRegStr HKCR "Folder\shell\Open in Minimal Explorer\command" "" '"$INSTDIR\minimal-explorer.exe" "%1"'
  
  ; Register for drive directories (C:\, D:\)
  WriteRegStr HKCR "Directory\shell\Open in Minimal Explorer" "" "Open in Minimal Explorer"
  WriteRegStr HKCR "Directory\shell\Open in Minimal Explorer\command" "" '"$INSTDIR\minimal-explorer.exe" "%1"'
!macroend

!macro CustomUnInstall
  ; Clean up registry keys upon uninstallation
  DeleteRegKey HKCR "Folder\shell\Open in Minimal Explorer"
  DeleteRegKey HKCR "Directory\shell\Open in Minimal Explorer"
!macroend