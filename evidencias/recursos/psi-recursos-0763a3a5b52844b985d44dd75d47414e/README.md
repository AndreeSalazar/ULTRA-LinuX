# PSI real y analisis de recursos: 2026-10-23

Referencia anterior: 9dd71f3. Se encontro CONFIG_PSI desactivado en generico
#51 y PC #37. La ronda habilita PSI en #52/#38, comprueba que su contador CPU
aumenta bajo carga acotada sin privilegios y verifica captura/analisis real.

- resultados/: lector C en O2/O3 y ASan/UBSan O1/O2, fronteras, asignaciones
  fallidas, rechazo de argumentos y error de salida diferida /dev/full.
- regresion-salida.json: antes devolvia 0, ahora devuelve 1; sin escrituras
  a discos personales. El archivo vacio salida-fallida.txt no representa
  ausencia de prueba: el resultado se comprueba en el runner/informe.txt.
- auditoria-*/: ELF del lector, HEX de main integrado y manifiesto de fuentes.
  Fuentes y binario se contrastaron con archivos actuales e imagen descargada.
- configuracion-*.json y compilacion-*.txt: configuracion PSI y construccion.
- arranque-*.log/resultados.txt: generico 156 controles y PC 157, cero fallos.
  Incluyen guardianes, persistencia, escritorio/Brave y PSI bajo carga CPU.
- generico/ y pc/: informes originales del invitado descartable; no contienen
  sesiones personales. qemu-*-resumen.json es la salida del analizador.
- pruebas-intervalos.txt/pruebas-resumen.txt: deltas conocidos, reinicios,
  formatos y unidades, valores ausentes/duplicados, CRLF y limites de entrada.
- validacion-*.txt: seis ejecutables auditados y nueve rechazos, incluidos
  informe HEX de otro binario o sin contraste de bytes ELF.
- instalacion-*.json/txt y despliegue-resumen.json: copia A: verificada por
  hash, conservacion de datos.ext4 y respaldo de informes en Windows.
- respaldo-anterior.json: la imagen dual pre-PSI permanece en build/, fuera
  de Git, con su hash de referencia. No se copian discos privados a Git.

El banco expone un vCPU/2 GiB y renderiza por software; CPU/memoria/E/S
pueden tener presion a la vez. No se suman porcentajes ni capas de disco.
No se atribuyen sus tasas al SSD fisico, Warframe o Brave particulares.
CPU full del sistema se deja null. Los major faults pueden ser paginacion
normal y no significan UB. Falta la captura nativa de juego con tiempos por
fotograma; PSI no acredita una mejora de FPS ni coste nativo nulo.

Las capturas previas sin PSI y el resumen fisico anterior se conservan por
separado en ../intervalos-2026-10-23/. Las pruebas QEMU no usan discos fisicos,
cuentas autenticadas ni archivos reales de juegos. El taller queda apagado.
