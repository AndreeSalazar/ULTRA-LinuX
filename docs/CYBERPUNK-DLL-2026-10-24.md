# Cyberpunk: DLL originales ocultas por la capa de escritura

La sesión del PC del 24 de octubre a las 15:07 (Perú) confirma que Heroic
recibe el protocolo de lanzamiento y ejecuta `REDprelauncher.exe`. REDlauncher
termina sin llegar al juego. El intento de recuperación directo dejó un
registro Proton con salida **53**, errores `import_dll` y estado **c0000135**:

- `ffx_backend_dx12_x64.dll`
- `ffx_fsr3_x64.dll`

Ambos archivos existen completos en la base Linux verificada. La capa
`/juegos/overlay/upper/bin/x64` contenía, con esos mismos nombres, dispositivos
de carácter 0:0: son **whiteouts de OverlayFS**, marcas que ocultan los archivos
de la capa inferior. No eran DLL vacías ni faltaba un paquete Vulkan del
sistema. La inspección no permite atribuir quién creó esas marcas.

## Reparación aplicada

Con Linux apagado, se verificaron la identidad de la partición, su UUID, el
recibo de la copia y la naturaleza de las dos marcas. QEMU solo recibió el
rango de la partición Linux, sin red, sin otras particiones físicas y sin
ejecutar el juego.

Se movieron únicamente las dos marcas a:
`/juegos/.ultra-reparaciones-cyberpunk-20261024/`, directorio privado de root.
No se borraron ni reemplazaron las DLL. El respaldo conserva las marcas,
la configuración anterior de Heroic y un recibo de reparación. No se debe
restaurar una marca mientras el overlay del juego esté montado.

Se ajustaron únicamente `targetExe` y `verboseLogs` en la configuración de
Cyberpunk, conservando el resto, incluido el runner y el prefijo. Heroic
admite esos campos en su [interfaz GameSettings de la versión 2.22.4](https://github.com/Heroic-Games-Launcher/HeroicGamesLauncher/blob/v2.22.4/src/common/types.ts).
El ejecutable seleccionado ahora es:
`/juegos/Cyberpunk2077/bin/x64/Cyberpunk2077.exe`.

## Comprobaciones y límite

Un montaje desechable de la unión confirmó que UID1000 puede leer ambas DLL
como archivos PE. Tras desmontar, ext4 pasó `e2fsck -f -n`. Una segunda
inspección independiente, con dispositivo de solo lectura y `ro,noload`,
verificó que los hashes de las DLL originales permanecen iguales y que las
dos marcas ya no están activas. No se tocaron partidas, prefijos, credenciales
ni los archivos de A:; no hace falta reconstruir para esta reparación de datos.

Esto corrige la causa comprobada del fallo de carga de DLL. El arranque completo
de Cyberpunk y la carga de partida deben probarse en el PC con su RTX; no se
ejecutó el juego en QEMU. Si vuelve a cerrarse, Heroic ya generará un registro
detallado para identificar el siguiente fallo sin atribuirlo al kernel o a la
GPU por suposición.

[Evidencias sanitizadas](../evidencias/cyberpunk-dll/2026-10-24).
