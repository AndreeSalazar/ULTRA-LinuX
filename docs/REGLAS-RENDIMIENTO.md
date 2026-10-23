# Reglas pendientes para demostrar rendimiento

2026-10-23, America/Lima. Investigacion y criterios de aceptacion. La
[ronda agresiva de Custodia](CUSTODIA-ESTRES.md) ya esta documentada y conserva
sus evidencias. Sus checks no miden velocidad. Esta ronda no modifica el gestor,
parametros de kernel, prioridades ni la imagen de A:.

## Hallazgos del codigo actual

En `src/guardian/laboratorio/custodia/ultra_custodia.c`, uc_begin llama a
ultra_owner_alloc para crear un ticket con payload de un byte. Owner ejecuta
calloc para cada asignacion aceptada. Retirar el ticket llama a owner_free,
que libera ese objeto. Es una asignacion/liberacion por pedido aceptado,
ademas de la metadata fija del gestor y los costes del asignador.

uc_lend asigna y copia el payload; uc_complete copia el rango al destino.
Find_loan/find_request recorren hasta 16 entradas. Son candidatos de coste,
no cuellos de botella medidos. No existen lecturas de SSD, io_uring ni
autenticacion de procesos en este modelo; sus tiempos futuros no pueden
atribuirse a esta implementacion en RAM.

## Reglas y estado

| ID | Prioridad | Regla | Estado actual y evidencia necesaria |
| --- | --- | --- | --- |
| RF-01 | Antes de comparar | Mismo contrato y resultados | Pruebas de correccion del modelo pasan. Cada candidato debe conservar copia estable, limites, retirada, cuotas y rechazo de tickets caducados; comparar bytes y errores |
| RF-02 | Primero | Referencia reproducible | Pendiente benchmark de Custodia: fijar commit, binario/flags, CPU/kernel, carga, tamaños y condiciones; guardar datos crudos |
| RF-03 | Primero | Medir el ciclo completo y sus etapas | Pendiente: separar crear/prestar, begin, completar/cancelar, retirar y destruir; declarar que parte incluye el total |
| RF-04 | Primero | Reducir variabilidad y medir la instrumentacion | Pendiente: repeticiones, orden alternado/pareado, calentamiento separado y coste de reloj/observador; registro de temperatura, politica CPU y carga disponible |
| RF-05 | Antes de optimizar tickets | Menos asignaciones sin reutilizar autoridad vieja | Candidato pendiente: almacenamiento de tickets reutilizable con generacion/identidad correctas. No basta guardar y devolver el mismo handle |
| RF-06 | Antes de eliminar copias | Mantener propiedad y vida util | Copias actuales verificadas, coste pendiente. Quitar la copia inicial rompe el contrato probado con origen inaccesible; requiere otro contrato y pruebas |
| RF-07 | Antes de atribuir un limite al equipo | CPU/RAM/E/S observables | Snapshot e inventario probados en taller, despliegue nativo pendiente. Correlacionar PSI, reclaim, faults y E/S con la carga; ausencia es NA |
| RF-08 | Antes de priorizar o agrupar | Cola y presupuesto con contrapresion | Quotas locales probadas. Falta evaluar latencia vs caudal en colas 1/4/16, justicia entre clientes y limite real de recursos del proceso |
| RF-09 | Antes de adoptar io_uring/SQPOLL | Consumidor compatible y beneficio medido | Pendientes E/S real, restricciones y coste de CPU. SQPOLL no se activa globalmente ni se supone mas rapido por reducir entradas de envio |
| RF-10 | Antes de medir carreras como soportadas | Sincronizacion con contrato de progreso | Pendiente diseno concurrente; el actual es serializado. No confundir un test TSan limpio con prueba de todas las intercalaciones |
| RF-11 | Antes de promover | Beneficio relevante y coste aceptable | Pendiente: objetivo de latencia/CPU/memoria fijado por consumidor, variabilidad reportada y regresiones completas; resultado inconcluso si no se distingue del ruido |
| RF-12 | Antes de declarar mejora del escritorio/juego | Integracion y prueba nativa representativa | Pendientes identidad autenticada, consumidor, imagen/arranque y carga real. Un microbenchmark no acredita FPS ni ausencia de tirones |

Las reglas no asignan porcentajes globales. Un cambio puede estar probado en
laboratorio y mantener rendimiento o integracion pendientes. La falta de
autenticacion no impide medir el modelo local; si impide presentarlo como
frontera de seguridad entre procesos.

