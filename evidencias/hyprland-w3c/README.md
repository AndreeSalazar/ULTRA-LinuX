# W3c: Hyprland, apps y preparacion NVIDIA

2026-10-19. Imagenes generico y PC construidas e instaladas en A:.
Hyprland oficial de Arch 0.56.2-4, commit
efb50993780079460b0cbed1363e2166a2de1d9f. Incluye Foot, Fuzzel, Waybar,
Thunar y Mousepad. [Uso, atajos y limites](../../docs/HYPRLAND-W3C.md).
No se ha modificado el renderer upstream ni integrado los contratos C++.

## Pruebas finales

| Prueba | VERDE | ROJO |
|---|---:|---:|
| Generico BMO | 80 | 0 |
| PC BMO | 84 | 0 |
| Seleccion automatica generico/PC | 11 | 0 |
| Recuperacion ante EFI PC invalido | 6 | 0 |

Ambos procesos terminaron con salida 0. Los logs *-pruebas.txt y *-serie.log
son de las imagenes finales, sin parches de diagnostico. QEMU usa virtio-gpu,
2 GiB de RAM y renderizado Mesa en CPU. No es una prueba de la RTX.
Las dos pruebas de seleccion/recuperacion tambien terminaron con salida 0;
modifican exclusivamente una copia desechable del disco virtual combinado,
no la particion A: ni las entradas de firmware del PC.

Se comprueban ventanas iniciales por IPC, apertura de editor/archivos/apps
con teclado emulado, ventana flotante, movimiento a otro escritorio y cierre
de una ventana sin terminar el compositor. Las capturas *-hyprland.png se
inspeccionaron visualmente: terminal, ayuda, editor, archivos y barra visibles.
La captura se toma antes de las pruebas de mover/cerrar ventanas.

Los PID de las apps se obtienen del IPC porque GTK puede cambiar comm/argv0.
Se inspecciona /proc de Hyprland, Foot, Mousepad y Thunar: cuatro UID/GID
1000, capacidades Inh/Prm/Eff/Bnd/Amb cero, NoNewPrivs=1. Tambien se verifica
la recuperacion Cage/Foot, limpieza de seatd, teclado de consola y persistencia
tras reiniciar. No se declara aislamiento entre apps del mismo usuario.

regresiones-terminal.txt comprueba el menu real en PTY y tres casos de
preparacion NVIDIA con backend simulado: valido, nodo ausente y error del
helper. Se reemplaza -c0 -u por -c 0 -m y se exigen los tres nodos graficos;
la prueba simulada no acredita su creacion en hardware real.

## Diagnostico previo

La primera imagen de ensayo intentaba anidar Hyprland en Cage/Pixman y no
pudo crear el backend. generico-primer-intento.txt y su consola conservan
el fallo. Se paso a DRM directo compatible con GBM y Mesa por software.

diagnostico-drm-virtual.txt y diagnostico-iniciador-oficial.txt son ensayos
en un overlay desechable de aquella imagen, con helpers sustituidos desde
la fuente. diagnosticar-overlay.py conserva ese procedimiento; se ejecuta
desde la raiz del proyecto. No son las pruebas de las imagenes finales.
Tambien se corrigieron carreras de lectura del banco serial: el prompt de
un proceso en segundo plano podia adelantar la respuesta del comando.

pc-antes/ conserva las fotos y los cuatro informes del PC aportados por el
propietario antes de W3c. Su secuencia fue ultra-escritorio --nvidia, Foot
visible, exit y consola con proveedor/driver EGL NVIDIA y salida 0 de Cage.
La foto de nodos ausentes es otro intento; no se atribuye a Hyprland.

## Integridad e instalacion

Los manifiestos, paquetes y fuentes.sha256 registran las versiones y las
fuentes. Los cuatro helpers de cada raiz coinciden con los archivos locales
(helpers-generico.sha256 y helpers-pc.sha256). Los logs de texto se normalizan
en finales de linea y espacios finales; originales en
build/hyprland-registros-originales, fuera de Git.

instalacion-generico.txt, instalacion-pc.txt y verificacion-instalacion.txt
registran la copia a BMO, disco 0, particion 2, y la comparacion SHA256 de los
dos EFI y los dos EROFS frente a los artefactos probados. No se modificaron
entradas de firmware. datos.ext4 conserva su estado posterior a las ultimas
pruebas fisicas del propietario:
60F96C6C7F16A0102C5990142AB83C90FA63D0F673833C23B1A2CDF5A75B816A.

El arranque anterior tiene copia verificada en
build/antes-hyprland-89feb19ec3d846069a02e3f6ba609c28; el registro local es
build/instalacion-hyprland.json. No se reemplazo datos.ext4.

## Siguiente prueba fisica

F11, perfil PC, `e`, Hyprland NVIDIA, Enter. Comprobar ventanas, teclado,
raton, Apps y salida con Win+Shift+M. El menu conserva Cage software como
recuperacion. El home actual es temporal en RAM y pierde documentos al
apagar: no se presenta como persistencia de escritorio terminada.
La prueba de nodos desde arranque limpio y Hyprland acelerado en RTX queda
pendiente del PC; no se anuncian FPS, rendimiento ni cobertura total de UB.
