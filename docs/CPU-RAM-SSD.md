# CPU, RAM y SSD: diagnostico dirigido y fluidez

Investigacion complementaria: [ruta rapida de operaciones y syscalls](RUTA-RAPIDA-SYSCALLS.md).

2026-10-23, America/Lima. Alcance de esta ronda: investigar, exponer metricas y
preparar comparaciones. No altera GPU, frecuencias, voltajes, swap, colas de
disco ni parametros de memoria. No es una nueva build instalada en A:.

## Equipo y evidencia disponibles

Consulta local de Windows sin numeros de serie: Ryzen 5 5600X, 6 nucleos y
12 hilos; dos modulos de 8589934592 bytes, SMBIOSMemoryType=26 (DDR4), Speed y
ConfiguredClockSpeed reportados como 3200, DataWidth=TotalWidth=64. Son datos
del firmware via WMI, no una medida de ancho de banda ni confirmacion de ECC
activo, doble canal o estabilidad de un perfil de memoria. No confundir la
tasa DDR con el reloj fisico de los chips.

El informe fisico de A: del 22/23 de octubre muestra MemTotal=16303376 kB y
SwapTotal=0 en las muestras consultadas. PC-configuracion identifica NVMe y
SATA/AHCI; no proporciona latencias de E/S ni el tipo fisico de cada disco.
Se conserva la distincion entre esas observaciones y el taller virtual.

## Memoria fisica: que se puede abstraer

| Familia o propiedad | Control relevante | Aplicacion a este PC |
| --- | --- | --- |
| DRAM DDR y LPDDR, generaciones distintas | Controlador/firmware establecen parametros; Linux administra paginas | DDR4 reportada. No se cambia voltaje ni timings ni se convierte en DDR5 por software |
| Cache de CPU, normalmente SRAM | Localidad, disposicion de datos, evitar compartir lineas escritas entre hilos | Revisar codigo propio y medir fallos de cache si hay soporte; no es RAM libre para un pool |
| ECC | Correccion en hardware y reporte EDAC cuando plataforma/driver lo soportan | No confirmada. Un guardian de punteros no sustituye ECC ni diagnostica todos los fallos fisicos |
| NUMA y memoria por niveles, incluidos sistemas CXL | Topologia, afinidad y politica de ubicacion de paginas | Catalogo para otros equipos; no asumir hardware presente ni activar ajustes de servidor |
| Memoria de dispositivos, por ejemplo HBM/VRAM | Contratos del dispositivo y sus controladores | Fuera de esta ronda; no equivale automaticamente a RAM del sistema |

No es un inventario de cada estandar historico de RAM. La abstraccion util
para los guardianes es propietario, region, capacidad, vida util, ubicacion y
coste de acceso; estas propiedades deben comprobarse o marcarse desconocidas.
No se expone memoria fisica cruda ni se propone /dev/mem como interfaz.

## Usos de RAM en Linux: evitar sumar categorias solapadas

| Uso | Que observar | Hipotesis que se puede probar |
| --- | --- | --- |
| Anonima: heap, pila y mmap anonimo | AnonPages, PSS por proceso, faults, presion y swap | Reducir asignaciones/copias, reutilizar buffers acotados y revisar vida util |
| Archivos y page cache | Cached, Dirty, Writeback, refaults y E/S | Mantener datos reutilizados; reducir relecturas y escrituras innecesarias |
| Compartida/tmpfs y capa writable en RAM | Shmem y montajes del consumidor | Controlar crecimiento; tmpfs ocupa memoria, no almacenamiento gratis |
| Kernel | Slab, SReclaimable/SUnreclaim, tablas de paginas y pilas | Detectar crecimiento sostenido antes de atribuirlo a apps |
| Fijada/no desalojable | Unevictable y Mlocked | Evitar fijar toda la RAM o impedir que el kernel recupere memoria |
| Paginas grandes THP/mTHP/hugetlb | Politica activa, AnonHugePages, fallback y compactacion | Probar por consumidor; menor coste de traduccion puede competir con pausas de compactacion |
| Swap comprimida | zram: mm_stat/io_stat; zswap: politica y estadisticas disponibles | Intercambiar CPU y RAM por menos E/S solo si la carga lo justifica |

