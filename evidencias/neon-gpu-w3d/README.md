# W3d: control NVIDIA e interfaz neon

2026-10-19. Perfiles generico y PC compilados e instalados en A: con
hashes verificados y datos existentes conservados. [Uso y analisis](../../docs/GPU-NEON-W3D.md).

## Resultado

| Prueba final QEMU BMO | VERDE | ROJO |
|---|---:|---:|
| Generico | 84 | 0 |
| PC | 88 | 0 |

Ambos procesos terminaron con salida 0. QEMU usa virtio-gpu y renderizado
Mesa en CPU; no prueba la RTX ni acredita rendimiento fisico. Los logs de
pruebas y serie son de las imagenes finales sin parches temporales.

Se verifican menu, recuperacion Cage, ventanas Hyprland, editor, archivos,
lanzador, ventana flotante, cambio de escritorio, cierre, fondo Swaybg y
salida limpia. Hyprland y apps conservan UID/GID 1000, capacidades cero y
NoNewPrivs=1. Tambien pasan teclado, reinicio y datos persistentes del disco
virtual. Las capturas se revisaron: apps GTK oscuras, terminal y lanzador
coherentes, barra cyan/verde/rojo y fondo propio. Algunas capturas del fondo
incluyen el lanzador; no son maquetas.

La prueba comprueba que el rechazo NVIDIA mantiene salida 1, exporta el
informe a BMO y conserva el fallo anterior tras una recuperacion exitosa.
Se exige que el archivo de fallo contenga la ausencia real de nvidiactl
en esa VM sin NVIDIA. Los informes de sesion se copian al salir del menu
con Q o al terminar una invocacion directa.

## Regresion corregida

W3c solicitaba -c 0 -m; no solicitaba nvidiactl, minor 255. El stub anterior
creaba todos los nodos sin respetar los minors y dio una validacion
insuficiente. La nueva orden es -c 0 -c 255 -m. El modelo ahora crea solo
los minors solicitados y distingue UVM. regresion-validacion.txt muestra
cinco casos: correcto, nodo omitido, error del helper, antigua orden sin
control y antigua orden UVM. Ambas ordenes antiguas fallan en la prueba.
El modelo usa dispositivos de caracteres simulados; no reemplaza la prueba
con el binario y el driver en el PC.

Los validadores reales de Foot, Fuzzel e Hyprland aceptan sus configuraciones.
Foot exige colors-dark en la version instalada; se corrigio antes de construir.
pc-antes/ conserva las dos fotos y los informes aportados por el propietario
que muestran el rechazo en W3c. Esos datos no identificaban el nodo exacto
ausente; la omision del control se identifica inspeccionando el codigo.

## Integridad

fuentes.sha256 y helpers-empaquetados.sha256 registran fuentes y seis helpers
por perfil; los doce hashes se compararon con los archivos locales y coinciden.
Se conservan paquetes y manifiestos. Los textos se normalizaron en finales
de linea y espacios finales; originales en build/neon-registros-originales,
fuera de Git.

Los logs de instalacion y verificacion-instalacion.txt registran las copias
a FAT32 BMO, disco 0, particion 2. Los dos EFI y los dos EROFS coinciden con
los artefactos probados. No se modificaron entradas de firmware.
datos.ext4 conserva su hash posterior a las ultimas pruebas fisicas:
C70D2A6F13E85B9C62BB4ED8770390367000096EF28385C4C3860FEEE37E2F13.

La copia verificada del arranque anterior esta en
build/antes-neon-b66afd47fef74c3fb2428337a758e8a2; el registro local es
build/instalacion-neon.json. No se reemplazo la imagen de datos.

La correccion debe confirmarse desde un arranque limpio del PC, sin ejecutar
nvidia-smi antes. Los nuevos gpu-<kernel>.txt y gpu-fallo-<kernel>.txt apareceran
en A:\ULTRA\informe despues de probar y salir del menu. No se anuncia una
mejora medida de FPS ni una solucion fisica ya verificada. El home sigue en RAM.