## Primera medida propuesta, aun no ejecutada

1. Instrumentar la version actual sin cambiar su algoritmo: contar asignaciones,
   bytes copiados y operaciones; separar esa pasada diagnostica de las mediciones
   de tiempo. No llamar a un contador de syscalls un medidor de tiempo perdido.
2. Usar una carga determinista con payload inmutable y validar todos los bytes;
   variar tamaños (16 B, 512 B, 4 KiB, 64 KiB, 1 MiB) y pedidos pendientes
   (1/4/16), con limites suficientes y memoria acotada. Es un diseño de prueba,
   no una afirmacion de que represente ya los accesos de Warframe.
3. Registrar tiempo transcurrido con reloj monotono y tiempo CPU del proceso.
   Declarar la resolucion y el coste del reloj. Agrupar operaciones diminutas
   puede reducir el coste de medir, pero percentiles de lotes no equivalen a
   percentiles por operacion ni por fotograma.
4. Conservar muestras y numero de repeticiones, mediana, p95/p99 cuando la
   cantidad y granularidad lo permitan, maximo, CPU, RSS y variabilidad. No
   publicar una cola p99 fiable a partir de unas pocas repeticiones.
5. Proponer despues un cambio de tickets, repetir las pruebas adversas y
   comparar con la referencia bajo el mismo contrato y perfil de produccion.
   Mantener la version anterior como vuelta atras. Medir ambos candidatos bajo
   el mismo aislamiento; QEMU solo sirve aqui como entorno de laboratorio.

No se comparan tiempos ASan con tiempos sin ASan como ganancia del algoritmo.
Tampoco se cuenta como optimizacion terminar sin esperar operaciones en vuelo
o mover su coste fuera del intervalo declarado. No se eliminan comprobaciones
para lograr una cifra favorable.

## Reglas del sistema que acompañan la medida

- CPU: driver/governor/EPP reales, frecuencia efectiva si se puede medir y
  temperatura. El soporte AMD P-state depende de CPU/firmware; no fijar
  frecuencia maxima para todos los equipos del perfil generico.
- Memoria: distinguir heap, page cache y tmpfs; medir presion y refaults antes
  de probar THP, compresion o retencion de buffers. THP puede reducir coste de
  traduccion y tambien cambiar pausas y consumo; no es una mejora universal.
- E/S: identificar capas y soporte; controlar profundidad de cola y tamaño de
  lotes. Un lote mayor puede favorecer caudal y empeorar espera interactiva.
- Cuotas: max_bytes del modelo no incluye todo RSS. Para repartir recursos
  entre procesos, comprobar disponibilidad y delegacion de controladores
  cgroup v2; una cuota demasiado estrecha puede introducir throttling/reclaim.
  No hay un nuevo cgroup aplicado en esta ronda.
- Monitorizacion: PSI cuantifica esperas por recursos, no acusa a un programa
  de UB. CPU full a nivel de sistema se publica como cero por compatibilidad;
  no debe usarse como prueba de ausencia de espera de CPU.
- Medidas de proceso: getrusage ofrece CPU, faults y cambios de contexto. RSS
  maximo es un pico acumulado, no memoria exclusiva por etapa; inblock/oublock
  cuentan operaciones, no bytes. Los campos no mantenidos no prueban cero uso.

## Fuentes primarias consultadas

- [Relojes y resolucion](https://man7.org/linux/man-pages/man2/clock_gettime.2.html).
- [Uso de recursos de proceso](https://man7.org/linux/man-pages/man2/getrusage.2.html).
- [PSI y semantica de CPU full](https://docs.kernel.org/accounting/psi.html).
- [AMD P-state](https://docs.kernel.org/admin-guide/pm/amd-pstate.html).
- [THP y sus compromisos](https://docs.kernel.org/admin-guide/mm/transhuge.html).
- [Controladores cgroup v2](https://docs.kernel.org/admin-guide/cgroup-v2.html).
- [SQPOLL y casos donde puede perjudicar](https://man7.org/linux/man-pages/man7/io_uring_sqpoll.7.html).

Las reglas de aceptacion y el diseño del benchmark son decisiones propuestas
para ULTRA, no limites universales prescritos por esas fuentes. La disponibilidad
de interfaces se contrasta con el kernel fijado antes de usarlas.
