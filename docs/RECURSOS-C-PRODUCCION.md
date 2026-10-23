# Lector C de recursos, Custodia y verificacion HEX

2026-10-23, America/Lima. Primer consumidor local de Custodia:
`ultra-recursos-contadores`, invocado por `ultra-medir-recursos`. Observa RAM y
recuperacion de memoria sin privilegios; no intercepta punteros de juegos,
syscalls ni accesos del kernel. Custodia reside ahora en
`src/guardian/produccion`; las entradas de laboratorio incluyen esa unica
implementacion. La identidad 1 pertenece al consumidor local serializado,
no representa autenticacion entre procesos.

## Cambio y limites

Sustituye exclusivamente los bucles Bash de `/proc/meminfo` y `/proc/vmstat`.
Lee hasta 65537 bytes para detectar una captura mayor que el limite de 65536.
Rechaza fuentes vacias, errores de lectura y NUL incrustado antes de publicar
contadores. Custodia copia la captura; el origen se libera y el lector pide
bloques de hasta 1024 bytes mediante tickets. Descarta lineas mayores que 255
bytes, campos no seleccionados, decimales fuera de uint64 y tokens extra.
No publica una captura truncada de un archivo excesivo.

Usa una peticion pendiente. Cada archivo tiene su propio gestor; el ticket
de un byte vive en el slot de Owner, sin una asignacion heap por bloque.
Sigue habiendo asignaciones del gestor y copia del payload. No se atribuye
toda la ganancia a la optimizacion de tickets.

Mantiene el formato MEM/VM para datos normales del kernel. Un fallo es NA;
cero sigue siendo valido. Las rutas son fijas y rechaza argumentos. El banco
genera otro binario con rutas de fixtures: el instalado no acepta rutas ni
modos de aplicar ajustes. El snapshot entre campos sigue sin ser atomico.

## Verificacion y medida

- Produccion O2/O3 con trampas UB y static PIE; diagnostico O1/O2 con
  ASan/UBSan. Revision GCC con analyzer O0 y warnings O2.
- 2048 posiciones entre bloques, lineas largas, archivo al limite y excesivo,
  ultima linea sin salto, NUL, ausencia, cero, UINT64_MAX y overflow.
- Fallo de cada una de las cinco asignaciones: captura, gestor, dos pools y
  payload. Salida vacia y sin fugas diagnosticadas por los sanitizadores.
- Fixtures de inventario y sesion: fuentes intactas, AMD/Intel, limites de
  trabajo y ausencia distinguida de cero.
- Custodia por perfil: 50076 checks dirigidos, 192000 operaciones contra
  modelo independiente y 8574533 checks de fronteras. Detectados cuatro
  mutantes eliminando controles de titular, cuotas, retirada y cola.
- HEX: cinco simbolos del lector en cuatro perfiles y seis del banco de
  Custodia. Los bytes de objdump se contrastan con la tabla de simbolos y
  secciones ELF; quedan desensamblado y hashes. Es inspeccion pasiva,
  complementada con pruebas de ejecucion; no una prueba formal de todo
  comportamiento ni una traza de todas las instrucciones ejecutadas.

Comparacion del consumidor completo: doce pares alternados, veinte procesos
por ruta y par; captura MEM/VM identica y salida contrastada byte por byte.
En QEMU: mediana por captura **7.230740 ms Bash frente a 0.513745 ms C**.
Mediana de reducciones pareadas: **92.93 %**. Incluye arranque de proceso;
se conservan muestras de tiempo CPU y faults. Es menor coste del observador,
no aceleracion de RAM, SSD ni juegos. No describe p99 por fotograma.

La regresion O2 de crear/destruir continuamente del microbenchmark anterior
sigue en [Custodia rendimiento](CUSTODIA-RENDIMIENTO.md). Aqui se evalua el
consumidor completo frente al lector que sustituye; no justifica integrar
Custodia en cualquier app sin medirla.

### Un ejemplo concreto en HEX

En el banco prod-O2, ELF SHA256
`4dbd6f03f5a667046a9e78352c1c82772d8ec95f78ed6c168ba0fe84d7fbefb0`,
Owner conserva la bifurcacion que se queria optimizar:

