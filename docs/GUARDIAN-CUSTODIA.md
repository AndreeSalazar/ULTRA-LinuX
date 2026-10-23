# Cuarta categoria: Custodia de recursos y prestamos

2026-10-23, America/Lima. Definicion de arquitectura, sin servicio nuevo,
ABI de kernel, politica de prioridad o cambio de permisos instalado.
Complementa [las categorias de guardianes](CATEGORIAS-GUARDIANES.md) y
[las rutas rapidas](RUTA-RAPIDA-SYSCALLS.md).

Primer avance: [modelo C ejecutable y HEX](CUSTODIA-PROTOTIPO.md), serializado
y con copia en RAM. Todavia sin identidad autenticada ni autoridad del kernel.

## Objetivo

Prestar recursos acotados mediante referencias controladas; autorizar su uso,
mantenerlos vivos hasta terminar y retirar el acceso cuando corresponda.
La ruta rapida reutiliza trabajo de preparacion y puede agrupar pedidos.
No pretende que Linux comprenda la intencion completa de cualquier programa.
La analogia de consola sirve como idea de interfaz, no como afirmacion de como
funciona un sistema propietario de Nintendo.

El kernel ya mantiene referencias y derechos para recursos como archivos
abiertos. Un protocolo propio debe integrarse con esos mecanismos o con un
gestor que los haga cumplir. Cambiar un puntero por un entero no crea por si
solo una autorizacion del kernel.

## Contrato del prestamo

| Campo | Regla |
| --- | --- |
| Identidad | Recurso, gestor y generacion; no aceptar referencia ajena o retirada |
| Titular | Identidad del cliente verificada por el gestor; no confiar en el PID declarado por el cliente |
| Derechos | Operaciones explicitas; prestamo de lectura no autoriza escritura, ejecucion ni delegacion |
| Alcance | Regiones y tamanos, archivos/dispositivos permitidos y dependencias |
| Presupuesto | Memoria fijada, bytes, cola y operaciones en vuelo; contabilidad antes de conceder |
| Vida util | Estado y condiciones de cierre/revocacion; referencias retenidas hasta finalizar trabajo pendiente |
| Servicio | Prioridad acotada y contrapresion; conservar capacidad para escritorio y otras tareas |
| Evidencia | Motivo del rechazo, estado de la peticion y version de politica; no afirmar finalizacion desconocida |

El handle sera opaco al consumidor, pero no se basa solo en que sea dificil
adivinarlo: el gestor verifica titular, derechos y estado. Transferirlo entre
clientes requiere una operacion de delegacion definida. No se reutiliza la
API `ultra_owner` actual como capacidad de seguridad: sus tokens son locales
al proceso, sin proteccion ante codigo hostil dentro de ese proceso, y no
los conoce el kernel como prestamos autorizados.

## Ciclo de vida propuesto

1. **Solicitado:** cliente pide un recurso, derechos y presupuesto.
2. **Activo:** gestor autoriza y entrega referencia; cada pedido comprueba
   identidad, derechos, rangos, cuotas y estado segun el contrato.
3. **Retirando:** se rechazan pedidos nuevos. Se cancelan los cancelables y
   se espera o contabiliza el resto; una cancelacion solicitada no significa
   que el acceso ya termino.
4. **Cerrado:** solo despues de resolver las referencias en vuelo se liberan
   buffers y cargos. Una referencia antigua se rechaza.

Un timeout no prueba que una operacion de E/S se haya detenido. Tampoco se
promete deshacer escrituras ya confirmadas. Si hubo un mmap, retirar el handle
no revoca automaticamente las paginas ya mapeadas: el contrato del mecanismo
debe resolverlo, o ese tipo de prestamo queda fuera del primer prototipo.

## Priorizar recursos pesados

Ser pesado no basta para recibir mas prioridad: una tarea por lotes puede
ser pesada y tolerar espera. Se distinguen latencia interactiva y caudal de
fondo; limites de cola/bytes evitan que una sola tarea monopolice la ruta.
Se evaluan controles del subsistema y cgroup disponibles antes de inventar un
planificador. No se concede prioridad de tiempo real a todos los juegos.

Primera seleccion: lectura de archivos acotada en un consumidor propio,
buffering/lotes antes de io_uring normal con recursos registrados. SQPOLL queda
condicionado a medir beneficio frente a su coste de CPU. vDSO se conserva para
consultas compatibles de libc, sin convertirlo en servicio de prestamos.
Se descartan de esta etapa el bypass universal, el hipervisor adicional y la
ejecucion de apps en ring 0. No se borran alternativas del catalogo historico.

## Rechazo, expulsion y fallo del kernel

| Situacion | Respuesta prevista | Lo que no se promete |
| --- | --- | --- |
| Pedido fuera de derechos, rango o cuota | Rechazar antes de aceptarlo; sin cambios parciales donde el contrato lo garantice | Que todo rechazo sea prueba de ataque |
| Violacion confirmada de la politica de una caja | Retirar prestamos nuevos y terminar la caja conforme a su politica, con identidad estable del proceso/grupo | Que matar un PID aislado elimine hijos o trabajo de kernel pendiente |
| EOF, fallo de disco o timeout ordinario | Resultado de error definido; recuperar/cerrar segun el contrato | Expulsar automaticamente por cualquier fallo de E/S |
| Corrupcion detectada dentro del kernel | Politica de fallo del kernel y preservacion de evidencia cuando sea posible; puede requerir panic/reinicio | Curar memoria arbitraria, atribuir siempre al culpable o continuar de forma fiable solo matando una app |

Una accion automatica necesita evidencia especifica y alcance definido. No se
activa panic_on_warn o panic_on_oops globalmente en esta ronda. Un kernel ya
comprometido no constituye por si mismo un arbitro confiable para garantizar
su propia recuperacion o un registro completo del incidente.

## Comprobaciones necesarias antes de implementar y promover

- Titular incorrecto, referencia ajena/caducada, generacion agotada y derechos
  insuficientes; rechazo conserva datos segun contrato.
- Overflow de offset/tamano, cuota y cola llena; contrapresion y error definidos.
- Carrera pedido/revocacion/cierre, cancelacion tardia y muerte del cliente con
  operaciones en vuelo; no liberar recursos mientras otro actor los usa.
- Aislamiento efectivo del consumidor y medicion de coste/latencia; probar
  primero en QEMU sin discos personales. Un modelo de estados no prueba el kernel.

Escritura revisa el protocolo; Caza UB prueba limites y fallos; Fluidez mide
coste y justicia de servicio; Custodia aplica titularidad, presupuestos y vida
util en el consumidor integrado. Estado actual: categoria y contrato definidos;
modelo C de copia/retirada probado. Gestor con identidad autenticada, enforcement
entre procesos, experimento de rendimiento e integracion siguen pendientes.

Fuentes primarias: [recursos registrados io_uring](https://man7.org/linux/man-pages/man2/io_uring_register.2.html),
[comportamiento del kernel ante oops/panic](https://www.kernel.org/doc/html/latest/admin-guide/sysctl/kernel.html).
Soporte y semantica se contrastan con la version fijada antes del prototipo.
