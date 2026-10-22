# Warframe de Windows en Steam de ULTRA

El kernel reconoce el disco; Steam mantiene su propio registro de bibliotecas.
Abrir un EXE en el explorador no prepara Steam, Proton ni el prefijo del juego.
Esta integracion selecciona Warframe (AppID 230410), no toda la biblioteca Windows.

## Uso desde el escritorio

1. Cierra Steam por completo, incluida su ventana de notificaciones.
2. Super+J > **Preparar Warframe de Windows**. La terminal muestra la ruta.
3. Abre Steam > Ajustes > Almacenamiento y agrega
   `/juegos/usuarios/ultra/Juegos/SteamLibrary`.
4. En Warframe > Propiedades > Compatibilidad, selecciona Proton. Usa
   Archivos instalados > Verificar integridad antes de jugar. Si aparece
   Instalar, selecciona esta misma biblioteca para descubrir los archivos.
5. Super+J > **Warframe / Steam** pide a Steam iniciar el AppID correcto.

La importacion del registro de instalacion no demuestra que el juego funcione
con Proton. No se fijan versiones antiguas de Proton ni se modifica su politica
global. El primer inicio puede descargar Proton, componentes y actualizaciones.
La seleccion de compatibilidad y el registro de la biblioteca se hacen en la
interfaz de Steam; el ayudante no reescribe `libraryfolders.vdf` de un cliente vivo.

## Montajes y aislamiento

La preparacion privilegiada de la sesion monta primero el almacenamiento de D:
y Windows C:, con sus identidades ya comprobadas. C: sigue en `/windows` con
`ro,nosuid,nodev,noexec` y su dispositivo de bloque en solo lectura.

`ultra-preparar-warframe` selecciona exclusivamente
`/windows/Program Files (x86)/Steam/steamapps/common/Warframe`, comprueba el
ejecutable y `Tools/Launcher.exe`, y rechaza rutas redirigidas por enlaces.
Monta un overlay en `/juegos/Warframe`: los originales son la capa inferior;
`/juegos/.ultra-warframe/upper` y `work` pertenecen al ext4 persistente. El padre
privado es de root, sin escritura de terceros. El escritorio usa el montaje
resultante sin capacidades de administrador.

La version publica `buildid` del manifiesto Windows se registra en el padre
privado. Si cambia entre montajes, se rechaza reutilizar la capa hasta revisarla:
no se borran partidas, modificaciones ni actualizaciones Linux para resolverlo.
Este control identifica cambios de version declarados por Steam; no es una
verificacion criptografica del contenido ni detecta todos los cambios manuales.
No actualices la copia Windows mientras Linux utiliza su capa.

El ayudante de usuario crea una biblioteca ext4, enlaza solamente Warframe y
prepara `compatdata` y `shadercache` en Linux. Del manifiesto Windows importa
solo AppID, nombre fijo, version, tamano y los numeros del depot 230411. Exige
que Steam Windows declare la instalacion completa. No copia LastOwner,
credenciales, sesiones ni configuraciones de cuenta. Conserva manifiestos y
destinos preexistentes; rechaza prepararlos con Steam abierto.

PID1 desmonta Warframe antes de Windows y del ext4. Un cierre limpio debe
mostrar `Warframe desmontado` antes de `NVMe desmontado` en el registro.

## Espacio y diagnostico

El juego original ocupa unos 52,4 GiB segun el manifiesto Windows observado.
La imagen persistente de D: tiene 16 GiB: no cabe otra copia completa alli.
El overlay evita esa copia inicial, pero actualizar un archivo grande puede
copiarlo entero a ext4; Steam tambien puede necesitar espacio de preparacion.
No se garantiza que una actualizacion quepa. No se amplia ni se formatea D:
automaticamente, ni se permite escribir en C: para sortear un fallo de espacio.

Super+J > **Estado y espacio** distingue discos visibles al kernel, montajes,
biblioteca preparada, RAM del sistema y VRAM NVIDIA. La presencia del disco o
de un manifiesto no acredita que Steam haya registrado o verificado la biblioteca.
Las consultas NVIDIA/Vulkan tienen tiempo limite. **Registros de juegos**
muestra los ultimos mensajes de los lanzadores y como activar temporalmente
`PROTON_LOG=1 %command%` para obtener `~/steam-230410.log`.

`procps-ng` se incluye explicitamente para consultar la RAM y comprobar procesos.
Si falta `pgrep`, la preparacion de biblioteca se detiene. La prueba del arranque
exige una lectura real de RAM y rechaza preparar la biblioteca mientras corre
un proceso ficticio llamado Steam; no basta con imprimir los titulos del informe.

Los registros de usuario pueden incluir rutas personales; no se publican
automaticamente. Los informes de GPU incorporan solo el resultado de preparacion
privilegiada y los metadatos del montaje, sin exportar cuentas de Steam.

## Validacion y limites

`scripts/probar-warframe.sh` usa un namespace de montajes en el taller, un NTFS
ficticio y un ext4 desechable. Comprueba 15 casos, incluida escritura como UID1000,
persistencia de la capa, rechazo de rutas inseguras, Steam activo, destinos ya
ocupados y cambio de version. Compara el SHA256 completo del NTFS antes/despues.

`scripts/probar_audio_juegos.py generico|pc --warframe --banco-grafica` arranca la
imagen ULTRA publicada en el proyecto con ese NVMe ficticio. Comprueba los
ayudantes incorporados, la importacion sin cuenta, el diagnostico, persistencia,
desmontaje ordenado y el hash inalterado. No pasa C: ni D: reales a QEMU.

Estas pruebas no ejecutan Warframe real, no autentican Steam ni demuestran FPS,
compatibilidad con su launcher o funcionamiento del antitrampas. La verificacion
de archivos y el arranque real con Proton quedan para el PC.

Referencias: [Proton de Valve](https://github.com/ValveSoftware/Proton) y
[bibliotecas NTFS en la wiki de Proton](https://github.com/ValveSoftware/Proton/wiki/Using-a-NTFS-disk-with-Linux-and-Windows).
La wiki es comunitaria y advierte que compartir bibliotecas NTFS no es una
configuracion soportada; aqui se separan escrituras, metadatos y prefijos en ext4.
