# Steam, Heronite y Discord

Heronite 1.0.0 conserva la biblioteca unificada. Steam sigue siendo el cliente
necesario para sus juegos: Heronite le entrega un enlace `steam://`, no sustituye
su runtime ni Proton. La imagen incluye `xdg-utils`, el manejador
`ultra-steam-enlaces.desktop` y la base MIME de aplicaciones. Al abrir Heronite se
confirma la asociación de Steam y Steamlink. Los demás protocolos no se modifican.

El acceso Steam anterior redirigía a Heronite y no declaraba estos protocolos.
Además faltaba `xdg-open` en la imagen. Eso podía impedir iniciar Warframe aun
cuando el cliente y sus archivos estaban presentes. La prueba nueva usa el
`xdg-open` instalado y sustituye solamente el ejecutable final de Steam por un
programa Vulkan de prueba; comprueba también la carga efectiva de MangoHud.

## Actualizaciones de Heronite

La imagen EROFS es de solo lectura. El ejecutable empaquetado no podía
reemplazarse allí al anunciar una versión nueva. El iniciador ahora copia una vez
la AppImage oficial, verificada por SHA-256, a:

`~/.local/share/ultra/apps/heronite/Heronite.AppImage`

La copia pertenece al usuario. El actualizador de Heronite puede reemplazarla
allí con la verificación de firmas de upstream. Una nueva compilación de ULTRA
conserva esa copia y sus actualizaciones; no restablece una versión anterior.
La extracción usa `~/.cache/ultra/heronite-extract` sin requerir FUSE ni privilegios
adicionales. La configuración sigue en `~/.local/share/heronite`; no se migran ni
borran juegos, cuentas, prefijos o partidas como parte de esta corrección.

## Discord oficial

**Super+D**, «Discord» en SUPER o el lanzador de aplicaciones abren la aplicación
oficial de Linux. El bootstrap oficial instala el programa en el perfil
persistente `~/.config/discord`, donde puede actualizarse. El primer inicio
requiere red y espacio para descargarlo; `zenity` presenta su progreso. El usuario
inicia sesión manualmente. No se importan sesiones de Windows.
El escritorio puede pedir crear o desbloquear su llavero; no se configura una
contraseña vacía de forma automática.

El iniciador usa Wayland y el usuario normal del escritorio. Conserva el sandbox;
no añade `--no-sandbox`. El registro está en `~/.cache/ultra/discord.log`.
No se ejecuta el `postinst.sh` del archivo oficial, que detiene procesos y limpia
cachés de otros perfiles.

## Alcance de la comprobación

Los bancos usan una VM desechable, sin discos físicos, cuentas ni juegos
personales. El arranque real de Warframe, el inicio de sesión de Discord y una
futura actualización firmada de Heronite requieren confirmación en el equipo.
Las versiones, hashes, resultados y capturas de esta entrega se guardan en
`evidencias/clientes/2026-10-31`.

Se corrigió también el verificador serie: ahora delimita la salida de ejecución
con marcas por comprobación. El eco de un comando que menciona el texto esperado
ya no puede convertir un fallo en un resultado verde. El banco de Steam exige
cerrar el cubo anterior y registrar que el enlace completo llegó al cliente.

La entrega pasó 192 comprobaciones del banco completo, con dos arranques, sin
fallos. Una prueba adicional confirmó la ventana principal de Discord, UID 1000
y `NoNewPrivs=1`. Kernel, imagen instalada en A: y conservación del contenedor de
datos están registrados por SHA-256. Queda la prueba con la cuenta y el Warframe
reales del usuario; el banco sustituye únicamente el ejecutable final de Steam.
QEMU utiliza renderizado por CPU; sus FPS y los ceros en sensores ausentes no
representan el rendimiento ni la temperatura del Ryzen o de la RTX físicos.

Fuentes oficiales: [Heronite 1.0.0](https://github.com/RafaelGoulartB/Heronite/releases/tag/v1.0.0),
[Discord para Linux](https://discord.com/download).
