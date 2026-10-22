# Gráficos Vulkan/OpenGL y Windows en el NVMe

ULTRA ya incluía OpenGL/EGL mediante libglvnd/Mesa y el proveedor NVIDIA en el
perfil PC, además del cargador Vulkan, vulkan-tools y bibliotecas de 32 bits.
Esta ronda incorpora mesa-utils y las capas de validación Vulkan. Las versiones
reales por perfil quedan en sistema-paquetes.txt dentro y fuera de la imagen.
No se sustituye la implementación NVIDIA por una API propia.

## Uso con teclado

- Super + Shift + G: informe gráfico, cubo Vulkan y validación opcional.
- Super + Shift + E: abrir C: en el explorador, en /windows y solo lectura.
- Super + Space: centro BMO-X, con entradas para ambas funciones.
- Super + Shift + A: efectos vivos/reducidos para comparar el coste visual.

El informe consulta EGL en Wayland, OpenGL mediante Xwayland si está disponible,
y Vulkan. Distingue proveedores software y físicos y conserva códigos de salida.
Se guarda atómicamente en /run/user/1000/ultra-graficos.txt y, al cerrar la sesión,
se incorpora con un límite de tamaño al informe GPU exportado a A:.
La lectura del archivo del usuario se hace con UID1000, sin privilegios de root.
No es un benchmark ni convierte una consulta de capacidades en una mejora de FPS.
En QEMU, el perfil PC no tiene una RTX: su consulta Vulkan puede informar que
no hay dispositivo. El cubo Vulkan se valida en el genérico con lavapipe;
la ruta Vulkan de NVIDIA requiere la siguiente prueba física del usuario.

El cubo se limita a doce segundos. Validación activa VK_LAYER_KHRONOS_validation
solo para ese proceso; no la exporta a Hyprland, Brave o Cyberpunk. Las capas
son herramientas de diagnóstico y añaden coste. No se dejan activadas globalmente.

## Integración de servicios de escritorio

El informe real del 21 de octubre mostraba que el portal GTK no podía abrir
la pantalla y faltaban los servicios Settings y FileChooser. La sesión D-Bus
se creaba antes de que Hyprland asignara WAYLAND_DISPLAY. El nuevo iniciador
actualiza las variables de activación desde el compositor antes de abrir las
aplicaciones. No usa --systemd: el PID1 de ULTRA es el init propio en C.
La regresión consulta un servicio Settings activado realmente, además de
comprobar ventanas y aplicaciones. No certifica compartir pantalla o cámara.

## Convivencia con Windows

El usuario autorizó mostrar todo C: en solo lectura. Se identificó el NVMe
KINGSTON SNV2S1000G, cuya partición NTFS ShowOS tiene UUID D8BAB58CBAB567A0.
No se elige un disco por su número o por su tamaño. Si no hay exactamente una
partición NTFS con esa identidad, no se monta otro volumen como sustituto.

En el perfil original, el preparador pone el dispositivo de esa partición en modo de solo lectura
durante Linux, comprueba esa protección y monta /windows con ro,nosuid,nodev,
noexec. No reparticiona, repara, fuerza hibernación ni cambia archivos o ajustes
de Windows. Todo C: queda legible por las aplicaciones de la sesión: solo
lectura evita modificaciones, no impide copiar archivos desde ese volumen.
Root conserva la capacidad de cambiar esa política de montaje deliberadamente.

Windows tenía Inicio rápido habilitado; no se modificó. Si el estado de NTFS
impide montar, se registra el fallo y el escritorio continúa. Apagar Windows
completamente antes de probar. No se fuerza el montaje de un volumen incoherente.

Desde la migracion documentada en [Almacenamiento en C:](ALMACENAMIENTO-C.md),
el home y Proton usan un contenedor de 96 GiB en C:. Para permitir sus escrituras,
el volumen tiene un montaje privado de root y `/windows` usa un bind de solo
lectura; ya no se pone todo el dispositivo C: en modo de solo lectura en ese
caso. Si el contenedor no comparte volumen, se conserva la proteccion anterior.
Abrir C: no convierte sus juegos en instalaciones Linux. Un juego que necesite
escribir junto a su ejecutable requerirá una capa específica o importación.

## Investigación C++ separada

C++/laboratorio/grafica/sonda-vulkan.cpp usa un dispositivo CPU y rechaza una GPU
física. Crea un buffer válido de 4096 bytes y somete un buffer de tamaño cero a
la capa Khronos. El callback aborta la llamada incorrecta; se exige el VUID
VkBufferCreateInfo-size-00912 y VK_ERROR_VALIDATION_FAILED_EXT. No envía shaders,
no asigna memoria de GPU y no presenta comportamiento indefinido C++ intencional.

Se ejecuta junto a la sonda EGL existente dentro de bwrap sin /dev/dri, red ni
socket del escritorio, con 512 MiB, 64 tareas y tiempo acotado. El software
Vulkan se fija en lavapipe. Construcciones normal y ASan/UBSan quedan separadas.

El primer ensayo Vulkan con LSan informó 512 bytes tras descargar bibliotecas.
Se conserva su salida y se repite con la opción oficial del cargador
VK_LOADER_DISABLE_DYNAMIC_LIBRARY_UNLOADING=1. Las bibliotecas siguen mapeadas,
lo que mejora atribución y también cambia la alcanzabilidad de datos globales.
LSan no se desactiva ni se agregan supresiones. El resultado posterior no
certifica que el cargador, el driver y la capa carezcan de fugas en general.

Para compilar el banco: añadir -DULTRA_CXX_VULKAN=ON y -DULTRA_CXX_EGL=ON a CMake.
Se requieren vulkan-headers, vulkan-validation-layers y vulkan-swrast en el taller.
La sonda no se instala en producción ni se ejecuta con la RTX del usuario.

El banco de Windows se genera con scripts/crear-banco-windows.sh en una carpeta
nueva ~/linux-reconstruido/windows-prueba. Crea un NTFS ficticio de 256 MiB con
una marca de texto; ningún archivo real de C: entra en las pruebas. El modo
--windows de probar_audio_juegos.py conecta esa imagen a un controlador NVMe
virtual, prueba lectura y rechazo de escrituras de usuario y root, reinicia
y compara el hash íntegro de la imagen antes/después.
Tras descargar y verificar la imagen comprimida de Windows, se descomprime
en build/juegos/windows.img. Su archivo SHA256SUMS verifica ambos formatos.

La actualización del cliente Steam llenó el banco anterior de 6 GiB y ese
intento se interrumpió. Se conserva su salida como prevalidación fallida.
El banco final usa 12 GiB de ext4 dentro de un NTFS ficticio de 14 GiB,
creado desde cero en ~/linux-reconstruido/juegos-grafica con
scripts/crear-banco-juegos.sh. Se descarga a build/juegos-grafica y se selecciona
con --banco-grafica --windows al ejecutar probar_audio_juegos.py. No se amplía
ni se reinicia el archivo personal de 16 GiB de D:. Cada perfil instala el
cliente sobre su propia copia limpia del banco y registra el espacio restante.

Referencias: [integración de portales](https://flatpak.github.io/xdg-desktop-portal/docs/system-integration.html),
[variables D-Bus](https://dbus.freedesktop.org/doc/dbus-update-activation-environment.1.html),
[validación Vulkan](https://docs.vulkan.org/guide/latest/validation_overview.html),
[cargador Vulkan y sanitizadores](https://github.com/KhronosGroup/Vulkan-Loader/blob/main/docs/LoaderInterfaceArchitecture.md).
