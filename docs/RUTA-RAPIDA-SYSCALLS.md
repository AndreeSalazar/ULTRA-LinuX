# Ruta rapida de operaciones: amortizar trabajo sin saltar aislamiento

Cuarta categoria: [Custodia de recursos y prestamos](GUARDIAN-CUSTODIA.md).
Define titular, derechos, presupuesto y cierre; su gestor aun no esta implementado.

2026-10-23, America/Lima. Investigacion y diseno; no modifica el filtro,
no habilita io_uring en cajas ni instala un hipervisor. No es una optimizacion
medida o desplegada en juegos. Complementa [CPU/RAM/SSD](CPU-RAM-SSD.md).

## Idea y limite

Preparar recursos autorizados y reutilizarlos puede reducir trabajo repetido.
La meta es menos transiciones, busquedas y copias para la misma tarea. Una
syscall no ejecuta las demas syscalls del catalogo antes de hacer su trabajo;
existen costes de entrada, filtros y subsistemas que hay que medir por separado.

La autorizacion inicial no valida operaciones futuras arbitrarias: cambian
rangos, tamanos, recursos y estados. Los mecanismos elegidos siguen teniendo
contratos y comprobaciones de cada operacion cuando correspondan. Una cola en
memoria compartida tampoco convierte sus entradas en confiables.

Un hipervisor no es requisito para esta ruta y agregarlo no demuestra ahorro.
No se propone pasar juegos a ring 0 ni entregar discos/dispositivos crudos.

## Mecanismos existentes a comparar

| Ruta | Trabajo que puede ahorrar | Lo que permanece / condicion |
| --- | --- | --- |
| vDSO mediante libc | Entrada al kernel para ciertas consultas compatibles, por ejemplo algunos relojes | API limitada, soporte del reloj/arquitectura y fallback. No sustituye read/write/open |
| I/O con buffers y lotes | Muchas operaciones pequenas pueden agruparse | Misma semantica de datos, orden, errores y durabilidad; requiere adaptar el consumidor |
| io_uring con recursos registrados | Referencias a archivos/buffers reutilizadas y envios agrupados | Rangos y operaciones validos, vida util hasta completar, limites de memoria y manejo de errores |
| io_uring SQPOLL | Puede evitar entradas de envio mientras el hilo del kernel esta activo | Consume CPU; despertar el hilo dormido puede requerir syscall. No garantiza menos latencia en carga escasa o CPU saturada |
| mmap donde el contrato lo permita | Acceso posterior mediante memoria en vez de read por fragmento | Faults, sincronizacion, truncado/cambios del archivo y durabilidad. No es una sustitucion universal |

Registrar buffers puede mantener paginas fijadas; no debe ocupar toda la RAM
ni destruir el margen de memoria del escritorio. No basta autorizar
io_uring_setup/io_uring_enter para dar por revisadas todas las operaciones SQE.
Antes de usarlo en una caja hay que disenar restricciones de operaciones,
recursos y credenciales compatibles, y comprobar su interaccion con LSM y la
politica concreta. No se traslada automaticamente la lista de syscalls a una
lista de opcodes io_uring.

## Lo que ya hace ULTRA

Se revisaron `src/caja/caja.c` y `src/caja/seccomp-politica.h`. Los filtros
permiten en el kernel las llamadas declaradas que cumplen sus condiciones;
usan RET_TRACE para las prohibidas, y el supervisor destruye la caja sin
reanudar la operacion infractora. No hay una aprobacion desde userspace por
cada syscall permitida. Esto no afirma coste nulo del filtro ni de ptrace.

Esa caja tiene declaraciones medidas para BusyBox, no para todos los juegos.
Brave/Steam/Proton y bibliotecas tienen sus propios caminos e integraciones.
No se puede cambiar un ajuste de arranque para convertir todas sus lecturas
en io_uring o todas sus llamadas en vDSO. No se amplian permisos por fallos
de compatibilidad sin revisar el contrato.

## Primer experimento propuesto

1. Elegir un consumidor propio de lectura de archivos y una carga pequena
   reproducible. Medir su version actual: tiempo total, CPU, bytes y contenido
   resultante. Contar syscalls en una pasada separada, porque el trazado altera
   el tiempo; un contador de llamadas no demuestra tiempo perdido.
2. Comparar primero agrupacion/buffering manteniendo resultado y errores.
   Solo si el coste lo justifica, anadir io_uring normal con limites de recursos
   y registros explicitos. No empezar con SQPOLL global.
3. Probar tamanos extremos, lectura corta, EOF, archivo truncado, errores,
   cierre/cancelacion y recursos aun en vuelo; mantener memoria viva hasta la
   finalizacion. Los negativos se ejecutan en el taller sin discos personales.
4. Separar correccion y coste: datos iguales, ninguna operacion fuera del
   contrato, tiempos y CPU antes/despues con repeticiones, frio/caliente y
   carga comparable. Un microbenchmark no acredita FPS de Warframe.
5. Conservar fallback explicito cuando el mecanismo no este soportado; una
   restriccion requerida que falte no puede degradarse en silencio. Integrar
   en un consumidor y reconstruir solo despues de revisar evidencia y politica.

Escritura define recursos, limites y vida util; Caza UB verifica el intercambio
y las rutas de error; Fluidez decide si se ahorra trabajo en esa carga.
Estado: investigacion documentada. Experimento, medicion e integracion pendientes.

## Fuentes primarias consultadas

- [vDSO](https://man7.org/linux/man-pages/man7/vdso.7.html).
- [Registro de recursos io_uring](https://man7.org/linux/man-pages/man2/io_uring_register.2.html).
- [SQPOLL](https://man7.org/linux/man-pages/man7/io_uring_sqpoll.7.html).
- [Creacion y opciones de io_uring](https://man7.org/linux/man-pages/man2/io_uring_setup.2.html).
- [Seccomp del kernel](https://kernel.org/doc/html/latest/userspace-api/seccomp_filter.html).

Las paginas actuales orientan el diseno; soporte y semantica deben contrastarse
con el kernel y bibliotecas fijados del consumidor antes de implementarlo.
