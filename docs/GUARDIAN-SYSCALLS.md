# Guardian de llamadas al sistema: politica estricta

**Estado actual: ronda 6.** Se restringieron comandos ioctl/fcntl/prctl/arch_prctl,
se rechazan sockets heredados por stdio, se separa lectura de ejecucion en
Landlock (ABI minima 6) y se corrigio la terminacion del guardia como PID 1.
Hay equivalencia simbolica verificada del filtro exportado por caja contra su
contrato y 17/17 controles con datos sinteticos. Alcance, evidencia y pendientes:
[GARANTIAS-RONDA6.md](GARANTIAS-RONDA6.md).

El inventario y la validacion descritos debajo corresponden a la ronda 5;
las limitaciones y requisitos que cambiaron quedan sustituidos por la ronda 6.

La caja concede una declaracion compilada; el kernel filtra todas las llamadas
que llegan por la ABI admitida. Observar una llamada no la autoriza. El catalogo
describe, las pruebas verifican y la revision modifica la declaracion.

## Inventario reproducible

`scripts/catalogo-syscalls.py` lee la tabla x86 del commit fijado
`32c52ec87d6245ee2d12ed5dd509e40062939b50`: **386 entradas nativas y 36 x32**.
Emite CSV, metadatos con hashes de entradas y un header completo de nombres para
los diagnosticos. No depende de los numeros de los headers del taller para nombrar
llamadas del kernel ULTRA.

El CSV incluye numero, ABI, entrada, simbolo esperado, resultado de busqueda en
System.map, permiso, motivo, observacion en strace, proposito y fuente del proposito.
Los 22 nombres sin resumen en el indice man(2) instalado tienen una descripcion
complementaria contrastada con su implementacion en el commit fijado. No hay
propositos pendientes en esta ronda.

**System.map es evidencia de simbolos, no una prueba de disponibilidad o funcionamiento.**
No encontrar el simbolo esperado tampoco prueba que una interfaz sea imposible.
La carga strace usa BusyBox estatico en el kernel Arch del taller; los simbolos
corresponden al perfil generico ULTRA. Son mediciones distintas y quedan identificadas.
No observar una syscall en esa carga no demuestra que nunca se necesite.

## Permisos y restricciones

`src/caja/syscalls-politica.h` declara **46 llamadas**, cada una con motivo.
Las otras **340 entradas nativas** quedan denegadas por defecto, al igual que
numeros no declarados que aparezcan en kernels posteriores. No existe un modo
de aprendizaje que conceda permisos al encontrar un rechazo.

El filtro compartido por caja y pruebas, `seccomp-politica.h`, comprueba primero
la arquitectura y rechaza la ABI i386 y el rango x32. Ademas:

- `mmap` y `mprotect` no pueden pedir PROT_EXEC. Se admite memoria de datos.
- `setuid` y `setgid` solo admiten 65534: BusyBox vuelve a confirmar su identidad.
- Una declaracion con duplicados, numeros invalidos, demasiadas entradas o ptrace
  se rechaza antes de instalar el filtro.
- Sin autorizacion, SECCOMP_RET_TRACE detiene la llamada antes de ejecutarla.
  El supervisor registra nombre y numero, mata al PID 1 de la caja y **no reanuda
  el hilo infractor**. La muerte de PID 1 termina sus descendientes.
- La caja exige namespaces de montaje/PID/UTS/IPC/red y Landlock ABI >= 3;
  falta de soporte implica rechazo. Los montajes quedan privados.
- Se cierran descriptores heredados desde el 3. Stdin/stdout/stderr siguen siendo
  capacidades concedidas por quien lanza la caja.
- Se retiran y verifican las capacidades del conjunto limite. Tras cambiar a
  65534 se aplica no_new_privs. Hay rlimits de memoria virtual, procesos,
  descriptores, CPU (30 s por proceso), archivo (16 MiB por archivo) y core cero.

`ioctl`, `fcntl`, `prctl`, `arch_prctl` y otras llamadas concedidas aun admiten
mas argumentos de los que se han caracterizado. Una lista por numero no equivale
a autorizacion semantica completa. No hay cuotas agregadas mediante cgroups ni
limite propio de tiempo de pared en caja; la auditoria tiene timeout externo.
La politica de PROT_EXEC es para este BusyBox estatico: programas dinamicos/JIT
necesitaran otra declaracion revisada. execve sigue permitido para los binarios
accesibles; no se elimina todo codigo ejecutable del proceso.

Seccomp **no desmapea codigo del kernel** ni sustituye Landlock, credenciales o
namespaces. Se aplica a syscalls que llegan al kernel, no a operaciones resueltas
por vDSO. No se ha medido el coste; no se promete coste nulo ni cobertura de todas
las combinaciones de argumentos.

## Validacion

`probar-syscalls.ps1` transfiere las fuentes a una auditoria separada del taller:

1. Mide una carga de BusyBox con strace y genera el catalogo del kernel fijado.
2. Revisa y compila caja y su prueba con los guardianes de C.
3. Evalua el BPF para las 386 entradas nativas y sus variantes x32, arquitectura
   i386, numero desconocido, PROT_EXEC, cambios de identidad y declaraciones invalidas.
4. Ejecuta cinco controles en el kernel del taller: getpid permitido, socket
   denegado, mprotect ejecutable denegado, mmap de datos permitido, x32 terminado
   por SIGSYS. Estas pruebas usan el filtro sin tracer: TRACE se traduce a ENOSYS.
5. Ejecuta **la caja con su supervisor real**, en un rootfs de prueba dentro de
   namespaces externos de la VM: 13/13 controles, incluido descriptor heredado
   cerrado y llamada prohibida de un descendiente sin continuar despues.

Los numeros del BPF comprueban decisiones del filtro; **no ejecutan las 386
syscalls contra el kernel**. Los controles positivos evitan considerar seguro
un filtro que simplemente impida funcionar a todo el sistema.

Resultados: `evidencias/syscalls-ronda5.txt`, `syscalls-ronda5.csv`,
`syscalls-metadatos-ronda5.json` y `syscalls-busybox-ronda5.strace`.
El script requiere gcc, python, strace, man-db, BusyBox estatico y sudo en la VM.
El indice man-db debe haberse creado con mandb.

Para repetir desde Windows: `./probar-syscalls.ps1`.
Para el frente C: `./probar-guardianes.ps1`.
Las pruebas no instalan el kernel en el PC fisico.

Estado de integracion: la caja y los filtros de esta ronda se compilaron y probaron
en el kernel Arch 7.2.11 del taller. La reconstruccion completa de ULTRA intento
usar un init que acababa de dividirse en varios archivos y fallo por `init.h`
ausente en la transferencia de ese intento. Los fuentes/lanzador del init
siguieron cambiando en paralelo; no se atribuye ese trabajo a esta auditoria.
No se acredita un nuevo arranque de ULTRA con estos guardianes. El build exige
ahora SYSVIPC e IPC_NS, sin la antigua degradacion silenciosa de namespaces.

Referencia primaria: [Seccomp BPF del kernel Linux](https://www.kernel.org/doc/html/latest/userspace-api/seccomp_filter.html).
