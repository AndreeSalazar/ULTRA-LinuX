# Audio maestro, Steam y GOG en BMO-X

El informe fisico de esta ronda identifica un GeneralPlus USB Audio Device
1b3f:2008 y dos controladores HDA (NVIDIA y AMD). El kernel anterior incluia
el controlador HDA, pero no sus codecs Realtek/HDMI/generico ni SND_USB_AUDIO.
De ahi los mensajes `Cannot probe codecs` y la ausencia de un driver de audio
para el USB. No se interpreta la interfaz HID del auricular como audio funcional.

## Audio

Los perfiles generico y PC incorporan SND_USB_AUDIO, HDA generico, Realtek y
HDMI como modulos, ademas de PipeWire, PipeWire-Pulse, WirePlumber, ALSA y
pavucontrol. El usuario ultra pertenece a audio; los procesos mantienen UID1000,
NoNewPrivs y capacidades efectivas cero. El servidor de sonido vive dentro de
la sesion D-Bus del escritorio y se cierra con ella, sin requerir systemd usuario.

Super+A abre el control maestro. Las teclas multimedia suben/bajan y silencian.
La barra muestra volumen y silencio; click abre el control y click derecho abre
pavucontrol para salidas y volumen por aplicacion. USB recibe prioridad 2000,
manteniendo disponible seleccionar otra salida. El boost a 125/150% es explicito;
el boton de subir se limita a 150%. El arranque no fuerza amplificacion.

La prueba usa un dispositivo USB Audio de QEMU y un backend WAV, no altavoces
o microfonos del host. Comprueba enumeracion, prioridad real, volumen aplicado,
limite, silencio y muestras PCM no nulas. La propagacion del volumen es asincrona:
se espera su valor efectivo, no se exige que otra consulta inmediata vea el cambio.
El tono ALSA tiene una ventana de diez segundos; el reloj USB emulado no se usa
para afirmar latencia, continuidad de reproduccion o calidad del sonido fisico.
La audicion y el comportamiento del GeneralPlus fisico se confirman en el PC.

## Clientes y juego

Steam y UMU proceden del repositorio multilib oficial de Arch. PC incluye
lib32-nvidia-utils con la misma version 615.78.08 que el controlador de 64 bits;
generico usa Mesa y Vulkan software para el laboratorio. IA32 sigue habilitado.
Se incluyen gawk y las bibliotecas XTest/XRandR de 32 bits: la presencia del
paquete Steam por si sola no acreditaba que su interfaz pudiera cargar.
La preparacion de la sesion crea los enlaces estandar /dev/fd y stdin/out/err.
La red activa loopback antes de comprobar Ethernet, incluso sin cable: Steam
usa TCP local para comunicar su interfaz WebUI con el cliente.
Heroic 2.22.4 procede del archivo tar.xz oficial, con SHA256 publicado por
GitHub `81497c4e67d42905da5839a32be053c011eae35fb1fd5ec9441249b238d8f99b`.
No se ejecutan instaladores de AUR ni se agrega una bandera para desactivar
el sandbox de Chromium/Electron. Las bibliotecas se verifican al construir.

Super+J y el icono del mando abren Steam, GOG/Heroic, Cyberpunk y estado/espacio.
Los clientes necesitan que el almacenamiento persistente este montado: si falta,
el escritorio sigue funcionando en RAM y el lanzador muestra un aviso, evitando
descargas grandes en el home temporal.

Cyberpunk se identifico como la edicion GOG 1423049311 en D:. El acceso directo
usa `bin/x64/Cyberpunk2077.exe` con UMU y GE-Proton; el primer inicio puede
descargar Proton y Steam Runtime. No exige comprar otra copia en Steam.
Las cuentas de Steam/GOG y cualquier confirmacion del cliente se completan por
el usuario. No se inicia sesion automaticamente ni se exportan credenciales.

## Almacenamiento sin reparticionar

D: Personal es NTFS, sin BitLocker, UUID `60A60325A602FAF4`. Se creo el archivo
ordinario `D:\ULTRA-juegos.ext4` de 16 GiB; antes habia unos 26,6 GiB libres.
No se modifico una tabla de particiones ni se formateo el volumen D:.
La imagen vacia verificada y sus hashes se conservan comprimidos en el proyecto
Windows, dentro de build/juegos. El archivo en D: es el estado vivo de los juegos.