Shmem, cache, RSS y memoria compartida no forman una particion sumable simple.
MemAvailable es una estimacion; RAM sin ocupar no es por si sola una meta.
zram es un dispositivo comprimido en RAM; zswap es una cache comprimida para
paginas de swap. No se activan ambos ni se inventa un tamano optimo en esta
ronda. SwapTotal=0 no prueba un problema mientras no exista presion, ni prueba
que nunca vaya a existir. No se crean nuevas reservas en discos de Windows.

## Observador implementado

`src/rootfs/bin/ultra-medir-recursos` obtiene una instantanea de solo lectura,
sin argumentos para contadores, o con `--inventario` para identificacion;
no usa herramientas privilegiadas. `ultra-medir-sesion` lo consulta
como usuario ultra con su timeout existente de dos segundos por llamada; el
constructor lo incluye en la proxima imagen. Se mantiene el limite de 120
muestras y pausa minima de cinco segundos del observador de sesion.

- PSI de CPU/memoria/E/S, si el kernel lo ofrece; falta de interfaz es NA.
- Contadores de faults, reclaim, refaults, swap, compactacion y OOM disponibles.
- Desglose de meminfo y politicas THP, writeback, swappiness y zswap.
- Hasta 16 politicas cpufreq: driver, governor, EPP y frecuencias en kHz.
  scaling_cur_freq puede describir la frecuencia solicitada segun el driver;
  no se presenta como frecuencia efectiva medida.
- Hasta 32 dispositivos de bloque: scheduler, read-ahead, tamanos de bloque,
  soporte de discard y contadores stat; zram agrega mm_stat/io_stat.

Los contadores son acumulados: necesitan deltas y un intervalo real antes de
calcular tasas. Las capas loop/dm/disco no se suman como trafico independiente.
rotational=0 no identifica por si solo un SSD. stat no ofrece percentiles de
latencia; estos requieren una captura adicional. Un snapshot no es atomico.
Los limites 16/32 se publican y pueden omitir dispositivos/politicas; no se
afirma cobertura total. El observador no es un detector de UB ni un benchmark.

Puede ejecutarse en el sistema de usuario como:

```bash
/usr/local/libexec/ultra-medir-recursos
```

Validacion de esta ronda: [evidencias y hashes](../evidencias/recursos/recursos-f029b95855544ce6868d9473dacfd1c8/SHA256SUMS).
Pasan sintaxis Bash del observador, sesion y constructor; fixtures de recursos
con fuentes intactas, cero frente a ausencia, contadores y limites; regresion
de sesion e integracion del snapshot sobre el mismo procfs de prueba. Una
consulta real de solo lectura como builder finaliza dentro de dos segundos en
QEMU. El inventario Windows sin seriales se conserva aparte del snapshot de
la VM: su RAM virtual de unos 4 GiB no representa los 16 GiB fisicos del PC.
Esto no es una prueba completa de reconstruccion o arranque de ULTRA.

No aparece en una imagen anterior hasta reconstruirla y desplegarla. Aun debe
medirse su coste y validarse el informe integrado en arranque nativo; no se
declara monitorizacion permanente ni ganancia de FPS por agregar lecturas.

## Deteccion adaptable en el perfil generico

El constructor instala el mismo observador en generico y pc. El generico no
usa las banderas znver3 del perfil pc; la identificacion lee el hardware donde
se ejecuta, no reutiliza la consulta Windows ni presume Ryzen. Esta ronda no
modifica el kernel ni su seleccion de drivers.

La sesion obtiene un inventario una sola vez antes de muestrear. Incluye
release del kernel, vendor/modelo/familia de la primera CPU logica, CPUs online,
nodos NUMA online y hasta ocho indices de cache de cpu0. Publica hasta 32
dispositivos de bloque con vendor/modelo si son legibles, capacidad en sectores
de 512 bytes, estado de solo lectura y hasta 16 dispositivos inferiores por
capa. Los campos de serie, UUID de firmware y hostname no se recogen.

Las familias derivadas del nombre de dispositivo son solo orientativas. Un
loop, zram o dispositivo virtual no se convierte en SSD porque rotational=0.
El tipo DDR, ECC y canales no se deducen de meminfo: quedan desconocidos en
este inventario sin una fuente adicional fiable. Los limites publicados pueden
omitir hardware en equipos mayores o topologias heterogeneas.

