# Contenedor en C: y errores de espacio

Los perfiles generico y PC aprobaron **18/18 comprobaciones cada uno** y se
instalaron en A:, conservando `datos.ext4`. El recibo `instalacion-A.json`
registra los hashes verificados. No se modificaron particiones ni firmware.

El informe fisico muestra `/juegos` al 100 %, sin espacio disponible. Steam
registra Warframe y se observa su proceso. El usuario informa I/O y Retry del
lanzador. La falta de espacio es una causa compatible; no se acredita aun una
partida ni se descartan otros errores del juego o del navegador.

`informe-resumen.json` resume el informe sin cuentas o sesiones. `ampliacion.json`
registra el resultado final de ampliar una copia de 16 a 96 GiB: UUID conservado,
e2fsck posterior cero y 80,63 GiB disponibles. No se publican datos del perfil ni
el contenedor personal. El original de D: y su respaldo en el proyecto Windows
se conservan. El archivo activo pasa a `C:\Espacio para Linux\ULTRA-juegos.ext4`.

La ampliacion inicial agoto su espera y no se considero valida. La candidata
se restauro desde el respaldo verificado antes de repetir la operacion; solo
la ejecucion final aprobada se publico. El mantenimiento no tuvo NIC, no adjunto
D: ni ejecuto clientes. Los registros completos estan fuera de Git, en build.

`generico.json` y `pc.json` corresponden a imagenes construidas, sin inyectar
fuentes ni exponer discos personales a las pruebas de aplicaciones. El banco
pone un ext4 en una subcarpeta de un NTFS ficticio, prueba escritura en Linux
y rechazo en la vista Windows, abre Brave y verifica persistencia tras reiniciar.
Tambien verifica hashes de los auxiliares y simula falta de espacio para el
perfil de Brave con un tmpfs de 128 MiB.

La captura del generico muestra Brave con la pagina local y el dialogo de
creacion de llavero de un perfil ficticio nuevo. No contiene una sesion personal.
La prueba usa renderizado por software: acredita apertura, no rendimiento GPU.

El acceso directo de Warframe busca manifiesto y carpeta en bibliotecas activas
del selector, con fallback al destino antiguo. `probar_busqueda_steam.py` prueba
esa resolucion sin Steam ni archivos reales, incluidos metadatos de una
actualizacion pendiente y rechazo de AppID/rutas invalidos. No sustituye la
verificacion de archivos o la compatibilidad que realiza Steam.

No se recompilan kernels, no se cambian banderas graficas de Brave ni se borra
cache, perfiles, prefijos o archivos de juegos. Los avisos de 256 MiB (Brave)
y 2 GiB (juegos) son margenes minimos, no garantia de espacio para actualizar.
La vista publica de C: es de solo lectura mediante bind; el montaje privado
que sostiene el contenedor es escribible por root. No se hace inmutable el
dispositivo completo ni se fuerzan NTFS sucios/hibernados.

Ver [detalle y limites](../../docs/ALMACENAMIENTO-C.md).