| Direccion de enlace | Bytes | Lectura del desensamblado |
| --- | --- | --- |
| B82E | `49 83 F9 01` | Compara el tamano con 1 |
| B832 | `75 6C` | Si difiere, salta a la ruta general B8A0 |
| B834 | `48 8D 44 07 14` | Calcula la direccion del byte dentro del slot |
| B8C5 | `E8 F6 F0 FF FF` | La ruta general llama a __wrap_calloc del banco |

El salto separa el byte interno del asignador general; el conteo de
asignaciones y los tests aportan la comprobacion dinamica complementaria.
Estas direcciones pertenecen a ese ELF de prueba con inyeccion de fallos;
no son offsets de archivo ni direcciones runtime de las imagenes con ASLR.
O3/LTO puede reordenar o integrar las funciones. No se exige que esos bytes
se repitan en todos los perfiles. `0F 0B` identifica UD2 en las trampas UB
del binario, pero encontrar esa instruccion no acredita que todo UB tenga
un detector. Lo mismo ocurre con ENDBR64 y la disponibilidad real de CET.

## RAM y SSD fisicos

El informe previo tiene 39 muestras: usada estimada entre 693.71 y 2720.05
MiB, mediana 2493.79 MiB; swap 0. No contiene RECURSOS v1, PSI de memoria ni
latencia de almacenamiento. No permite decidir THP, reclaim o colas optimos.
El dmesg no contiene las seis cadenas de error seleccionadas en el resumen;
eso no prueba ausencia general de fallos.

Windows informa dos modulos de 8 GiB, tipo SMBIOS 26 y velocidad configurada
3200; un Kingston NV2 NVMe de 1 TB y SATA A400 de 120/480 GB. Linux registra
64 MiB de host memory buffer para NVMe y 8/0/0 colas default/read/poll.
Estos inventarios no miden ancho de banda ni salud exhaustiva.

La integracion conserva PSI, VM faults/reclaim/refaults/compactacion, dirty y
writeback, politicas CPU y estadisticas de bloque por capa. No modifica
timings de RAM, ticks, frecuencia, scheduler de SSD, flush, barreras o caches.
Falta recoger una sesion nativa con estos contadores, correlacionarla con
tiempos por fotograma y comparar cada ajuste relacionado con el cuello
de botella observado. QEMU no sustituye esa medida fisica.

## Reproduccion y evidencias

`scripts/verificar-recursos-c.sh` requiere ULTRA_RECURSOS_OUT nuevo.
`scripts/comparar-recursos.py` recibe el helper Bash anterior y un directorio
nuevo. Referencia: `6840526:src/rootfs/bin/ultra-medir-recursos`.

[Evidencias de esta ronda](../evidencias/recursos/recursos-prod-0983bec7bad24b89bcef0b34770f3ff8/)
separan pruebas del lector, Custodia, benchmark, resumen fisico,
auditoria del binario final y pruebas de arranque/despliegue.

## Despliegue comprobado

Generico construccion #51 y PC #37, ambos instalados en A:. El arranque
descartable con escritorio/Brave y persistencia pasa 152 comprobaciones en
generico y 153 en PC, cero fallos. Incluye el lector con UID 1000 y sin
capacidades, y el inventario completo. Cada imagen audita seis ejecutables
propios; la validacion de descarga pasa ademas siete controles negativos.
Los hashes de sus fuentes actuales y lectores finales se contrastaron con
la auditoria y el HEX. En PC se inspecciona main del ELF final con O3/LTO;
los simbolos internos pueden haberse integrado por el compilador.

La copia final de kernel y EROFS coincide con los artefactos probados.
`datos.ext4` conserva SHA256
`e0b263e1d1b953d6e1677fac5b26add145c899d7f10455529cf8ebfb81b42d2f`.
Informes y respaldo de datos quedan en el proyecto Windows; las sesiones
privadas no entran en Git. El archivo de juegos de C: no se modifica.
El taller se apago al terminar. La prueba NVIDIA de QEMU comprueba carga de
componentes y rechazo por GPU ausente, no ejecucion sobre la RTX fisica.