La preparacion privilegiada identifica exactamente ese NTFS por UUID, comprueba
tipo, tamano e identidad del ext4, y monta el archivo con loop autoclear y
nosuid/nodev. No crea ni formatea imagenes durante el arranque. NTFS se monta
debajo de un directorio root 0700: el escritorio no puede recorrer el resto de D:.
No se usa force ni se elimina hibernacion si Windows no dejo el volumen limpio.

El home de la sesion pasa a `/juegos/usuarios/ultra`: alli quedan clientes,
configuracion, caches, prefijos y partidas. El tema se copia al primer uso y
se conservan las personalizaciones posteriores. Brave tambien sigue ese home.
La creacion de carpetas y la copia del tema se ejecutan con UID1000, sin seguir
rutas controladas por el usuario con permisos de root.
El antiguo datos.ext4 de A:, 64 MiB, continua separado para el estado del arranque.

`/juegos/Cyberpunk2077` es un overlay: lee la instalacion existente de Windows,
pero escribe en upper/work del ext4. Esto permite logs/modificaciones sin
sobrescribir el juego original. Los cambios en esa capa consumen espacio del
ext4, no se sincronizan automaticamente con Windows. Al actualizar el juego
en Windows conviene revisar la compatibilidad de sus mods y de esa capa.

Al apagar, PID1 detiene procesos, sincroniza y desmonta primero el overlay,
despues ext4/loop y finalmente NTFS, antes de liberar el sistema y A:.
Apagar Windows completamente antes de arrancar ULTRA. Si D: no se puede montar
con escritura normal, se informa el fallo y no se fuerza el volumen.

## Limites de las pruebas

La ronda final paso 254 comprobaciones: 107 de arranque en generico, 111 en PC
y 18 de audio/juegos por perfil. Incluye guardianes, Brave, DHCP/DNS/HTTPS,
cliente Steam con formulario visible, Heroic, permisos y persistencia tras
reiniciar. El generico reutilizo la cache del cliente del banco; PC comenzo
con un ext4 de laboratorio vacio. Ninguna prueba final inyecto ayudantes ni
bibliotecas en la imagen. Ambos perfiles se copiaron a A: con hashes
verificados y datos.ext4 conservado. Evidencias en `evidencias/audio-juegos`.

El laboratorio usa un NTFS propio, una imagen ext4 y un ejecutable ficticio.
Se comprueba que escribir en el overlay no modifica el original y que los
archivos sobreviven al reinicio. No se pasa D: real a QEMU. Las capturas de
Steam y Heroic acreditan la interfaz, no una sesion autenticada ni Cyberpunk
renderizando en la RTX. El juego y el sonido fisico requieren el siguiente boot.
El criterio de Steam exige la ventana de acceso a la cuenta y steamwebhelper;
un dialogo de error de transporte no cuenta como cliente operativo.
QEMU en Windows puede dejar longitudes RIFF/data en cero al salir con
no-reboot: la prueba valida los bytes PCM producidos y guarda una copia WAV
con longitudes correctas, sin sintetizar ni sustituir las muestras capturadas.

Para recrear el banco, `scripts/crear-banco-juegos.sh` se ejecuta en el taller
QEMU (requiere mkntfs, mkfs.ext4, mount, gzip y sudo). Solo acepta una carpeta
nueva `~/linux-reconstruido/juegos` o `juegos-prueba`; rechaza sobrescribirla.
Genera NTFS de 8 GiB, ext4 de 6 GiB y un ejecutable ficticio, junto con JSON
de identidades y hashes. Se copian `ntfs-lab.img.gz`, `ntfs-lab.json` y
`ntfs-lab.sha256` al directorio Windows `build/juegos`, se descomprime la
imagen y se verifica su hash. `scripts/probar_audio_juegos.py generico|pc`
arranca una copia; `--reanudar` conserva solo el disco desechable del cliente.
Los modos `--prevalidar`/`--deps-steam` son diagnosticos y se identifican como
tales en el JSON; las pruebas finales no los utilizan.

Referencias: [PipeWire](https://pipewire.pages.freedesktop.org/pipewire/page_man_pipewire_1.html),
[WirePlumber/ALSA](https://pipewire.pages.freedesktop.org/wireplumber/daemon/configuration/alsa.html),
[Heroic oficial](https://github.com/Heroic-Games-Launcher/HeroicGamesLauncher/releases/tag/v2.22.4)
y [UMU](https://github.com/Open-Wine-Components/umu-launcher).
