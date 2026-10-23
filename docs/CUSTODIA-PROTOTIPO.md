# Primer modelo ejecutable de Custodia

Evolucion: [consumidor local de contadores RAM y verificacion HEX](RECURSOS-C-PRODUCCION.md).

Ampliacion: [pruebas de estres, modelo de estados y mutaciones](CUSTODIA-ESTRES.md).
La primera ronda descrita abajo conserva su evidencia historica.
Primer candidato de velocidad: [ticket inline y resultados](CUSTODIA-RENDIMIENTO.md).

[Evidencias y hashes de la ronda](../evidencias/custodia/custodia-76f107552ae748529c0ff7c6d6cda321/SHA256SUMS):
GCC 16.2.1 20260810, kernel del taller 7.2.11-arch1-1 x86-64.
Se conservan textos/HEX e identidades de binarios probados; los ejecutables del
taller no se copian al repositorio. Los 13 fuentes/herramientas listados en el
manifiesto se contrastaron con el proyecto antes de guardar la evidencia.

2026-10-23, America/Lima. Implementacion C17 de laboratorio en
`src/guardian/laboratorio/custodia/`. No esta en el kernel ni desplegada en A:.
No introduce un bypass de syscalls, prioridad de juegos, io_uring o aislamiento
de aplicaciones. Materializa una parte del [contrato de Custodia](GUARDIAN-CUSTODIA.md).

## Lenguajes y papel de HEX

- C: estados, referencias, cuotas, resultados obligatorios y copia acotada.
- ASM: no se agrega codigo manual en esta ronda. Solo se justificara una pieza
  concreta con contrato ABI y evidencia de necesidad/coste.
- HEX: representacion de los bytes compilados, junto con desensamblado y hash.
  No es un lenguaje distinto que otorgue permisos o acelere automaticamente.

Se reutiliza el extractor de `scripts/inspeccionar-asm-hex.py` mediante el nuevo
`scripts/inspeccionar-custodia-hex.py`. La influencia de TALK-CPU disponible es
la [revision local documentada de b5b8e88](HEX-TALK-CPU.md). No se ha podido
recuperar main durante esta ronda; no se afirma una nueva auditoria del repo.
No se copian bytes de su ABI metal64 al ABI System V de Linux.

El inspector valida la cabecera ELF64 little-endian x86-64 y extrae uc_begin,
uc_complete y uc_revoke con GNU objdump sin ejecutar el binario inspeccionado.
Guarda sus bytes .hex, desensamblado, hashes y version de herramienta. No exige
bytes identicos entre optimizaciones ni certifica semantica de esas funciones.
La ejecucion de tests es otra etapa y solo ocurre dentro del laboratorio.

## Contrato implementado

`uc_create(max_bytes, max_requests)` crea un gestor vivo con hasta 16 prestamos
y 16 pedidos pendientes. Max_requests se limita a 1..16. Max_bytes cuenta
payloads copiados; metadata, tickets y coste del asignador no estan incluidos.
No es una cuota de memoria del proceso ni una reserva de RAM fisica.

`uc_lend` copia un span real y vivo a un recurso privado. No expone un puntero
interno. La modificacion posterior del origen no altera la copia prestada.
Los tokens reutilizan las generaciones e identidad de pools de ultra_owner,
con pools separados para payloads y tickets. No se reciclan identidades de pool;
su agotamiento rechaza nuevas creaciones segun el contrato de owner.

`uc_begin` acepta un rango dentro de un prestamo activo del titular indicado.
Reserva un ticket y una plaza de cola. Los limites usan resta comprobada para
no aceptar overflow de offset + size. En errores, el token de salida permanece
intacto. Una asignacion fallida no consume cuota o plaza.

`uc_revoke` impide pedidos nuevos. Los ya aceptados pueden completarse o
cancelarse. Payload y cuota permanecen retenidos hasta retirar el ultimo.
`uc_complete` comprueba capacidad de destino y copia; un rechazo por destino
insuficiente/NULL no consume el ticket, por lo que se puede reintentar o cancelar.
Capacidad y puntero externo siguen teniendo que corresponder a un objeto vivo.

`uc_cancel` retira un ticket sin copiar. Repetir una finalizacion/cancelacion
consumida se rechaza como caducada. `uc_destroy` devuelve UC_BUSY si quedan
pedidos pendientes, sin destruir el gestor; en otro caso libera sus prestamos.
Un fallo de una invariante interna de owner aborta: no se oculta y se continua
como si el recurso siguiera siendo valido. Esto no detecta toda corrupcion.

## Limites que siguen abiertos

El campo subject lo proporciona un llamador confiable: el modelo compara ese
valor pero NO autentica a un proceso. Codigo hostil en el mismo proceso puede
mentir sobre el valor o corromper memoria. El token no es una capacidad de
seguridad. El kernel no reconoce estos numeros como permisos especiales.

Todos los accesos al gestor se serializan por contrato. No hay prestamos entre
hilos sin sincronizacion, IPC, operaciones reales de archivo, mmap, DMA,
expulsion de procesos ni autentificacion. No simula que una peticion haya
terminado por agotar un timeout. Destruir el gestor invalida su puntero; no se
detecta automaticamente que el llamador vuelva a usarlo.

El primer recurso es una copia en RAM de solo lectura, no un archivo abierto.
Esto permite probar ciclo de vida/cuotas sin efectos en discos. La siguiente
frontera necesita un consumidor real y una identidad verificable del cliente,
antes de extender el contrato a archivos y operaciones asincronas reales.

## Pruebas y reproduccion

`scripts/campo-custodia.sh` requiere una salida nueva ULTRA_CUSTODIA_OUT. Usa
los perfiles existentes, GCC -fanalyzer y avisos como errores; ejecuta mediante
aislar-mina.sh (bwrap sin red/home/discos personales, timeout y prlimit).
Los perfiles diagnosticos usan ASan/UBSan; no se trasladan a la imagen de juegos.

Por cada perfil pasan 49076 comprobaciones en 1000 ciclos, con copia estable,
titular incorrecto, handle de otro gestor, rangos extremos, cuota de bytes y
cola, 16 slots, retirada y drenaje, doble finalizacion/cancelacion, destruccion
ocupada, destino corto intacto, lectura vacia y fallos de asignacion. Son las
mismas pruebas en cuatro configuraciones, no clases independientes de defectos.

El inspector conserva tres simbolos en cada perfil. Su extractor previo pasa
selftest y los negativos de ELF invalido/simbolo ausente rechazan sin publicar
una carpeta de resultado. La inspeccion estatica no demuestra la ejecucion.

No se ha medido una mejora de velocidad, coste comparado ni FPS. No se ejecuta
este modelo en juegos; integracion, concurrencia y politica de prioridades
siguen pendientes. No se necesita recompilar el kernel para probar este modelo.
