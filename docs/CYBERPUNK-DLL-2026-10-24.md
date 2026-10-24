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

## Seguimiento: dependencias transitivas de FSR

La sesión de las 15:27 confirmó que el ajuste anterior persistió: Heroic
ejecutó `bin/x64/Cyberpunk2077.exe` y las dos DLL recuperadas seguían presentes.
Sin embargo, `ffx_fsr3_x64.dll` importa otras tres DLL que también estaban
ocultas por marcas 0:0 de la capa superior:

- `ffx_fsr3upscaler_x64.dll`
- `ffx_opticalflow_x64.dll`
- `ffx_frameinterpolation_x64.dll`

La primera reparación corrigió las dependencias directas, pero no revisó su
cadena transitiva; por eso persistía `c0000135`. El problema no era una imagen
antigua: el SHA-256 de A: coincide con el sistema entregado.

Se añadió `scripts/auditar_dll_pe.py`, lector estático de las tablas de
importación normal y diferida del [formato PE de Microsoft](https://learn.microsoft.com/en-us/windows/win32/debug/pe-format).
Recorre únicamente dependencias locales de la misma carpeta provistas por la
base y la capa superior. No ejecuta código, no resuelve las DLL de Windows de
Proton ni las cargas dinámicas decididas durante la ejecución. Tampoco sustituye
un montaje real de OverlayFS: la detección de ocultación cubre marcas de
carácter 0:0 en esa carpeta, no todas las variantes de metadatos de OverlayFS.

La auditoría recorrió **26 archivos PE** y encontró exactamente esas tres
marcas. Se apartaron a otro respaldo privado, sin tocar las DLL originales:
`/juegos/.ultra-reparaciones-cyberpunk-dependencias-20261024/`.
La auditoría posterior terminó con **cero dependencias locales ocultas**.
Un montaje de unión comprobó que UID1000 lee las tres DLL, y ext4 pasó
`e2fsck -f -n`. Ocho pruebas con PE ficticios verificaron el lector, incluidos
ciclos, dependencias transitivas, importaciones diferidas y entradas truncadas.
Una nueva inspección independiente de solo lectura confirmó 26 PE sin
dependencias locales ocultas y los hashes originales de las cinco DLL
recuperadas entre ambas reparaciones.

No se reconstruyó el kernel ni se reemplazó A: en esta reparación de datos.
El arranque del juego y la partida siguen pendientes del PC con RTX. El
resultado del lector no certifica todas las dependencias posibles en ejecución.

[Evidencias de la cadena completa](../evidencias/cyberpunk-dependencias/2026-10-24).
