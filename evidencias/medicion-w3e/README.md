# W3e: evidencia fisica y observador de memoria

`pc-real/` conserva la foto del escritorio real con dos ventanas y los
informes recibidos de A:. Las copias legibles usan LF y eliminan espacios al
final de linea; originales-informes.zip guarda sus bytes originales.
SHA256.txt identifica la captura inicial (informes dentro del ZIP y foto);
SHA256-normalizados.txt identifica las copias legibles.
El resumen JSON se genera despues y no forma parte de aquel manifiesto.

El informe fisico PC confirma RTX 3060, nodos, modo NVIDIA, Hyprland y
salida 0. No contiene RAM/VRAM: los valores null del resumen significan
ausencia de medicion. No hay informe GPU fisico del perfil generico.

`observador-fixtures.txt`: pruebas con procfs simulado de aritmetica RAM,
UID, memoria compartida/parcial, limite de procesos, nodos ausentes y parada.
`resumen-fixtures.txt`: conversion de unidades, deltas CPU, exclusion de
sumas incompletas y tratamiento NA/legado. Estos datos son sinteticos.

`generico-prueba-bmo.log` y `generico-hyprland.png`: arranque QEMU de la
imagen nueva, ventanas y guardianes; incluye RAM/PSS durante Hyprland y
exportacion separada software. QEMU usa 2 GiB y renderer software, sin RTX.

`pc-pruebas.txt`: 91 comprobaciones verdes, cero fallos y salida 0.
`pc-prueba-bmo.log`, `pc-hyprland.png` y `qemu-pc/` conservan el arranque,
las ventanas y el informe completo extraido. Cuatro muestras durante inicio
y apertura de apps: RAM estimada 84.96-283.83 MiB; no representa la RTX.
`instalacion-verificada.json` comprueba ambas EFI/EROFS instaladas y el hash
sin cambios de datos.ext4. Los manifiestos de cada perfil identifican las
imagenes probadas; las copias previas locales quedan fuera de Git.

La medicion fisica nueva se obtiene arrancando el perfil, usando `e`, dejando
al menos 30 segundos el escritorio abierto y cerrando con Super+Shift+M,
seguido de Q en el menu. Los informes quedan en A:/ULTRA/informe.
Las aplicaciones no reciben acceso a esa particion.

Diseno, limites y matriz de fronteras C/C++/GL:
[MEDICION-W3E.md](../../docs/MEDICION-W3E.md).
