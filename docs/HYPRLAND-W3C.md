# W3c: Hyprland con aplicaciones

El aspecto y los controles actuales se describen en
[BMO-X / Night Circuit](ESCRITORIO-BMO-X.md); los efectos desactivados descritos
en esta etapa historica ya no son el valor inicial de NVIDIA.

Actualizacion: [W3d](GPU-NEON-W3D.md) corrige la omision del minor 255
(nvidiactl), sustituye la prueba insuficiente y agrega informes persistentes
y el tema neon. Las afirmaciones sobre -c 0 -m mas abajo son historicas y
no describen la preparacion vigente.

## Alcance

La imagen de ULTRA incorpora Hyprland oficial de Arch 0.56.2-4, revision
efb50993780079460b0cbed1363e2166a2de1d9f, sin parches al compositor. Se
construyen los kernels y las imagenes de ULTRA; esto no se presenta como
una compilacion propia de Hyprland ni como integracion de los contratos C++.
Las versiones de paquetes se guardan con la evidencia de cada imagen.

Incluye Foot, Fuzzel, Waybar, Thunar y Mousepad: terminal, buscador de apps,
barra, archivos y editor. La primera sesion abre ayuda y terminal en ventanas.
La barra usa America/Lima. Efectos de desenfoque, sombras y animaciones estan
desactivados como referencia sencilla; no se declara una mejora medida de FPS.

## Entrada y recuperacion

Desde la consola ejecutar `e`. Flechas y Enter; numeros 1-6 seleccionan,
Enter confirma; Q o Esc devuelve la consola.

1. Hyprland automatico: usa NVIDIA si el preparador detecta una unica NVIDIA;
   en otro caso solicita el modo de laboratorio GLES por software.
2. Hyprland NVIDIA: exige la GPU, proveedor EGL y nodos correctos.
3. Cage software: recuperacion con una terminal, renderer Pixman.
4. Diagnostico: resumen de credenciales, renderer, errores y avisos.
5. Estado NVIDIA.
6. Volver a la consola.

El modo software de Hyprland usa Mesa y sigue necesitando un dispositivo
DRM/GBM compatible. SimpleDRM/Pixman no basta: la primera prueba anidada
fallo al crear el backend. Las pruebas de ventanas usan virtio-gpu de QEMU,
DRM directo y LIBGL_ALWAYS_SOFTWARE=1. Cage/Pixman se conserva como una ruta
de recuperacion distinta. Ningun fallo NVIDIA cambia silenciosamente a CPU.

La interfaz conserva el log del lanzador en /arch/run/ultra-escritorio/lanzador.log,
el cliente en ultima.log y el broker en seatd.log (los tres dentro del mismo
directorio). Son temporales y corresponden al ultimo lanzamiento de su ruta;
pueden incluir avisos de intentos anteriores. El menu resume las lineas
relevantes; no interpreta esos textos como comandos.

CLI directa:

```sh
ultra-escritorio --hyprland --nvidia
ultra-escritorio --hyprland --software
ultra-escritorio --software
ultra-escritorio --nvidia
```

Las dos ultimas conservan Cage/Foot y su compatibilidad con pruebas anteriores.

## Teclado y raton

SUPER es la tecla Windows. SUPER+Enter abre terminal; SUPER+R abre Apps;
SUPER+E abre archivos; SUPER+N abre editor; SUPER+H abre ayuda.
SUPER+Q cierra la ventana activa, SUPER+V alterna flotante, SUPER+flechas
cambia foco. SUPER+1..9 cambia escritorio y SUPER+Shift+1..9 mueve ventanas.
SUPER y arrastrar con boton izquierdo mueve; con derecho redimensiona.
SUPER+Shift+M termina Hyprland. Tambien se puede pulsar Apps, Ayuda y Salir
en la barra. Cerrar una terminal ya no termina todo el escritorio.

## Preparacion NVIDIA corregida

Antes se ejecutaba nvidia-modprobe -c0 -u. -u selecciona la rama UVM en lugar
de crear los nodos graficos solicitados por -c. Ahora el preparador ejecuta
nvidia-modprobe -c 0 -m y verifica nvidia0, nvidiactl y nvidia-modeset como
dispositivos de caracteres antes de retirar privilegios. No depende de abrir
nvidia-smi primero, no crea nodos a mano ni cambia permisos globalmente.

La regresion prueba la orden grafica, la ausencia de un nodo y el fallo del
helper con un backend simulado; no sustituye el arranque fisico en RTX.

## Privilegios y limites

Root prepara udev, nodos y seatd. Hyprland se lanza mediante start-hyprland y
una sesion D-Bus privada como ultra (UID/GID 1000), sin capacidades y con
NoNewPrivs=1. AQ_DRM_DEVICES selecciona el dispositivo para Aquamarine;
las variables WLR del recuperador se retiran antes de iniciar Hyprland.
Se conserva el proveedor EGL NVIDIA explicito en el modo GPU.

El home vive en el overlay de RAM: **los documentos guardados alli se pierden
al apagar**. No se migra el contenido ni se cambia el propietario de /datos;
conserva sus permisos actuales. El home no esta conectado a esa persistencia.
No se han integrado portales, audio, red de escritorio, navegador ni juegos.
Las apps comparten UID y sesion: no se anuncia aislamiento por aplicacion.
No se cargan plugins de terceros ni se desactivan mitigaciones del kernel.

## Validacion

La configuracion Lua se verifica con Hyprland --verify-config. El banco de
arranque comprueba ventanas por IPC, abre editor, archivos y lanzador desde
el teclado emulado, inspecciona las credenciales reales de los procesos,
captura pantalla y termina mediante SUPER+Shift+M. Tambien prueba Cage,
limpieza de seatd, reinicio y datos persistentes en discos desechables.
Comprueba ventanas flotantes, mover a otro escritorio y cerrar solo una
ventana. Resultado final: generico 80/80 y PC 84/84; seleccion automatica
11/11 y recuperacion ante EFI invalido 6/6. Ambos perfiles instalados en A:
con hashes verificados y los datos existentes conservados.

Ver resultados finales e instalacion en
[evidencias/hyprland-w3c](../evidencias/hyprland-w3c/README.md).
La prueba de Hyprland sobre la RTX requiere otro arranque fisico. Las fotos
anteriores acreditan Cage/Foot con proveedor EGL NVIDIA; no se etiquetan
como una sesion Hyprland ni como una medida de rendimiento.

Fuentes: [paquete Arch](https://archlinux.org/packages/extra/x86_64/hyprland/),
[configuracion de la version](https://github.com/hyprwm/Hyprland/blob/v0.56.2/example/hyprland.lua),
[seleccion DRM Aquamarine](https://github.com/hyprwm/aquamarine/blob/v0.15.1/docs/env.md),
[nvidia-modprobe](https://github.com/NVIDIA/nvidia-modprobe/blob/main/nvidia-modprobe.c).
