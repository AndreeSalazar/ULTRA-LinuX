# Vida util de objetos C y primer banco ASM

Fecha: 2026-10-22. Verificado en QEMU; candidato de biblioteca, sin consumidor
de produccion ni despliegue en A:. No se afirma una mejora de FPS o de latencia.

## Objetos C con propietario

[ultra_owner.h](../src/guardian/produccion/ultra_owner.h) y su implementacion
mantienen hasta 16 objetos por pool opaco. Cada handle contiene indice y
generacion. Liberar invalida la generacion antes de permitir otra reserva en
ese espacio. Una generacion agotada retira el espacio definitivamente en vez de
volver a validar un handle antiguo. Esa rama extrema se reviso en fuentes;
el ensayo no recorre 2^32 generaciones.

El usuario opera con read/write acotados; no recibe un puntero al almacenamiento
interno. Los objetos se inicializan a cero. Los errores se devuelven como estado
obligatorio y no cambian el destino ni el handle de salida. Los limites se
comprueban por resta despues de validar el desplazamiento para evitar overflow.
La destruccion del pool libera tambien los objetos que siguen reservados.

Contrato imprescindible:

- El pool debe seguir vivo y los handles pertenecen exclusivamente a ese pool.
  Un handle de otro pool puede coincidir: no es una capacidad criptografica.
- Uso en un hilo o con exclusion externa que cubra todas las operaciones,
  incluida la destruccion; no ofrece proteccion concurrente automatica.
- Los buffers externos deben ser spans reales y vivos. No valida punteros C
  arbitrarios, un pool destruido, corrupcion de memoria ni aliases externos.
- El consumidor debe fijar presupuestos de memoria: 16 objetos no limitan la
  suma de sus tamanos. No hay borrado seguro de secretos antes de liberar.

El ensayo incluye 1000 ciclos de reserva/liberacion/reutilizacion, lectura con
handle antiguo mientras el espacio ya tiene otro objeto, doble liberacion,
matrices de lectura y escritura, nulos, SIZE_MAX, agotamiento de 16 espacios y
fallos simulados de calloc. Comprueba todos los bytes del destino y que una
escritura rechazada conserva el objeto. El wrapper de calloc existe solo en
el ejecutable de pruebas; no agrega hooks al codigo de produccion.

## Puentes C/ASM

[ASM/laboratorio](../ASM/laboratorio) implementa un banco GNU assembler para
Linux x86-64 System V. El puente guarda y restaura los seis registros preservados
rbx, rbp y r12-r15 aunque el callee sintetico los altere. Verifica centinelas y
que el callee observe rsp modulo 16 igual a 8 en una llamada ordinaria.

Cada uno de 1000 ciclos ejecuta cuatro controles: callee correcto, alteracion
de seis registros (mascara 63), llamada desalineada (mascara 64) y nueva llamada
correcta para comprobar recuperacion. El caso desalineado solo llama una hoja
ASM que observa rsp: no llama libc ni instrucciones SIMD con ese contrato roto.
La pila tiene marca GNU-stack no ejecutable.

Esto verifica contratos seleccionados; no contiene un callee que corrompa rsp
o la direccion de retorno, no prueba todos los registros SIMD/flags, no es un
interceptor de ASM arbitrario y no aporta tablas CFI de desenrollado ni valida
excepciones C++ a traves del puente. No se integra en el kernel. Los sanitizadores
de C no convierten automaticamente el interior ASM en codigo instrumentado.

Referencias: [System V AMD64 psABI](https://gitlab.com/x86-psABIs/x86-64-ABI) y
[opciones de instrumentacion GCC](https://gcc.gnu.org/onlinedocs/gcc/Instrumentation-Options.html).

## Resultado reproducible

[Ronda 11](../evidencias/laboratorio-c/ronda11-116ad4197ab843599a2c700c94378ee2/README.md):
GCC 16.2.1 20260810, kernel del taller 7.2.11-arch1-1, VM de 4 GiB y cuatro CPU,
sin discos personales. Procesos bwrap sin red, con timeout y limites prlimit;
no se aplica un cgroup de 512 MiB por proceso.

Cuatro perfiles: produccion O2/O3 static-pie con endurecimiento y trampas UBSan;
diagnostico O1/O2 con ASan/UBSan. En cada perfil pasan **644023 comprobaciones C**
y **1000 ciclos ASM de cuatro controles**. Son las mismas pruebas repetidas bajo
cuatro configuraciones, no millones de clases independientes de defectos.
El analizador GCC y la compilacion con avisos como errores tambien pasan.

La auditoria existente sigue pasando; sus ocho huecos historicos permanecen
abiertos globalmente. La carrera de hilos sigue amarilla. Esta API evita algunos
accesos caducados dentro de su contrato y no elimina UAF en el resto del sistema.

Siguiente: escoger un consumidor con propiedad real de objetos, medir su coste
y migrarlo con pruebas; ampliar agotamiento de generaciones, contratos entre
pools e hilos; despues verificar desenrollado/CET e inline ASM por separado.
Antes de desplegar, reconstruir imagen y validar arranque nativo.
