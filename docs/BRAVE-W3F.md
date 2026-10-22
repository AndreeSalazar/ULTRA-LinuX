# W3f: Brave en el escritorio ULTRA

Objetivo: abrir Brave desde Hyprland, navegar por HTTPS y comprobar sus
procesos de paginas con sandbox, sin escribir comandos largos.

## Fuente y dependencias

Brave 1.97.59 estable, publicado el 16 de octubre de 2026, ZIP amd64 oficial:
[release upstream](https://github.com/brave/brave-browser/releases/tag/v1.97.59).
SHA256 publicado por la API de assets de ese repositorio:
`32979198ff09c4c656b0b060d929a44cdd6af167b2a141a77c2b7f36fd59e661`.
Se comprueba antes de extraer; no se ejecutan instaladores descargados.
El binario queda root:root y sin escritura para el usuario del escritorio.
El helper chrome-sandbox queda 0755, sin setuid.

[Brave documenta sus formatos ZIP/DEB/RPM y la opcion Arch](https://brave.com/linux/).
Esta integracion propia usa el ZIP oficial y bibliotecas de Arch; el navegador
conserva su binario upstream. No se le atribuyen las banderas GCC/G++ de los
preparadores C de ULTRA. La version queda fijada y sus actualizaciones se
integran reconstruyendo la imagen; este ZIP no queda gestionado por pacman.

Se agregan NSS/NSPR, ALSA como biblioteca, CUPS, bibliotecas X requeridas aun
usando Wayland, certificados CA, curl, iproute2 y dhcpcd. El build registra
ldd y rechaza bibliotecas ausentes o falta del almacen de certificados.
brave-origen.txt, brave-bibliotecas.txt y sistema-paquetes.txt acompanian
los artefactos descargados.

## Requisito de sandbox que faltaba

El kernel anterior tenia CONFIG_USER_NS desactivado. Se habilita y exige
en generico y pc; PID, IPC, red y seccomp ya estaban configurados.
El arranque anterior con chroot tampoco permite crear el user namespace
sin privilegios: la prueba negativa reproduce EPERM en el kernel del taller.

ultra-raiz, compilado con el perfil C de produccion y revisado con el
analizador, exige invocacion privilegiada. Crea un namespace de montajes
privado, monta la nueva raiz, usa pivot_root y desprende la raiz anterior.
Los montajes del proceso que lo invoca se conservan. No es setuid.
La sesion sigue preparando dispositivos como root y luego entrega el
compositor al UID 1000 con capacidades cero y NoNewPrivs=1.

La prueba positiva permite mapear el UID interno 0 al UID inicial 1000.
Root dentro del user namespace es una identidad limitada a ese namespace.
Se verifican los UID desde el procfs del namespace inicial y los namespaces
de los renderers. El navegador usa su sandbox de namespaces y seccomp; el
lanzador no pasa banderas para desactivarlo. Habilitar user namespaces
expone esa funcionalidad del kernel a procesos sin privilegios, como requiere
este sandbox: no equivale a eliminar todo riesgo del kernel o del driver.

La arquitectura multiproceso se describe en la
[documentacion de Chromium](https://chromium.googlesource.com/chromium/src/+/374888addcbefab05e8d3e9c4f806d18a135a9f5/docs/linux/sandboxing.md).

## Uso

1. Entrar con `e` y seleccionar Hyprland; en PC, NVIDIA.
2. Super+B abre Brave con una pagina local, sin necesitar internet.
3. Super+G abre brave://gpu; Apps incluye tambien «Brave - sandbox».
4. La pagina ofrece una prueba WebGL y un enlace a internet.
5. Super+Q cierra la ventana. Super+Shift+M termina el escritorio; Q sale del menu.

El perfil sigue el home activo: persistente con ULTRA-juegos.ext4 montado,
temporal en RAM cuando ese almacenamiento falta. Se integra un llavero cifrado
para las credenciales; consulta [persistencia y desbloqueo](BRAVE-PERSISTENCIA.md).
El launcher fija Wayland y omite el asistente inicial; conserva el sandbox.
Su registro queda en $HOME/.cache/ultra/brave.log.

## Red

Tras udev, el preparador intenta IPv4 DHCP en hasta cuatro interfaces
Ethernet con enlace. Reutiliza una ruta IPv4 existente. El cliente obtiene
DNS de DHCP y se ejecuta con un limite de espera; no se fijan DNS publicos.
Un fallo de red no impide abrir el escritorio y la pagina local.
El log esta en /arch/run/ultra-escritorio/red.log desde la consola exterior.
No se han configurado Wi-Fi, VPN ni un servicio de sincronizacion horaria.
La prueba HTTPS exige DNS, ruta y certificados validos; no ignora errores TLS.

## Pruebas y alcance

La prueba previa del taller cargo el HTML con Brave real y su sandbox.
El DOM registro WebGL 2.0, ANGLE/SwiftShader y error GL 0. Es software y
no demuestra aceleracion de la RTX ni que todos los formatos de video funcionen.

La regresion QEMU con --bmo --brave agrega una NIC virtio con red de usuario
sin puertos entrantes. Comprueba version, userns, apertura con Super+B,
UID/capacidades/NNP del navegador, renderer con UID inicial 1000,
namespaces user/PID distintos y seccomp=2, cierre sin perder Hyprland y HTTPS.
Las pruebas normales de ventanas, guardianes, exportacion y persistencia
siguen incluidas. Los resultados finales se guardan en evidencias/brave-w3f.

Generico: kernel #37, 319 paquetes, EROFS 607 MiB; 95 comprobaciones verdes,
cero fallos y salida 0. Incluye navegacion HTTPS real en Brave por la barra
de direcciones. La primera pasada fallo esa interaccion; red, certificados
y sandbox ya pasaban. La repeticion sincroniza las pulsaciones, sale de F11
antes de editar la direccion y verifica el PNG completo antes de copiarlo.
El intento fallido y su captura original completa se conservan.

El observador recogio siete muestras: RAM estimada 89.48-532.10 MiB y PSS
del UID1000 0.52-705.01 MiB durante inicio y apertura/cierre de aplicaciones.
Son datos QEMU/software de una secuencia activa, no reposo ni VRAM RTX.
Las medidas no son categorias excluyentes; ver [definiciones](MEDICION-W3E.md).

PC: kernel #25, 325 paquetes, EROFS 1009 MiB; 99 comprobaciones verdes,
cero fallos y salida 0. Siete muestras QEMU/software: RAM estimada
93.09-512.91 MiB y PSS UID1000 0.59-755.01 MiB. NVIDIA permanece empaquetado
para el PC fisico; QEMU no dispone de esa GPU.

Las dos EFI y las dos EROFS instaladas en A: coinciden por SHA256 con las
imagenes comprobadas. datos.ext4 conserva el hash anterior; se guardo una
copia verificada de las imagenes previas en build/antes-brave-<id>.

Pendientes fisicos para probar mas funciones:

- RTX: comprobar renderer y funciones de brave://gpu en este PC; comparar
  con los datos de RAM/VRAM del observador existente. No confundir SwiftShader
  con aceleracion NVIDIA.
- Audio: los informes previos muestran codecs HDA sin inicializar. Instalar
  libasound satisface el enlace del navegador; salida, microfono, permisos
  de dispositivos y servicios de sesion requieren una etapa propia.
  La configuracion recibida tiene SND_HDA_GENERIC, SND_HDA_CODEC_REALTEK y
  SND_HDA_CODEC_HDMI desactivados; no basta con cargar snd_hda_intel.
  Tambien se debe revisar la ruta de autoload /sbin/modprobe del initramfs
  antes de presentar el audio como operativo.
- Compartir pantalla: faltan la integracion de portal Hyprland/PipeWire y su
  prueba de permisos. Camara, DRM de contenido y aceleracion de video se
  validan por separado cuando sean necesarios.

Esta etapa hace utilizable el navegador como aplicacion y banco de pruebas;
no certifica todas las APIs web ni una aceleracion global del escritorio.
