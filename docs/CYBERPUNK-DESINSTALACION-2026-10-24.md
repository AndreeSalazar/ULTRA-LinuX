# Cyberpunk: recuperación tras una desinstalación incompleta

Heroic registró un intento de quitar `/juegos/Cyberpunk2077` a las 17:22
del 24 de octubre de 2026 y un error `EACCES` en `rmSync`. También intentó
ejecutar el desinstalador Windows. La inspección posterior encontró 49
whiteouts (dispositivos 0:0), sin archivos regulares ni subdirectorios en
la capa superior. Ocultaban, entre otros, `bin`, `archive`, `r6`, el ejecutable
de REDlauncher y los metadatos GOG. No se puede atribuir cada marca a uno de
esos dos pasos únicamente con este registro.

La base conservaba sus 4.991 archivos. La comprobación estática de dependencias
PE previa solo examinaba `bin/x64`: no detectaba el whiteout que ocultaba la
carpeta `bin` completa. Por tanto, su resultado sin DLL locales faltantes no
demostraba que la vista montada fuese utilizable.

Se ensayó la recuperación en una instantánea qcow2 con el disco físico de solo
lectura. Después se aplicó en la partición ext4 identificada por UUID y tamaño:

- Se respaldó íntegra la capa de marcas en
  `/juegos/.ultra-cyberpunk-desinstalacion-20261024/upper-desinstalacion`.
- Se creó una capa superior vacía del usuario del escritorio. No se cambiaron
  partidas, prefijos, runtimes, configuración Heroic ni archivos de la base.
- La vista OverlayFS permitió al UID 1000 abrir los 4.991 archivos,
  91.350.992.227 bytes; REDlauncher y Cyberpunk conservaron cabeceras PE válidas.
- Tras desmontar, `e2fsck -f -n` terminó sin errores. Una inspección independiente
  de solo lectura confirmó que la nueva capa seguía vacía y la base presente.

El usuario aclaró que el fallo anterior al intento de reinstalación mencionaba
mods. El texto exacto no está confirmado. Esta recuperación corrige el estado
posterior a la desinstalación; no prueba resolver aquel fallo, ni verifica el
contenido contra el catálogo actual de GOG. El registro detallado de Heroic
estaba desactivado en la última sesión. No se ejecutó el juego en QEMU.

En la base, las ubicaciones habituales `archive/pc/mod`, `red4ext/plugins` y
`r6/scripts` no aportaron entradas; `mods` solo contenía `.stub`. El archivo
`launcher-configuration.json` apunta a `bin\\x64\\Cyberpunk2077.exe` y a GOG.
Esto no demuestra qué aviso mostró REDlauncher ni audita todas las formas
posibles de modificación.

La partición ya está administrada por Linux y tenía aproximadamente 232 GiB
disponibles para el usuario. No se formateó ni se volvió a descargar el juego.
La reparación cambia datos persistentes, por lo que no requiere reconstruir
el kernel ni reemplazar los archivos de arranque de A:.

Para la próxima prueba: abrir Heroic desde Cyberpunk, activar el registro
detallado de ese juego y usar Jugar → REDlauncher. Si vuelve el aviso, conservar
el texto exacto antes de desinstalar. La verificación oficial de archivos puede
reparar diferencias de contenido, pero no debe confundirse con la comprobación
local de accesibilidad realizada aquí.

[Evidencias sanitizadas](../evidencias/cyberpunk-desinstalacion/2026-10-24).
