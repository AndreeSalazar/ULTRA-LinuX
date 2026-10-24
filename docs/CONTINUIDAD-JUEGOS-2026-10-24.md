# Continuidad de Steam y de los perfiles de juegos

Steam volvió a descargar componentes de Proton después de haber funcionado.
La causa comprobada estaba en la renovación de bibliotecas de ULTRA: la firma
agrupa los buildid de los juegos de Windows. Al cambiar esa firma, se archivaba
toda la capa superior, incluidos los datos que pertenecen a Linux. La nueva
vista quedaba sin sus prefijos y runtimes aunque los anteriores seguían en el
archivo. Un cambio de otro juego podía afectar así al entorno de Warframe.

La inspección encontró Proton Experimental con el mismo buildid `25936768`
en la biblioteca activa y en la archivada. Sniper estaba instalado completo
en la anterior (`StateFlags=4`, build `26116003`), mientras la activa tenía
una preparación incompleta (`StateFlags=1026`, build `0`). El prefijo anterior
de Warframe conservaba sus ajustes; el reciente solo tenía el registro de
preparación. Esto no demuestra que Steam nunca deba actualizar Proton: una
versión nueva puede requerir una descarga legítima.

## Corrección de la renovación

Antes de exponer una nueva vista Steam, el preparador conserva activos
`compatdata`, `shadercache` y los runtimes Linux instalados identificados por
sus manifiestos. Reutiliza sus carpetas y metadatos, sin recorrer recursivamente
los prefijos como root. Los accesos usan directorios fijados por descriptor y
aperturas sin seguir enlaces; los manifiestos tienen tamaño y rutas acotados.
Los directorios opacos siguen ocultando los prefijos Windows del origen.

La capa de archivos de juegos anterior permanece archivada. No se mezcla
automáticamente con una instalación Windows actualizada. La conservación del
estado Linux no elude las comprobaciones ni las actualizaciones de Steam.

## Recuperación del PC

Tras la autorización explícita del usuario se reactivó el prefijo anterior de
Warframe y su caché. Se recuperaron Sniper y su manifiesto completo; el Proton
Experimental activo ya completo se conservó. El prefijo reciente y los datos
parciales quedaron respaldados en
`/juegos/.ultra-recuperacion-linux-20261024/`.
El hash de `EE.cfg` coincide con el de la sesión anterior.

Cyberpunk usaba entornos distintos en Heroic y en recuperación. Heroic pasó a
usar `~/Juegos/prefijos/cyberpunk2077`, donde ya existían sus ajustes gráficos.
Se compararon las partidas de los prefijos sin detectar conflictos; los
prefijos anteriores y las configuraciones previas se conservan. El recorrido
**Heroic → REDlauncher → juego** no se cambia. Se habilitaron temporalmente el
registro detallado y `PROTON_LOG` para la próxima prueba. La causa de la
pantalla negra **sigue sin estar confirmada**, y no se acredita su resolución.

La revisión automática exigió autorización para recuperar los datos y rechazó
reabrir el disco en escritura solo para comprobarlo tras un fallo del montaje
de prueba. Esa comprobación se realizó sobre una instantánea desechable con
la partición física de solo lectura. La configuración pendiente de Heroic se
completó después con respaldo, fsync del archivo y directorio y desmontaje
limpio. Una lectura independiente confirmó la persistencia de la recuperación.

## Validación y entrega

- 13 casos de montajes desechables: renovación, prefijos y runtime conservados,
  origen Windows protegido y rechazo de enlaces/rutas inseguros.
- Registro y búsqueda Steam, VDF y selección de bibliotecas comprobados con
  datos ficticios, sin cuentas ni descargas.
- 152 comprobaciones de arranque y escritorio PC; cero fallos.
- Sistema de A: verificado por SHA-256; kernel idéntico y `datos.ext4` conservado.
- Lectura de prefijo y runtimes como UID1000; revisión ext4 posterior sin errores.

La prueba autenticada de Steam y la pantalla de Cyberpunk requieren el PC real.
No se ejecutaron los juegos ni se hicieron mediciones RTX en QEMU.

[Evidencias sanitizadas](../evidencias/continuidad-juegos/2026-10-24).
