# Espacio persistente en el NVMe

El informe fisico del 22 de octubre termina con `/juegos` al 100 %, cero bytes
disponibles para el usuario. Warframe aparece tanto en el registro Steam como
entre los procesos; no hay evidencia de que Steam desconozca sus archivos.
El usuario informa un error I/O y Retry del lanzador, con una pagina de ayuda
para Windows. Esto es compatible con el fallo de escritura por falta de espacio;
no acredita por si solo la causa de todos los errores o una averia del disco.

La [ayuda oficial I/O de Warframe](https://www.warframe.com/en/iofail) esta
dirigida a Windows. Aqui su lanzador corre mediante Proton. Antes de cambiar
la red o las banderas graficas se corrige la capacidad de su almacenamiento.
El informe no exporta el registro del perfil de Brave ni aporta tiempos de
fotograma del navegador; no se declara una mejora de FPS medida.

El usuario pidio mover el archivo a `C:\Espacio para Linux`. Se prepara una
copia de **96 GiB**, con UUID ext4 conservado, respaldando los 16 GiB originales
en `build/almacen-ampliado/ULTRA-juegos-respaldo.ext4`, fuera de Git. D: conserva
su archivo original. Una VM sin NIC amplia solamente la copia del proyecto;
no adjunta D:, no inicia clientes y solo comprueba metadatos del sistema de
archivos. La publicacion es una operacion posterior de Windows.

La primera ampliacion agoto su espera y se descarto como resultado valido.
Se restauro en la candidata el contenido del respaldo verificado y se repitio.
La ejecucion final completo resize2fs y e2fsck posterior con salida cero:
**86.575.235.072 bytes disponibles para el usuario (80,63 GiB)**. Se mantiene
el respaldo original; no se presenta el intento interrumpido como correcto.

La configuracion fija UUID del NTFS, ruta relativa limitada, UUID ext4 y tamano.
El preparador rechaza rutas redirigidas y no elige otro disco como sustituto.
Admite el formato anterior sin image_path para bancos y compatibilidad.
Cyberpunk original sigue en D:, con sus escrituras en la capa ext4 nueva.

C: necesita un montaje NTFS privado escribible para el contenedor. Su ancestro
es exclusivo de root; `/windows` y `/discos/<UUID>` usan vistas bind de solo
lectura, nosuid, nodev y noexec para las aplicaciones. Esto protege esas vistas,
no hace inmutable todo el dispositivo frente a root. No se fuerza un NTFS sucio
o hibernado ni se modifica Inicio rapido. Apagar Windows completamente antes
del siguiente arranque; si NTFS rechaza escritura, el preparador registra el
fallo y no sustituye el almacenamiento por otro archivo.

Brave comprueba 256 MiB libres antes de abrir el perfil y avisa si faltan.
Los accesos de juego comprueban 2 GiB antes de iniciar Warframe o Cyberpunk;
Steam sigue disponible con aviso para gestionar archivos. Son margenes de
prevencion, no una estimacion de cada actualizacion. No se borra cache, perfil,
prefijo ni archivo del juego. Warframe comprueba manifiesto y carpeta tanto en
las bibliotecas activas del selector como en la biblioteca antigua; Steam se
encarga de la compatibilidad y verificacion real.

Las pruebas de montaje usan NTFS/ext4 ficticios y copias de las imagenes ULTRA;
las pruebas de aplicaciones nunca reciben el contenedor personal. La evidencia
publica se guarda en `evidencias/almacen-c-2026-10-22` y los registros privados
quedan en build. Una partida de Warframe y el rendimiento de Brave en el PC
siguen requiriendo confirmacion despues del siguiente arranque.
