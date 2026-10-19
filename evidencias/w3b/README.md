# W3b inicial: seleccion NVIDIA GLES y sesion sin root

2026-10-19. Reconstruidos e instalados en A: los perfiles generico y PC.
Esta fase mantiene Cage/Foot: no instala Hyprland ni integra todavia los
guardianes C++/.gpus del laboratorio en el compositor.

## Resultado final QEMU

| Prueba | VERDE | ROJO |
|---|---:|---:|
| generico BMO | 66 | 0 |
| generico disco | 60 | 0 |
| PC BMO | 70 | 0 |
| seleccion automatica | 11 | 0 |
| recuperacion de EFI PC invalido en disco desechable | 6 | 0 |

Cada proceso final termino con codigo 0. Archivos *-pruebas.txt contienen
las salidas y *-serie.log las consolas. Las capturas muestran Foot cubriendo
el 97 % de la pantalla. Se verificaron teclado, retorno a consola, limpieza
del socket seatd y credenciales reales de Cage/Foot mientras ejecutaban:
UID/GID (real/efectivo/guardado/fs) 1000, CapInh/Prm/Eff/Bnd/Amb cero y NNP=1.

QEMU no tiene NVIDIA: --nvidia devuelve el error esperado y despues el modo
software abre la ventana. Esto prueba rechazo y recuperacion, no GLES RTX.
La seleccion del dispositivo y el proveedor NVIDIA deben probarse en el PC.

## Regresion encontrada y corregida

generico-primer-intento.txt y su consola registran la ventana fallida. El
diagnostico de secuencia reproduce el candado retenido por udevd despues de
rechazar NVIDIA ausente. diagnostico-correccion-overlay.txt aplica el cierre
del descriptor en una capa temporal de una VM desechable y confirma la
sesion; ese ensayo diagnostico no se presenta como imagen final instalada.
Despues se reconstruyeron ambos perfiles con la correccion y pasaron las
pruebas finales de la tabla.

fuentes-locales.sha256 registra fuentes y test; helpers-empaquetados.sha256
registra los helpers de ambas raices de sistema y se comparo con las fuentes
finales. Los manifiestos por perfil registran hashes de artefactos. Los
listados de paquetes fijan las versiones usadas. Logs normalizados en sus
finales de linea/espacios finales para Git; originales en
build/w3b-registros-originales, fuera de Git.

## Instalacion verificada

instalacion-generico.txt e instalacion-pc.txt registran las copias a la
particion FAT32 BMO de A:, disco 0, particion 2. No se modificaron entradas
de firmware. verificacion-instalacion.txt comprueba que los cuatro archivos
instalados coinciden por SHA256 con los artefactos probados.

datos.ext4 conserva el hash anterior a esta instalacion:
21869E552FE75AEE74298F7E60F68234C98FE120DE6F7E5EA318DDE6CC0DA5C5.
Es el estado posterior al arranque fisico del usuario, no el hash anterior
de W3a. Copias previas verificadas de los dos kernels y sistemas se guardaron
en build/antes-w3b-121c8a1863274d6292a248948e737da7; el registro local esta en
build/instalacion-w3b.json. Ninguna imagen de datos se reemplazo.

El primer intento de copia no se ejecuto porque la revision automatica de
aprobacion no pudo completarse por limite de uso. Tras el mensaje del usuario
"continua", se reintento por la misma via de aprobacion y termino correctamente.

## Prueba siguiente

Ver [comandos y alcance](../../docs/W3B-SESION-GPU.md). Arrancar el perfil PC,
ejecutar ultra-escritorio --nvidia, comprobar teclado y salir con exit. Traer
el GL renderer real de /run/ultra-escritorio/ultima.log dentro de Arch.
La preparacion root y seatd siguen siendo privilegiados; compositor y apps
comparten UID. No se certifican aislamiento global, FPS ni aceleracion RTX.
