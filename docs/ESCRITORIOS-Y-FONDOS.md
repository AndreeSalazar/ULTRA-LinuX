# Escritorios, atajos y fondos preparados

La barra muestra los nueve escritorios numerados. El clic y Super+1..9 deben
seleccionar el mismo destino. Super+Tab abre una guia con destinos orientativos:
01 Trabajo, 02 Web, 03 Juegos, 04 Archivos, 05 Laboratorio y 06..09 libres.
Los nombres son una ayuda de organizacion; no trasladan aplicaciones por su cuenta.

## Atajos por tarea

| Tarea | Atajo | Accion |
| --- | --- | --- |
| Organizar | Super+1..9 | Cambiar de escritorio |
| Organizar | Super+Shift+1..9 | Mover la ventana al escritorio elegido |
| Organizar | Super+Tab | Mostrar los nueve destinos |
| Organizar | Super+flechas | Cambiar el foco entre ventanas |
| Organizar | Super+Ctrl+V | Alternar ventana flotante |
| Organizar | Super+arrastrar con boton izquierdo/derecho | Mover/redimensionar ventana |
| Buscar | Super+Espacio / Super+R | Centro SUPER / aplicaciones |
| Consultar | Super+F1 | Atajos con acciones seleccionables |
| Consultar | Super+H | Ayuda completa |
| Copiar | Super+V | Historial y pines persistentes |
| Capturar | Super+S / Super+Shift+S | Menu de captura / region |
| Configurar | Super+Ctrl+M | Mouse y teclado |
| Configurar | Super+P | Resolucion, frecuencia y escala |
| Configurar | Super+Shift+P | Tema, fondo y perfiles |
| Medir | Super+F8 / Super+F10 | Energia CPU / VIGILANTE |

## Clic de Waybar

Waybar 0.15.0 enviaba `dispatch workspace N`, incompatible con el despacho Lua
de Hyprland 0.56.2. Se construye la misma version desde un archivo oficial
fijado por SHA-256, adaptando las dos rutas de clic numerico a `hl.dsp.focus`.
Se conserva el modulo nativo y su estado activo, sin un observador adicional.
El alcance es el esquema numerado de este escritorio; no se presenta como una
adaptacion completa de nombres especiales o todas las ordenes de Waybar.

Origen del problema y correccion de referencia:
[Waybar #5008](https://github.com/Alexays/Waybar/issues/5008) y
[correccion upstream](https://github.com/Alexays/Waybar/commit/05945748dccce28bf96d26d8f64a9e69a8dd49ba).

## Fondos sin generar escenas durante la sesion

En Personalizar > Perfiles estan `neon-cyan`, `neon-violeta` y `neon-verde`.
Usan tres cuadros de ciudad 3D originales, rasterizados a PNG de 1920x1080
durante la construccion. Tienen efectos reducidos y borde quieto; las elecciones
se guardan en el perfil del usuario. Tambien se pueden elegir los fondos
`ciudad-cyan`, `ciudad-violeta` y `ciudad-verde` por separado.

El fondo Neon clasico y Nucleo viajan tambien en PNG preparados. Al generar un
acento propio, un unico PNG privado se reutiliza por huella de contenido; se
reemplaza cuando cambia el dibujo. No se acumula una copia por cada cambio.
Los SVG originales siguen disponibles. Swaybg adapta el PNG al monitor con fill;
un monitor de mayor resolucion puede escalar la imagen de 1080p.

Esto evita la generacion inicial repetida, no demuestra un aumento de FPS.
El fondo estatico anterior ya era una imagen en reposo. Una animacion
precalculada sigue decodificando y presentando cuadros; los modos vivos
anteriores quedan como opciones aparte.
[Uso de swaybg](https://github.com/swaywm/swaybg/blob/master/swaybg.1.scd).

Como catalogo externo para explorar fondos abstractos oscuros, esta la
[coleccion oficial GNOME](https://github.com/GNOME/gnome-backgrounds/tree/main/backgrounds):
Pixels, Glass y Blobs son alternativas al paisaje cyberpunk. Sus archivos
y creditos deben revisarse antes de incorporarlos; no se descargan ni se
instala GNOME para usar nuestros fondos. La coleccion documenta sus
[criterios de contraste, escala y atribucion](https://github.com/GNOME/gnome-backgrounds/blob/main/README.md).
Los archivos JPEG XL de esa coleccion requieren conversion previa a PNG o WebP
para el selector actual de imagenes propias de ULTRA.

## Siguientes mejoras con motivo

| Prioridad | Idea | Motivo y comprobacion |
| --- | --- | --- |
| Alta | Comparar reposo, escritorio y juego con el mismo escenario | Separar coste de barra, tarjetas, compositor y juego; medir PSS y tiempos de cuadro |
| Alta | Mostrar el perfil visual y almacenamiento activos en configuracion | Evitar confundir temas, discos y homes de sesiones anteriores |
| Media | Perfiles por dispositivo de entrada | Mantener sensibilidades distintas al conectar otro mouse |
| Media | Revisar los sensores restantes con sondeo fijo | Las tarjetas ya paran al ocultarse y consultan mas despacio en juego; ajustar otros observadores solo si la medicion lo justifica |
| Media | Vista de ventanas por escritorio en el menu | Encontrar una app sin recorrer nueve escritorios |

No se modifican syscalls, tiempos de red ni limites del kernel para arreglar
un boton de la barra. Los cambios visuales se verifican en QEMU y las mejoras
de consumo o rendimiento requieren medir el PC real.

[Fondos, pruebas e instalacion PC](../evidencias/escritorios/2026-10-31/README.md).
