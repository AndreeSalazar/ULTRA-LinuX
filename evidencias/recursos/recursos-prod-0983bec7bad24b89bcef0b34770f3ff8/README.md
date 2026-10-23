# Lector C/Custodia, HEX y recursos: 2026-10-23

Referencia anterior: commit 6840526. Fuentes finales y sus hashes exactos en
fuentes-ronda.json; el texto de Git puede normalizar finales de linea.

- resultados-v2/: cuatro perfiles del lector, errores de asignacion, fixtures,
  HEX y hashes. Los errores de asignacion se fuerzan en ejecutables de prueba.
- custodia/: secuencias, paginas protegidas, cuatro mutantes y HEX de seis
  funciones por perfil. Los fallos de mutantes son controles esperados.
- benchmark/: doce pares, veinte procesos por ruta; datos proc inmutables,
  script Bash anterior y muestras de tiempo/CPU/faults. La reduccion de 92.93 %
  corresponde al lector MEM/VM en QEMU, no al sistema completo ni a juegos.
- hex-produccion-*/ y elf-final-*.json: main de los lectores finales y
  auditorias estructurales. Sus SHA256 coinciden con los manifiestos descargados.
- fuentes-produccion-*.sha256: fuentes que compilo cada imagen; contrastadas
  con los archivos actuales del proyecto antes del despliegue.
- informe-fisico-resumen.json: contadores seleccionados de los informes A:,
  con hashes del origen. Ausencia de una cadena no demuestra ausencia de fallos.
- compilacion-*.txt, arranque-*.log, arranque-*-resultados.txt y validacion-*:
  construccion, ejecucion de cada perfil y rechazo de artefactos alterados.
- instalacion-*.json: hashes de EFI/EROFS copiados y conservacion de datos.ext4.

Los ejecutables, imagenes de disco, respaldos y las sesiones personales no
se guardan en Git. El taller usa su propio disco QEMU; las pruebas de arranque
usan copias descartables. El respaldo principal permanece en Windows.
No se han medido latencias de RAM/SSD fisicos ni tiempos por fotograma con
esta imagen. La observacion nueva queda lista para la siguiente sesion nativa.
