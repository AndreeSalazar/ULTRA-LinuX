# Brave: perfil y llavero entre arranques

El informe de ULTRA del 22-10 registra el home persistente preparado en
/juegos/usuarios/ultra, pero Gmail pidio iniciar sesion despues de reiniciar.
La imagen no incluia GNOME Keyring ni arrancaba Secret Service. Esto es una
carencia de integracion comprobada, no una prueba de la causa concreta de Gmail:
no se han inspeccionado cookies, tokens ni contrasenas del usuario.

## Cambio

La imagen generica y la PC incluyen gnome-keyring y libsecret. La sesion DBus
del escritorio arranca solo el componente secrets como usuario 1000, antes de
abrir Brave. El daemon termina con la sesion. La inicializacion grafica ya
actualiza el entorno de activacion DBus para mostrar los dialogos bajo Wayland.

Brave fija su directorio habitual en
$XDG_CONFIG_HOME/BraveSoftware/Brave-Browser (por defecto $HOME/.config) y
selecciona gnome-libsecret. Antes de abrirlo comprueba que Secret Service
responda. Esa comprobacion no desbloquea ni lee el llavero. Chromium puede
recurrir a almacenamiento basic si su backend falla posteriormente; un Ping
correcto no demuestra que el llavero este desbloqueado ni que una cookie este
cifrada. No se fuerza basic, no se exportan tokens y no se modifica el sandbox.

El perfil y $XDG_DATA_HOME/keyrings (por defecto $HOME/.local/share/keyrings)
siguen el mismo home persistente. No se borran ni se reemplazan perfiles
existentes. La sesion y el launcher crean archivos privados con umask 077;
el directorio de llaveros queda 0700.

## Primer uso y comprobacion en el PC

1. Reconstruir e instalar la nueva imagen de ULTRA. Cambiar el repositorio no
   actualiza por si solo el USB ni la imagen EROFS que ya esta arrancada.
2. Confirmar que el escritorio usa /juegos/usuarios/ultra como HOME. Sin el
   disco de juegos el home es temporal; tambien lo son Brave y su llavero.
3. Al crear el llavero, elegir una clave no vacia. Al ser un escritorio con
   entrada automatica, puede pedir desbloquearlo al primer uso de cada sesion.
   Esta clave es del llavero; no se guarda la contrasena de Gmail en un script.
4. Abrir Brave normalmente e iniciar sesion en Gmail si lo pide. Un perfil
   previo sin este backend puede requerir iniciar sesion una vez mas.
5. Cerrar Brave y salir del escritorio antes de apagar. Reiniciar ULTRA,
   desbloquear el mismo llavero y comprobar Gmail en el mismo perfil normal.

No se puede garantizar que Google conserve una sesion: puede caducar o exigir
una nueva verificacion. Tampoco se conserva una sesion privada ni una cookie
configurada para borrarse al salir. Si se repite el problema, comprobar primero
el HOME activo, las opciones de borrado al cerrar, reloj del sistema y los
mensajes de llavero; no adjuntar bases de datos de cookies o tokens al informe.

La verificacion de archivos de Warframe es independiente del login de Gmail.
No se falsifican manifiestos ni se marca una descarga como verificada.

## Fuentes

- [Chromium: almacenes Linux](https://chromium.googlesource.com/chromium/src/+/HEAD/docs/linux/password_storage.md).
- [GNOME: arranque del llavero y sesiones sin PAM](https://wiki.gnome.org/Projects/GnomeKeyring/RunningDaemon).

Las pruebas sinteticas de scripts/probar_llavero.py usan un HOME desechable y
una clave de prueba no vacia. Comprueban dos sesiones DBus independientes con
el mismo llavero cifrado, sin cuentas reales. No equivalen a probar Gmail ni
los dialogos de Wayland en el PC.

Validacion del 22-10-2026: tres pruebas correctas en el taller Arch como UID
1000 (inicio real sin contrasena, preservacion del perfil y llavero cifrado
entre dos sesiones). En la prueba sin interfaz grafica, el desbloqueo se
simula inyectando una clave ficticia al daemon de prueba; el escritorio de
produccion no recibe esa clave. Bash valida los dos ayudantes y los dos
scripts de construccion/actualizacion sin errores de sintaxis.

Para repetir en un Linux con gnome-keyring, libsecret y dbus instalados:

    python3 scripts/probar_llavero.py
