# Steam: biblioteca ausente despues de actualizar Windows

El informe fisico del 27 de octubre muestra Warframe montado, almacenamiento
ext4 con 188 GiB libres y la biblioteca Steam seleccionada como no disponible:
`ya existe una capa anterior; respaldarla y revisar antes de duplicar otra version`.
El usuario ve `Instalar`. El informe no demuestra que el juego se haya borrado
ni contiene una orden de Windows que exija instalarlo.

La vista usa archivos Windows como origen de solo lectura y cambios Linux
en una capa ext4. Su identidad incluye los buildid de los manifiestos Windows.
Cuando cambia ese origen, se conserva la capa anterior para no mezclar archivos
Linux modificados con un origen cambiado. El limite de una sola generacion
impedia renovar la biblioteca en la siguiente actualizacion y la dejaba sin montar.
La firma corresponde a toda la biblioteca; puede cambiar por otro juego.

## Correccion

Se admiten hasta tres generaciones anteriores. Se mantienen las comprobaciones
de identidad, espacio minimo y espacio respecto a bloques de la capa activa.
No se borran respaldos, partidas ni archivos Windows. La renovacion existente
traslada prefijos, cache de shaders y runtimes Linux completos a la nueva vista;
no mezcla los archivos de juegos de una generacion antigua con el nuevo origen.

Si alguna biblioteca seleccionada sigue indisponible, el registro lo comunica
por stderr y, en una sesion Wayland, con un aviso. No se cambian StateFlags,
buildid ni los manifiestos de juegos para fingir una instalacion correcta.
Tampoco se escriben los VDF mientras Steam esta abierto.

El limite de tres generaciones es deliberado y puede alcanzarse. En ese caso
se conservan la capa activa y las anteriores para revisar y archivar; no hay
limpieza automatica de datos personales. La solucion definitiva para independizar
actualizaciones es una biblioteca nativa en ext4, sin un origen Windows compartido.
Esta entrega no migra ni duplica los juegos personales hacia otra biblioteca.

## Banco

`scripts/probar_bibliotecas_linux.py` usa un ext4 desechable y montajes overlay
reales en un namespace privado de QEMU. Comprueba tres renovaciones, registro
del juego tras cambios sucesivos, conservacion de prefijo/cache/runtime y
respaldos, rechazo por falta de espacio y por cuota agotada, y rechazo de enlaces
y rutas con escape. `scripts/probar_registro_steam.py` comprueba ambos VDF,
idempotencia, bloqueo con Steam abierto y aviso sin borrar juegos ni registro.

Evidencias: `evidencias/steam-renovacion/2026-10-27/`. El siguiente arranque
fisico debe confirmar que la biblioteca queda activa y Steam reconoce Warframe.
Steam todavia puede verificar archivos o actualizar una version nueva real.

Entrega PC instalada en A: con kernel sin cambios y `datos.ext4` conservado
por SHA-256 antes/despues. Validacion: 16 comprobaciones de montajes, 18 de
selecciones, 7 de registro y 189 del arranque QEMU, incluidos Heronite, MangoHud,
capturas, persistencia y apagado limpio. El banco de menus ahora espera el
cierre real de Fuzzel con plazo acotado en vez de suponer 0,4 segundos.
