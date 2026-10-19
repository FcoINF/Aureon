; Hooks de instalación de Aureon (NSIS).
; Durante la desinstalación se informa al usuario dónde quedaron sus datos
; financieros y que no se eliminan automáticamente.

!macro NSIS_HOOK_PREUNINSTALL
  DetailPrint "Aureon se está desinstalando."
  DetailPrint "Tus datos financieros permanecen en la carpeta de datos de la aplicación:"
  DetailPrint "$LOCALAPPDATA\com.aureon.app"
  DetailPrint "Si ya no los necesitas, puedes borrar esa carpeta manualmente."
  Delete "$DESKTOP\Aureon.lnk"
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
!macroend

!macro NSIS_HOOK_PREINSTALL
!macroend

!macro NSIS_HOOK_POSTINSTALL
  CreateShortCut "$DESKTOP\Aureon.lnk" "$INSTDIR\aureon.exe"
!macroend
