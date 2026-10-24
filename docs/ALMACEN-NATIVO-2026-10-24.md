# Almacenamiento Linux nativo y actualización de Cyberpunk

## Diagnóstico físico

El informe del 24/10/2026 registra Hyprland con NVIDIA y salida 0. En 120
muestras, los máximos observados fueron 3,859 GiB de RAM usada estimada,
497 MiB de VRAM y 50 °C en la GPU. No son máximos continuos ni un benchmark
de Cyberpunk: el informe no demuestra una partida de ese juego.

Ethernet negocia a 100 Mb/s, con un pico muestreado de 11,614 MiB/s y sin
errores o descartes en sus contadores de interfaz. El enlace sigue siendo
un límite físico; cambiar temporizadores no lo convierte en gigabit.

El registro local de Heroic ya contiene Cyberpunk `1423049311` en
`/juegos/Cyberpunk2077`. El registro específico de **actualización** sí contiene
un error de falta de espacio. En el ext4 de 96 GiB quedaban
6.399.770.624 bytes disponibles al usuario. El porcentaje 100% del cliente
no acredita que la actualización se haya aplicado y registrado correctamente.

El inventario en lectura encontró unos 57,6 GiB en las bibliotecas activas,
incluyendo 49,5 GiB de Warframe, y unos 21,3 GiB en la capa de Cyberpunk.
Son datos reales: no se eliminan juegos o partidas para ocultar el problema.
Los subtotales del inventario no se deben sumar entre sí.

## Destino autorizado

La antigua F: es la partición **3** del Kingston SA400S37480G de 480 GB,
disco Windows **0**, identificado también por serial en el registro local
de mantenimiento (fuera de Git):

| Campo | Valor |
|---|---|
| Inicio | 34.989.932.544 bytes |
| Tamaño | 445.113.171.968 bytes, unos 414 GiB |
| Sistema | ext4 |
| UUID nuevo | `19a78f63-4252-45e8-b032-5b6e53f67af5` |
| Montaje de ULTRA | `/juegos` |

Este disco también contiene EFI y A:. **No se formatea el disco completo**.
La operación trabaja exclusivamente sobre el volumen F: y un nodo de QEMU
acotado al tamaño de esa partición. Las lecturas de sus dos extremos se
compararon por SHA-256 con las lecturas nativas de Windows antes de escribir.
QEMU sin acotar anunciaba el tamaño del SSD completo; la guarda de geometría
impidió que ese acceso se usara para formatear.

La copia del contenedor de C: se realiza con el origen adjunto en solo lectura,
sin red ni clientes de cuentas. Incluye home, Steam, Heroic, prefijos, partidas,
cachés y capas de escritura. La comprobación compara SHA-256 de archivos,
propietarios, permisos, fechas de modificación, atributos extendidos, enlaces
simbólicos y grupos de enlaces duros. El contenedor original se conserva como
vía de recuperación; no se borra automáticamente al copiarlo.

## Montaje y pruebas

`partition_uuid` en `juegos.conf` selecciona la partición nativa. El preparador
exige un único dispositivo ext4 con ese UUID. Si falta o está duplicado, falla
sin abrir la copia antigua de C:, evitando que dos homes evolucionen separados.
Sin esa clave sigue disponible el modo anterior mediante contenedor NTFS/ext4.
Los originales de Cyberpunk en Personal siguen montándose en solo lectura.

## Entrega verificada

Se copiaron y verificaron **121.866 entradas**, con comparación SHA-256 de
archivos regulares y comprobación de metadatos y enlaces. ext4 pasó la revisión
final. Quedaron **338.691.977.216 bytes disponibles al usuario**, unos **315 GiB**.
La partición recibió el tipo GPT Linux filesystem; se retiró su letra F: de
Windows. EFI y A: conservaron sus posiciones, tamaños, GUID y tipos.

La imagen nueva del perfil PC se instaló en A: FAT32 y sus hashes se verificaron.
`datos.ext4` de A: permaneció idéntico. El banco completo pasó **186 comprobaciones**,
además de los seis escenarios de almacenamiento nativo. Se conserva el perfil
genérico anterior como recuperación y el contenedor de C: como copia anterior;
el uso normal de esta entrega es **ULTRA LinuX PC (Zen 3)** en F11.

El origen sigue en C: y no se ha borrado para recuperar sus 96 GiB: se conserva
hasta completar el primer arranque físico y comprobar las partidas.
Después del arranque, Heroic debe reanudar la actualización que falló por espacio.
Esta entrega no demuestra todavía que Cyberpunk cargue una partida.

Evidencias: [`almacen-nativo/2026-10-24`](../evidencias/almacen-nativo/2026-10-24).
La ubicación del registro de actualización se contrastó con
[Heroic 2.22.4, rutas de registros](https://github.com/Heroic-Games-Launcher/HeroicGamesLauncher/blob/v2.22.4/src/backend/logger/paths.ts).

El banco `scripts/probar_particion_juegos.py` ejecuta seis escenarios en QEMU
sin red y con un ext4 ficticio en RAM: ausencia sin fallback, montaje por UUID,
home escribible como UID1000, idempotencia, persistencia y UUID duplicado.
No conecta F:, C: ni juegos reales.

La actualización del sistema EROFS conserva el kernel y los ejecutables C
auditados; solo cambia el preparador de almacenamiento y su configuración.
El arranque físico, la reanudación de la actualización de Cyberpunk y la carga
de una partida deben verificarse después de activar la imagen nueva.
