# BMO-X / Night Circuit: escritorio vivo

Diseño y controles: [documentacion](../../docs/ESCRITORIO-BMO-X.md).
Es una interfaz propia inspirada en la organizacion de end-4, construida con
Hyprland, Waybar, Fuzzel, GTK y Mako. No es la instalacion completa de Quickshell.

| Perfil | Comprobaciones correctas | Fallidas finales |
|---|---:|---:|
| Generico | 107 | 0 |
| PC | 111 | 0 |

Los ensayos arrancan copias de discos en QEMU con DRM virtual y render software.
Comprueban dos paneles mapeados, fuentes OTF, notificaciones disponibles,
teclado para activar/desactivar animaciones, apertura y cierre de ambos menus,
aplicaciones, movimiento de ventanas, sandbox de Brave, HTTPS y apagado limpio.
No acreditan rendimiento ni nuevos consumos de la RTX fisica. Los indicadores
visibles en las capturas son muestras del laboratorio mientras ejecuta pruebas.

`generico/` y `pc/` guardan pruebas, consola, capturas verificadas con el lector
PNG, lista de paquetes y hashes de las imagenes. `construccion-tema.txt` guarda
el reempaquetado visual sobre los kernels previamente construidos. El disco
doble tambien se actualizo con el nuevo EROFS generico y se verifico su hash.

Las prevalidaciones se conservan separadas. La primera encontro fuentes
WOFF2 sin glifos visibles y un control que necesitaba la API Lua. La comprobacion
de paneles tambien se adapto a los namespaces actuales y a esperar su mapeado;
las dos terminales iniciales podian aparecer antes que Waybar. No se cuentan
estos intentos como pruebas finales correctas.

Tambien se corrigio el capturador para esperar un PNG completo antes de copiarlo.
En la prueba PC, una copia parcial del fondo se sustituyo por el PNG original
completo de esa misma ejecucion; se verifico antes de conservarlo. Se repitio
el genérico con la comprobacion completa de PNG. Las imagenes no se retocaron.

`prueba-respaldos.txt` verifica que el instalador conserva datos y guarda informes
en el proyecto Windows, incluso antes de un reinicio intencional. Usa una unidad
temporal de archivos y no particiones reales. `respaldo-previo.json` identifica
los cinco archivos de A: respaldados y verificados antes de instalar el tema.
Los respaldos binarios estan en build, fuera de Git.

`instalacion-verificada.json` compara los dos kernels y los dos EROFS instalados
en A: con los probados y confirma que datos.ext4 sigue identico. El instalador
guarda ademas copias verificadas de los informes presentes en A: dentro del
proyecto Windows. En esta ronda no se recuperan eliminaciones de la papelera.

Los textos originales se conservan en `registros-originales.zip`; las copias
legibles normalizan finales de linea y espacios finales para Git. Los PNG
permanecen intactos. `archivos.sha256.json` registra los archivos entregados.

Queda confirmar aspecto, efectos y consumo con NVIDIA al arrancar el equipo real.
