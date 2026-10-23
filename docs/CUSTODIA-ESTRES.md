# Custodia: secuencias adversas, paginas protegidas y mutaciones

2026-10-23, America/Lima. Ampliacion del [prototipo C](CUSTODIA-PROTOTIPO.md),
sin modificar su implementacion ni el kernel. Las nuevas pruebas estan en
`src/guardian/laboratorio/custodia/`; se ejecutan desde campo-custodia.sh.

## Resultado y evidencia

[Evidencias verificadas](../evidencias/custodia/custodia-estres-89a9646f6ca7402f92ea5594322b2d96/SHA256SUMS),
GCC 16.2.1 20260810, kernel del taller 7.2.11-arch1-1 x86-64. Pasan revision
GCC -fanalyzer y compilacion con avisos como errores. Cada binario positivo se
ejecuta con bwrap sin red/home/discos personales, timeout y limites prlimit.
Los perfiles diagnosticos usan ASan/UBSan; no se aplica un cgroup de 512 MiB.

| Perfil | Regresion dirigida | Secuencias: 192000 operaciones | Fronteras |
| --- | --- | --- | --- |
| prod-O2 | 49076 checks, pasa | 532601 checks, pasa | 8574533 checks, pasa |
| prod-O3 | 49076 checks, pasa | 532601 checks, pasa | 8574533 checks, pasa |
| diag-O1 | 49076 checks, pasa | 532601 checks, pasa | 8574533 checks, pasa |
| diag-O2 | 49076 checks, pasa | 532601 checks, pasa | 8574533 checks, pasa |

Son las mismas pruebas bajo cuatro configuraciones. Muchos checks de fronteras
comparan bytes intactos en una pagina; no equivalen a millones de defectos o
clases de UB diferentes. No hubo fallos inesperados ni diagnosticos de memoria
en las ejecuciones positivas. Los controles negativos fallan como se esperaba.
Se verificaron los hashes de 16 fuentes/herramientas antes de guardar evidencia.

## Secuencias con un modelo independiente

`probar-secuencias.c` ejecuta semillas 1..32, 6000 operaciones por semilla y dos
gestores vivos con cuotas diferentes. El generador se fija en el fuente. No
lee metadatos privados de Custodia: mantiene un ledger externo con recursos,
datos esperados y tickets; calcula bytes retenidos y referencias por recorrido,
sin reutilizar los contadores internos del gestor.

Combina prestamos, pedidos, completado, cancelacion, retirada y destruccion;
mezcla tokens vivos, tickets usados como prestamos, referencias antiguas/de otro
gestor y numeros arbitrarios. Incluye subject cero/incorrecto, argumentos NULL,
rangos extremos, canarios de destino y fallos de calloc. Los origenes cambian
despues de prestar para contrastar que el gestor conserva una copia.

No se limita a rechazos: cada perfil registra 14396 prestamos, 4592 begin,
5307 revoke, 2113 complete, 2464 cancel y 13909 destroy aceptados. Tambien se
recrean gestores conservando historial de tokens para probar reutilizaciones
incorrectas. Si falla, informa semilla, paso, operacion y linea; el fuente fijado
permite reproducir la secuencia desde esa semilla y su historial previo.
No es fuzzing guiado por cobertura ni una prueba formal de todos los estados.

## Fronteras de memoria reales

`probar-fronteras.c` usa mmap/mprotect con una pagina accesible entre dos
PROT_NONE, para origen y destino. Prueba tamanos 1..64 y todos sus offsets;
por offset comprueba el resto exacto, un destino de un byte menos y un pedido
de un byte mas. No representa el producto cartesiano de todas las longitudes.

Despues de prestar, la pagina de origen se vuelve inaccesible: completar debe
usar la copia del gestor. Los destinos quedan al final de su pagina, y el caso
rechazado comprueba la pagina completa intacta. Prueba retirada con un ticket
pendiente y espacio de cola disponible, para verificar que no se acepten pedidos
nuevos aunque aun haya capacidad. Un acceso indebido a la guard page haria
fallar el proceso aislado, independientemente del diagnostico del sanitizador.

## Comprobar las pruebas con cuatro mutantes

`scripts/mutantes-custodia.py` escribe solo copias en la salida nueva del taller;
no altera los fuentes originales. Quita por separado controles de:

1. Titular del prestamo.
2. Cuota de bytes.
3. Estado de retirada.
4. Capacidad de cola.

Cada copia se compila con el perfil diagnostico O1 y ejecuta la regresion
dirigida dentro del aislamiento. Los cuatro procesos salen con codigo 1 y
`FAIL line=...`, como se exige. Un fallo de compilacion, timeout o error de
infraestructura no cuenta como mutante detectado. El JSON conserva el control
quitado, numero de sustituciones y hash del fuente derivado; no se guardan sus
binarios ni se despliegan esas copias debilitadas.

Cuatro mutantes detectados no certifican todas las defensas ni la exhaustividad
del banco. Sirven para demostrar sensibilidad de esas aserciones concretas.
La inspeccion HEX sigue pasando con sus negativos, sin atribuirle prueba de
semantica general ni ejecucion de las funciones inspeccionadas.

## Limites y siguiente frontera

No se han probado carreras como comportamiento soportado: el contrato exige
serializacion. Antes de aceptarlas se necesitan sincronizacion y nuevas pruebas.
El subject sigue siendo entrada de un llamador confiable: comprobar un numero
no autentica al proceso. Siguen pendientes IPC, consumidor real, E/S asincrona,
expulsion, prioridades y recuperacion del kernel. Tampoco se ha medido una
ganancia de velocidad. A: y los juegos permanecen sin cambios.

Para reproducir en el taller con la disposicion de fuentes documentada:

```bash
ULTRA_CUSTODIA_OUT=/ruta/nueva/salida bash scripts/campo-custodia.sh
```

La salida debe ser nueva; incluye resultados, inventarios HEX y hashes. Las
pruebas necesitan GCC, glibc, Python, binutils, bwrap y prlimit del taller.