Al terminar, el informe conserva las capacidades y politicas observadas, no
activa un ajuste al reconocer una marca. La deteccion es la entrada de Fluidez:
primero medir presion y deltas bajo la carga real, despues comparar un ajuste
compatible. No se afirma un cuello de botella ni una ganancia a partir del
modelo de un componente.

La deteccion se integra en el codigo de la proxima imagen generica; falta
reconstruir/desplegar esa imagen y validarla en el PC. El taller verifica los
casos de inventario y su integracion, no reemplaza ese arranque.

[Evidencia de deteccion generica](../evidencias/recursos/recursos-generico-b06a2c8b0ea74969b55f3e4e462e7721/SHA256SUMS):
pasan fixtures AMD e Intel, identidad de disco, capas acotadas, fuentes intactas,
inventario unico por sesion y parada previa sin inventario. Tambien pasan la
regresion de sesion, sintaxis y una consulta real en el taller dentro del timeout.
Los datos de QEMU describen la CPU y dispositivos expuestos al invitado: un
nombre Ryzen no convierte sus cuatro CPUs virtuales en la topologia fisica del
host. Cero indices de cache mostrados significa que no se recogieron indices,
no demuestra que el procesador carezca de cache.

## Orden para optimizar con los tres guardianes

1. **CPU:** confirmar driver/EPP reales y temperatura; medir espera del
   planificador y tiempos por fotograma. Comparar politicas compatibles una a
   una, sin fijar frecuencia maxima o desactivar ahorro/mitigaciones globalmente.
2. **RAM:** correlacionar tirones con PSI, major faults, reclaim, refaults y
   compactacion. Primero corregir crecimiento o copias evitables en codigo
   propio; despues evaluar THP o compresion con referencia y vuelta atras.
3. **SSD:** identificar dispositivo y todas las capas del archivo del juego;
   correlacionar presion E/S y dirty/writeback con picos. Evaluar scheduler,
   read-ahead y mantenimiento TRIM solo si soportados y despues de medir.
   No cambiar cache flush, barreras ni montar Windows en escritura para ganar
   una cifra de rendimiento. No ejecutar cargas destructivas o pruebas de
   desgaste como diagnostico inicial.
4. **Quirurgico bajo demanda:** activar trazado dirigido o DAMON si el kernel
   exacto lo soporta; limitar duracion, memoria y ambito. Diagnostico intensivo
   separado de la referencia de juego. Menos muestras no significa menos rigor
   si responden a una hipotesis concreta.

No vaciar caches periodicamente: elimina reutilizacion y puede degradar el
rendimiento. No aplicar un paquete universal de sysctls. Para cada cambio,
Escritura comprueba el contrato; Caza UB verifica regresiones; Fluidez compara
p95/p99, picos, consumo y variabilidad en la misma carga, fria y caliente.
CPU, RAM y SSD son el foco; una espera puede seguir viniendo de GPU o red aunque
esta ronda no las modifique.

## Fuentes primarias

- [Conceptos de memoria Linux](https://docs.kernel.org/admin-guide/mm/concepts.html).
- [PSI: espera por recursos](https://docs.kernel.org/accounting/psi.html).
- [THP y compactacion](https://docs.kernel.org/admin-guide/mm/transhuge.html).
- [zram](https://docs.kernel.org/admin-guide/blockdev/zram.html) y
  [zswap](https://docs.kernel.org/admin-guide/mm/zswap.html).
- [AMD P-state](https://docs.kernel.org/admin-guide/pm/amd-pstate.html).
- [Estadisticas de bloque](https://docs.kernel.org/block/stat.html) e
  [interfaces sysfs estables](https://docs.kernel.org/admin-guide/abi-stable.html).
- [DAMON](https://docs.kernel.org/admin-guide/mm/damon/) y
  [EDAC](https://docs.kernel.org/driver-api/edac.html).
- [Parametros de memoria, incluido drop_caches](https://www.kernel.org/doc/html/next/admin-guide/sysctl/vm.html).
- [Campos WMI de memoria fisica](https://learn.microsoft.com/en-us/windows/win32/cimwin32prov/win32-physicalmemory).

La documentacion latest/next orienta la investigacion; disponibilidad y
semantica exactas se contrastan con el kernel fijado antes de aplicar ajustes.
