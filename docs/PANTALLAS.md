# Pantallas, resolucion y escala

El panel se abre con **Super + P**, desde Apps como **ULTRA Pantallas y
resolucion**, o desde el centro BMO-X. Usa flechas, Enter y Escape.

1. Elegir cambiar una pantalla o recuperar perfiles guardados.
2. Seleccionar el monitor activo.
3. Elegir su modo nativo recomendado o una resolucion/frecuencia anunciada.
4. Elegir escala de interfaz: 1, 1.25, 1.5, 1.75, 2, 2.5 o 3.
5. Confirmar **Conservar y guardar** si se ve bien. Cancelar vuelve al ajuste anterior.

Resolucion es el numero de pixels, Hz es la frecuencia de actualizacion y escala
es el tamano de la interfaz. Cambiar escala no aumenta la resolucion fisica.
El panel conserva la posicion y transformacion actuales del monitor; no agrega
modelines, overclock, HDR ni un editor de disposicion de pantallas.

## Recuperacion y persistencia

Antes de aplicar se guarda el ajuste actual y se inicia un vigilante en otro
proceso. Si no se confirma dentro del plazo de 20 segundos desde el inicio del
cambio, intenta recuperar el ajuste anterior. Sigue existiendo aunque muera el
proceso principal. El resultado de IPC se contrasta con el modo y escala reales
informados por Hyprland antes de permitir guardar.

El perfil es privado del usuario, en `~/.config/ultra/pantalla.json`, con una
entrada por salida. No se ejecuta como Lua ni como comandos. Al iniciar el
escritorio se prueban los perfiles disponibles de nuevo con confirmacion. Si
cambias de monitor y el modo ya no existe, se omite ese perfil. Puedes recuperar
lo guardado manualmente desde el panel. Sin home persistente los perfiles se
pierden al apagar; no se copian datos personales a Git.

La vuelta depende de que Hyprland pueda seguir procesando cambios. Si muere el
compositor, no se puede recuperar la imagen desde su IPC; el arranque siguiente
comienza con el modo nativo por defecto y vuelve a pedir confirmar lo guardado.
La confirmacion de prueba no autoriza frecuencias que no anuncie el monitor.
Un modo anunciado tampoco garantiza que haya memoria o ancho de banda suficientes:
la confirmacion visual sigue siendo necesaria.

## Implementacion y alcance

`ultra-pantallas` usa `hyprctl -j monitors` y la API Lua `hl.monitor` disponible en
Hyprland 0.56.2. No aplica flags de SM86 ni depende de una GPU RTX concreta.
Acepta solo nombres de salida y campos de modo/escala/posicion/transformacion
validados. No interpola etiquetas libres como comandos de shell.

`probar-pantallas.ps1` ejecuta pruebas en un snapshot del taller. Los seis grupos
cubren entradas invalidas, inyeccion en nombres, lectura posterior del estado,
perfiles, confirmacion/cancelacion y un proceso principal realmente terminado
con SIGKILL: el vigilante separado recupera el estado despues de unos 20 segundos
con un IPC simulado. Es distinto de probar el compositor real.
Tambien comprueba que guardar una pantalla conserve el perfil de otra salida.

`scripts/pantallas-nativo.py` se transfiere solo a la VM desechable de
`probar_arranque.py`. Ejercita cancelacion, confirmacion, guardado, escala 1.25 y
restauracion con IPC real de Hyprland, sin adjuntar los discos personales. Usa un
perfil temporal de prueba y devuelve el modo original al terminar. El arranque
comprueba tambien que Super+P abra el selector real.

Para la entrega se reempaqueta el escritorio de los perfiles generico y PC con
`actualizar-escritorio.sh perfil fuentes`: conserva kernel y paquetes, actualiza
las fuentes propias y registra sus hashes. El modo completo anterior sigue
disponible para actualizaciones que requieran paquetes. El reempaquetado no
acredita por si solo el arranque ni el comportamiento del monitor fisico.

Referencias: [modos de monitor](https://wiki.hypr.land/configuring/core/monitors/),
[posicion, escala y transformacion](https://wiki.hypr.land/configuring/core/monitors/positioning/).

## Evidencia de esta entrega

Las pruebas unitarias finales estan en
`evidencias/pantallas/f6f663b966394233bfce03d9f71236e8/informe.txt`.
Un intento anterior observo el estado restaurado antes de que el vigilante
borrara el archivo de transaccion: se corrigio la espera de la prueba para
exigir ambas condiciones. Se conserva el informe fallido.

El primer intento de arranque fallo al transferir el ensayo por consola serie:
se redujeron los bloques de 1000 a 200 bytes. Posteriormente una prueba escogio
5120x2160 en Virtio y dejo la captura con «Display output is not active» pese
al estado informado por Hyprland. No se conto como prueba visual correcta.
El ensayo final acota el modo alternativo a 1920x1080 y exige color en la
captura del selector despues de restaurar. No limita los modos del panel real.

El generico final pasa 131 comprobaciones y la imagen PC 132, con cero fallos.
La captura del
[selector visible](../evidencias/pantallas/entrega-2026-10-22/pantallas-pc-final.png)
y los informes se guardan en `evidencias/pantallas/entrega-2026-10-22/`.
Estas pruebas usan salida virtual, no la RTX ni el monitor fisico.

Ambos escritorios se reempaquetaron y se copiaron a A: conservando los kernels.
El archivo `datos.ext4` mantiene su SHA-256 anterior, registrado junto con los
hashes de las imagenes en `entrega.json`. Los respaldos personales permanecen
en el proyecto Windows bajo `build/`, excluidos de Git. Esta entrega incluye
tambien el cierre normal de `ultra-graficos` ya probado anteriormente.
