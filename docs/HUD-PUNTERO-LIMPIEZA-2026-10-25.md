# HUD, cursor de REDlauncher y limpieza de compilaciones

## Evidencia fisica recibida

Informe SHA-256 `8efea8e2f21c8a7a720e15a095ec0cbe03d422dde934d43a8ab463d0bf055073`.
El original se conserva en `build/respaldo-informes/20261025-hud-puntero`, fuera
de Git; se publican solo agregados sin cuentas, registros crudos ni partidas.

- Ultimas 300 muestras RAPL: paquete CPU 26,732 a 77,852 W, media 40,309 W.
- CPU identificada k10temp/Tctl: 55,1 a 78,5 grados C; media 62,259 C.
- Ventana monotona de energia: 547,735 a 1146,233 segundos. No usar el reloj civil,
  que salto unas cinco horas, para calcular duracion o potencia.
- Heronite, proceso principal: PSS medio 141,3 MiB, minimo 108,8 y maximo 206,1,
  en 100 observaciones. No es RSS exclusivo ni memoria total de sus juegos.

Cyberpunk aparece ejecutandose. Estas medidas no identifican funciones internas
del juego ni distinguen IA, fisicas o compilacion de sombreadores. Tampoco son una
comparacion controlada con los 91 C anteriores o con Heroic.

## MangoHud

La entrega anterior ya contenia MangoHud y lib32-mangohud 0.8.4-1: la ausencia de
overlay no demostraba desinstalacion. Se reinstalan ambos paquetes en la imagen.
El control de ULTRA se conecta a la opcion nativa `showMangohud` de Heronite,
global y por juego. Se conservan el resto de ajustes, prefijos, runners y limites.
No se demuestra una causa unica del fallo anterior: el informe no guardaba ese
ajuste ni las capas Vulkan del juego.

El primer inicio de esta revision restaura el HUD una sola vez. Despues, la opcion
Desactivar HUD de ULTRA persiste; si Heronite esta abierto se aplazan escrituras
de su configuracion hasta cerrarlo. Abrir de nuevo Steam tambien es necesario para
que sus procesos reciban el entorno nuevo. No se cierra ningun juego por fuerza.
Heronite y su login siguen excluidos del overlay; los juegos no.

Acceso: Super+J > FPS y HUD. En el juego, Shift derecho+F12 alterna visibilidad y
Shift izquierdo+F2 graba cuadros. Los nuevos informes incluyen paquetes y estado
de activacion nativa sin exportar configuraciones completas ni sesiones.

La [configuracion oficial de MangoHud](https://github.com/flightlessmango/MangoHud)
documenta activacion y exclusion por ejecutable. Heronite 0.15.0 usa
`showMangohud` para construir su envoltorio nativo de lanzamiento; heredar una
variable del launcher no sustituye comprobar esa ruta.

## Puntero

El usuario precisa que desaparece dentro de REDlauncher. Ya se utilizaban cursor
compuesto en NVIDIA y XCursor; repetir esos ajustes no solucionaba el caso.

`ultra-puntero-red` corre como UID1000 y escucha cambios de imagen mediante XFixes.
Solo sustituye una imagen completamente transparente bajo ventanas X11 de clase
REDlauncher.exe o REDprelauncher.exe. Deja intactos cursores visibles y el resto
de ventanas, incluido el juego; no inyecta clics, cambia drivers ni pide root.
En reposo espera eventos, con comprobacion de respaldo cada cinco segundos.
El rescate usa la flecha basica X11 dentro del lanzador. El tema habitual sigue
vigente fuera de ese caso.

No cubre ocultacion mediante mecanismos distintos de una imagen transparente ni
ventanas Wayland nativas. La comprobacion QEMU debe simular transparencia y
confirmar tambien que no modifica el cursor transparente de una ventana de juego.
La validacion final con REDlauncher real y NVIDIA corresponde al siguiente boot.
El informe registra el contador de reparaciones cuando las haya.
El exportador corre como UID1000, rechaza enlaces/FIFO, limita la lectura y emite
solo el contador y tiempo monotono. El informe privilegiado no abre ese archivo
con permisos root. La prueba dedicada verifica tambien que no exporte otros campos.

## Limpieza

Se retiraron 24 imagenes antiguas/duplicadas de Windows: 47,12 GiB. Se conservaron
las entregas vigentes y respaldos personales. En el taller se limpiaron targets de
Rust/eww/Carpetas y empaquetados antiguos, incluyendo un area obsoleta que quedaba
oculta bajo el montaje del disco de compilacion. Se conservaron fuentes, la etapa
actual, una reversion y todas las imagenes de datos.

Ambos discos virtuales se apagaron, con TRIM previo dentro del invitado, y se
convirtieron a qcow2 nuevos. `qemu-img compare` comprobo identidad de contenido
virtual y `qemu-img check` su integridad antes de sustituir los originales.
La capacidad virtual sigue en 64 GiB; el archivo de Windows se reduce.
Los tamanos aparentes borrados dentro del invitado no se suman al ahorro de
Windows, porque eso contaria el mismo espacio dos veces.

No se borraron respaldos de 96, 28 ni 16 GiB de datos personales, ni fuentes,
partidas, prefijos o el almacenamiento nativo de juegos. Los proximos builds
recrearan targets; conviene conservar solo una reversion de compilacion despues
de validar e instalar cada entrega, separandola de los respaldos personales.

## Entrega y pruebas

Instalado en A: FAT32, perfil PC. Kernel/NVIDIA conservados y `datos.ext4`
verificado identico al de antes de la intervencion. La entrega final paso 195
comprobaciones QEMU, con aplicacion Vulkan real lanzada por Heronite, biblioteca
MangoHud cargada y overlay dibujado, rescate de transparencia, cursor del juego
sin modificar y exportacion del contador desde UID1000. Se agregan cinco tests de
preferencias y un banco de enlaces/FIFO/filtrado de campos. Estos no sustituyen
la prueba con REDlauncher real en la RTX.

Lectura final de Windows: C: paso de 72,45 a 164,29 GiB libres (puede variar con
actividad del sistema). Se conservaron los respaldos personales y una reversion
de la ultima entrega anterior. Los targets se regeneraran al compilar de nuevo.
Las [evidencias](../evidencias/hud-puntero-limpieza/2026-10-25/) separan tamanos
aparentes, archivos compactados equivalentes y espacio observado en Windows.
