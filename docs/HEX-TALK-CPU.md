# HEX en ULTRA: ideas tomadas del analisis de TALK-CPU

Fecha: 2026-10-22. Repositorio consultado, sin modificarlo ni ejecutar sus
instaladores: [TALK-CPU-x86-64, revision b5b8e88](https://github.com/AndreeSalazar/TALK-CPU-x86-64/tree/b5b8e881a33ffb8a9af4f63285dbfa6c0ae47e87).
Se leyeron README, CASM/verify.py, CASM/src/emitter.c, CASM/LANGUAGE.md,
hello.hex, guardians/README.md, allocate.hex y latch-test.hex. Es una revision
dirigida de esas partes, no una auditoria completa de todos los archivos.

## Influencia que sirve

| Idea observada | Aplicacion propuesta en ULTRA | Limite |
|---|---|---|
| Emisor CASM separado del verificador HEX | Inspeccionar artefacto final por simbolos y contratos pequenos | No demuestra toda la semantica de C |
| Ventanas [inicio, fin), propietario y motivo | Presupuestos de secciones, buffers, regiones y objetos | No copiar direcciones fisicas fijas del firmware a un proceso Linux |
| Cursor/complemento y bloqueo persistente tras corrupcion | Explorar un estado de error irreversible de un pool si sus metadatos fallan | El complemento no detecta alteraciones coherentes de ambos campos |
| Rechazos que no consumen memoria ni borran la causa | Mantener salidas/estado en errores de API; informar causa estable | Error de usuario y corrupcion interna necesitan acciones diferentes |
| Mutaciones de copias y controles validos | Comprobar que un contrato rechaza cambios deliberados | Un hash aprobado no certifica que el programa carezca de errores |

CASM reconstruye los bytes admitidos a partir de su IR y compara el cuerpo
entero; su propio informe reconoce que la semantica de fuentes depende del
frontend. Es un verificador de emision acotada, no un verificador general de C.
Su interfaz restringida evita exponer operaciones que todavia no implementa.

## Diferencia de ABI que hay que respetar

El ejemplo [hello.hex](https://github.com/AndreeSalazar/TALK-CPU-x86-64/blob/b5b8e881a33ffb8a9af4f63285dbfa6c0ae47e87/EVIDENCIAS/CASM/hello.hex)
comienza con una direccion relativa RIP en RSI, una llamada mediante
[r15+0x10] y RET; despues viene texto, que no debe interpretarse como codigo.
Ese servicio pertenece a **metal64**, con un contrato propio. El asignador usa
RCX y devuelve CF/RAX/EDX. ULTRA en espacio de usuario Linux usa **System V
AMD64**; una funcion C ordinaria no publica ese contrato de CF. Copiar el
hexadecimal de un servicio no adapta sus argumentos, pila ni permisos.

Tampoco corresponde cargar desde un proceso Linux las tablas de paginas,
IDT o registros de control de tu firmware. La idea transferible son contratos
verificables, no reemplazar mecanismos privilegiados desde el escritorio.

## Primera comprobacion realizada

Se agrego [inspeccionar-asm-hex.py](../scripts/inspeccionar-asm-hex.py).
No ejecuta el binario inspeccionado. Comprueba cabecera ELF64 little-endian
x86-64, usa GNU objdump y extrae cuatro simbolos del banco de la ronda 11.
Conserva bytes, tamano, desensamblado, version de herramienta y SHA-256.
Un contrato exacto se aplica **solo** a ultra_abi_good; los otros tres simbolos
se inventarian, sin afirmar que su semantica se verifica automaticamente.

Resultado real de ultra_abi_good, igual en los cuatro perfiles:

| Bytes | Instruccion Intel | Funcion |
|---|---|---|
| 48 89 E0 | mov rax, rsp | Observar el puntero de pila |
| 83 E0 0F | and eax, 15 | Obtener su resto modulo 16 |
| C3 | ret | Volver al puente |

La rutina ocupa siete bytes. Las direcciones del ejecutable pueden variar
entre perfiles; no se usa una direccion fija ni se comparan ejecutables enteros
esperando que sean identicos. Una codificacion alternativa equivalente o agregar
ENDBR64 cambiaria el contrato y requeriria revisarlo deliberadamente.

El autocontrol acepta el cuerpo valido y rechaza cuatro mutaciones en memoria:
vacio, truncado, byte adicional y primer byte alterado. Tambien comprueba el
extractor y el rechazo de un simbolo ausente. No se parcharon los ejecutables.

[Evidencias HEX](../evidencias/asm-hex/2026-10-22/README.md): cuatro perfiles
prod-O2, prod-O3, diag-O1 y diag-O2; GNU Binutils 2.47. Se contrastaron sus hashes
con los ejecutables de las evidencias de la ronda 11. Esa ronda aporta las
pruebas dinamicas de ABI; esta nueva entrega aporta inspeccion estatica.

## Lo que mejora y lo que falta

HEX hace visible lo que se emitio, detecta un cambio accidental en el contrato
seleccionado y conecta fuente, simbolo y artefacto probado. Es otra capa de
evidencia, no una optimizacion de CPU por si misma ni una prueba de vida util.
Los permisos y los handles siguen siendo necesarios. Una instruccion valida
puede operar sobre un objeto muerto; ese hecho no esta escrito en su opcode.

Siguiente orden propuesto: validar contratos estructurales del puente completo,
revisar CET y desenrollado, incluir relocaciones y limites de secciones, y ampliar
el pool con identidad entre pools y control de concurrencia. Una traduccion
general de GCC a un lenguaje restringido como CASM requeriria mucho mas trabajo;
no se incluye ni se promete aqui. No se midio aceleracion ni se instalo en A:.

Actualizacion posterior: [contratos estructurales ELF en produccion](ELF-PRODUCCION.md)
integra la idea en build/instalacion de cinco ejecutables propios por perfil.
Los perfiles generico y PC se reconstruyeron, probaron en QEMU y copiaron a A:.
El contrato de siete bytes de esta inspeccion sigue siendo un banco especifico.
