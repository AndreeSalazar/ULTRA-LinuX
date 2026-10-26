# MangoHud en Steam: preferencia y carga real

El informe fisico del 26 de octubre de 2026 contiene 112 muestras del VIGILANTE.
En las 107 que identifican `Warframe.x64.ex`, `requested` y `mapped` son falsos:
no se observo MangoHud solicitado ni cargado en ese proceso. La preferencia
global de Heronite estaba activada. Esto explica por que activar el atajo de
visibilidad no bastaba; no demuestra un problema de foco o de Wayland.

Heronite 0.15.0 sanea las variables `MANGOHUD*` antes de abrir Steam, en
`crates/launch/src/steam.rs` y `crates/steam/src/lib.rs`. Esa separacion evita
heredar el entorno completo del lanzador. Las opciones de lanzamiento de cada
juego Steam tienen su configuracion propia; no equivalen al valor global GOG.

ULTRA instala una entrada `steam` en `/usr/local/bin`, antes de `/usr/bin`
en el PATH de su escritorio. Esa entrada llama a `ultra-hud ejecutar` y despues
al lanzador original `/usr/bin/steam`, por ruta absoluta. Asi restaura la
preferencia explicita del usuario en el inicio del cliente nativo, sin cambiar
Heronite, Proton, prefijos, partidas ni `LaunchOptions` personales.

Se conserva `hud-desactivado`: al desactivarlo, los nuevos arranques reciben
`MANGOHUD=0`. La lista de exclusion mantiene Steam y sus ventanas fuera del HUD.
Los ajustes particulares de un juego todavia pueden anular el entorno global.
Una instancia de Steam ya abierta no recibe un entorno nuevo al pedir otro
lanzamiento: hay que cerrar Steam y el juego antes de probar la nueva imagen.
Esta activacion por `MANGOHUD=1` corresponde a Vulkan, incluido DXVK/VKD3D.
Los juegos OpenGL necesitan el envoltorio por juego `mangohud %command%`;
no se inyecta `LD_PRELOAD` globalmente en Steam. Vease el
[uso documentado de MangoHud](https://github.com/flightlessmango/MangoHud#normal-usage).

## Verificacion

`scripts/probar_hud_steam.py` verifica la frontera con un cliente simulado,
entorno saneado, activacion/desactivacion, argumentos literales y conservacion
del limite personal. Los bancos existentes de Heronite y el lanzador siguen
comprobando preferencias, enlaces y ausencia de cambios parciales.

`scripts/probar_arranque.py --hud-steam` prueba la entrada instalada en una
maquina desechable. Un namespace reemplaza solamente el cliente Steam por
`vkcube`; comprueba la biblioteca MangoHud y guarda una imagen para inspeccion.
No inicia un cliente con cuenta ni un juego personal. Esta prueba no sustituye
una sesion real Steam/Proton/NVIDIA: el siguiente informe debe confirmar
`mapped=true` en el proceso que dibuja Warframe y el usuario debe ver el HUD.

El banco completo termino con 189 comprobaciones verdes, incluidos reinicio,
persistencia y capturas. La imagen del overlay se inspecciono visualmente:
FPS y grafica de tiempos de cuadro presentes. En QEMU no hay sensor fisico
de CPU ni NVIDIA; un cero en temperatura de esa sonda no es una medicion real.
Evidencia: `evidencias/hud-steam/2026-10-26/`.

La captura personal figura guardada a las 10:57:14 y copiada al portapapeles.
El informe contiene su ruta, no la imagen. No se inspecciono esa imagen: el
acceso a la particion fisica fue rechazado por la revision automatica. Puede
adjuntarse el PNG desde `Imagenes/Capturas` para revisarlo sin exponer el disco.
