# BMO-X / Night Circuit

Interfaz original inspirada en la organizacion de paneles y controles de
[end-4/dots-hyprland](https://github.com/end-4/dots-hyprland). Ese proyecto usa
Quickshell/QtQuick; aqui se amplian Hyprland 0.56.2, Waybar, Fuzzel y GTK ya
integrados en ULTRA. No se ejecuta su instalador ni se importan sus servicios,
funciones de IA o configuracion completa. Los recursos graficos son propios;
los iconos Papirus y Font Awesome proceden de paquetes oficiales de Arch.

## Aspecto y uso

- Negro y azul oscuro como superficies, cyan y verde esmeralda como acentos,
  rojo metalico para salir y detalles del fondo.
- Fondo SVG con fibra, metal, microrejilla, lineas de circuito y skyline.
  Es estatico: las texturas no necesitan un bucle de animacion permanente.
- Barra superior con escritorios, ventana activa, reloj de Lima, CPU, RAM y red.
  Los indicadores usan datos reales; un enlace conectado no acredita Internet.
- Dock inferior con Brave, archivos, terminal, editor y ayuda. Los botones
  tienen iconos, respuesta al pasar el raton y pistas con atajos.
- Papirus-Dark en el lanzador y aplicaciones GTK; controles, pestanas,
  selecciones y menus comparten colores. Las paginas web y aplicaciones con
  interfaces propias pueden conservar su diseño.
- Notificaciones Mako con iconos y avisos criticos en rojo. Tres visibles y
  diez en historial como limites de presentacion.
- Animaciones breves de ventanas/escritorios, sombras y blur de una pasada.
  El software arranca con estos efectos desactivados. No se declara una mejora
  de FPS, RAM o consumo energetico sin comparar muestras equivalentes.

Super+Espacio abre el centro BMO-X con flechas y Enter. Super+Shift+A alterna
efectos del compositor; Super+C limpia notificaciones. Los atajos anteriores
siguen disponibles. El modo de efectos no cambia GCC, permisos, sandbox,
frecuencias de hardware ni mitigaciones del kernel.

## Proyecto principal y datos de pruebas

El proyecto Windows `C:\Users\Salazar\Documents\Linux Reconstruido` es la
fuente principal; los cambios se guardan en Git local sobre main. A: es el
destino de pruebas de BMO-X. Una eliminacion intencional no se deshace
automaticamente buscando copias en la papelera.

Antes de copiar una imagen, el instalador respalda datos existentes en
`build/respaldo-datos/<SHA256>.ext4`, incluso si se pide `-NuevosDatos`.
Los informes TXT presentes en A: se copian con sus nombres y hashes a
`build/respaldo-informes/<fecha-UTC-identificador>/`. Se comprueba cada copia
antes de escribir en A:. Estos respaldos locales estan fuera de Git para no
versionar imagenes personales o gigabytes de datos. Las evidencias seleccionadas
para cada cambio se documentan separadamente en `evidencias/`.

`-NuevosDatos` expresa un reinicio intencional del ext4 de pruebas. Si se espera
una actualizacion y faltan los datos anteriores, el instalador se detiene para
evitar crear una semilla sin advertirlo. La copia principal se conserva en
Windows. El [almacenamiento de juegos](AUDIO-JUEGOS.md) permite conservar el
home del escritorio, perfil de Brave y descargas en el ext4 de D:. Cuando ese
almacenamiento no esta disponible, el escritorio usa un home temporal en RAM.

## Validacion

`scripts/actualizar-escritorio.sh generico|pc` actualiza las imagenes del taller
despues de una construccion completa. Verifica los hashes de entrada, conserva
el kernel, instala los recursos/paquetes visuales y reempaqueta EROFS y los
discos de prueba. Retira la marca de construccion completa durante el proceso:
si falta espacio o falla un paso, las imagenes no se descargan como terminadas.
El disco doble incluye tambien el sistema generico actualizado. No accede a
discos fisicos ni a A:. Requiere que no haya otra construccion en curso.

La regresion de escritorio comprueba dos superficies Waybar mapeadas, Mako e
iconos instalados, cambio de animaciones con teclado y apertura del centro.
Tambien mantiene las pruebas de aplicaciones, Brave con sandbox, HTTPS,
foco, movimiento entre escritorios y salida limpia. Las capturas QEMU permiten
revisar el aspecto; no miden aceleracion o rendimiento de la RTX fisica.

La primera prueba detecto glifos ausentes: el nombre virtual de fuente resolvia
a WOFF2. Se exige `otf-font-awesome` y se retira WOFF2 al actualizar la imagen
anterior. Tambien se adapto el control a `hyprctl eval` con `hl.config`, y las
aserciones a los campos `bool` y los namespaces `hud`/`dock` que devuelve 0.56.
Las pruebas iniciales se conservan como prevalidacion; no cuentan como pasadas.

Referencias de API: [ejemplo Lua de Hyprland 0.56.2](https://github.com/hyprwm/Hyprland/blob/v0.56.2/example/hyprland.lua),
[configuracion de Waybar](https://github.com/Alexays/Waybar/blob/master/man/waybar.5.scd.in)
y [Mako](https://github.com/emersion/mako/blob/master/doc/mako.5.scd).
