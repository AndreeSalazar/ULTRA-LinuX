# CASM: primero Linux, despues metal64

## Estado comprobable actual

**CASM Native 0.1 ya se autocompila en Linux x86-64.** El compilador esta escrito
en `native/compiler.casm`; tres generaciones desde GCC y otras tres desde Clang
son identicas. Dentro de una raiz vacia, sin compiladores, Python ni libc, produce
dos generaciones mas como UID 1000 y compila y ejecuta otra fuente diferente.
[Lenguaje, verificador y uso](native/README.md). Evidencia:
[`evidencias/casm-native/2026-10-25`](../evidencias/casm-native/2026-10-25/).

Este cierre corresponde al **perfil Native**. El perfil **metal64 0.2** conserva
su frontend C; no es compatible automaticamente con la nueva gramatica. El
traspaso metal64 y la ampliacion de tipos/bibliotecas siguen pendientes.

Primer paso fuera de Linux: [modulos Native freestanding](native/FREESTANDING.md),
sin servicios Linux en la imagen ejecutable. El anfitrion del compilador permanece
Linux; faltan cargador BMO-X y ABI de servicios para repetir alli la autocompilacion.

## Estado del perfil metal64 0.2 y primer tramo anterior

**Este frontend metal64 todavia no se autocompila.** Su frontend y codificador son C11.
La gramatica CASM 0.2 no puede expresar ese compilador: solo tiene una funcion,
constantes y llamadas a servicios; faltan variables en ejecucion, funciones,
bucles, memoria mutable y entrada de archivos.

La prioridad acordada es un compilador ejecutable **Linux ELF x86-64**, seguido
del destino BMO-X/metal64. El seguimiento esta en
[`autocompilacion.csv`](autocompilacion.csv); los estados no son porcentajes de
compatibilidad ni un catalogo completo de C.

## Lo que ya cambia

1. **GCC deja de ser la unica semilla:** `construir.sh --compilador clang` construye
   el mismo compilador C con Clang. Esto no usa LLVM para generar los programas
   CASM: su codificador x86-64 sigue siendo propio.
2. **Uso independiente del ejecutable GCC/Clang:** `bootstrap.py asegurar` verifica
   fuentes y binario y reutiliza una semilla vigente. `compilar.sh` ya no recompila
   el compilador en cada ejecucion. La prueba pone trampas en PATH para ambos
   compiladores y aun puede producir y ejecutar el ELF. Siguen presentes libc,
   Python y utilidades del anfitrion; no es independencia total ni self-hosting.
3. **Destino Linux acotado:** `compilar.sh --linux` conserva `INICIO.X86` y añade
   `program.elf`, `program.linux.hex` y `program.linux.json`. El adaptador construye
   los encabezados ELF y una tabla de servicio propia: `metal_puts` termina usando
   `write`, con reintento EINTR y escrituras parciales, y `exit` al terminar.
   Un error devuelto por `write` termina con codigo 74 (SIGPIPE conserva su
   terminacion normal por señal). No se omiten los permisos del kernel.
4. **Contrato de bytes:** el payload se verifica antes de adaptarlo. Su contenido
   se conserva exactamente en `payload_offset`; el HEX Linux completo vuelve a
   convertirse en los mismos bytes ELF. No se invocan `as`, `ld`, GCC ni Clang
   para esa adaptacion. Segmento RX, pila RW/NX, sin interprete dinamico.

El ELF actual usa direccion fija, sin PIE ni declaracion de activacion CET; es un
adaptador de laboratorio, no el perfil completo de endurecimiento del compilador
C ni una implementacion general del ABI SysV. No carga archivos ni asigna memoria
para un compilador futuro. Los enteros 0.2 siguen calculandose durante la compilacion.
El parche experimental 0.3 de `traspaso/` permanece separado y sin aplicar.

Contratos de plataforma: [formato ELF de Linux](https://man7.org/linux/man-pages/man5/elf.5.html)
y [escrituras parciales, EINTR y SIGPIPE](https://man7.org/linux/man-pages/man2/write.2.html).

## Validacion del primer tramo

Evidencias y versiones exactas:
[`evidencias/casm-linux/2026-10-25`](../evidencias/casm-linux/2026-10-25/).
El taller uso GCC 16.2.1 y Clang 23.1.3; no se reconstruyo la imagen de la distro
ni se modificaron discos fisicos para este banco.

`python3 CASM/contrastar.py`, dentro de QEMU como usuario sin privilegios:

- GCC normal: 108 casos; GCC ASan/UBSan: 107.
- Clang normal: 108 casos; Clang ASan/UBSan: 107.
- En las variantes instrumentadas se omite expresamente `host-memory`: limitar
  RLIMIT_DATA a 256 KiB impide iniciar ASan, no reproduce el fallo de calloc del
  compilador. El caso si se comprueba en las dos variantes normales.
- 128 fuentes deterministas con cada una de las cuatro variantes: **512 emisiones**.
  HEX, IR y paquete deben coincidir byte por byte; el verificador Python vuelve a
  comprobar la aritmetica y los bytes. Se conserva la huella de oro de `hello.x86`.
- Uso de la semilla vigente con GCC y Clang bloqueados mediante trampas de PATH.
- Cuatro pruebas ELF: ejecucion de cadenas/enteros y fallo de escritura, texto
  vacio, bytes alterados y salida que ya existe.
- Cuatro pruebas de cache: cambio de ejecutable, fuente/perfil, manifiesto invalido
  y rutas redirigidas. SUPER GRAFO indexo 26 archivos/10 dependencias y reviso
  `src/frontend.c` con GCC y Clang: cero errores y avisos en esas dos pasadas.

Coincidir entre dos compiladores no demuestra ausencia de UB: comparten el
algoritmo fuente. Tampoco demuestra que CASM haya construido el compilador.

## GCC y Clang/LLVM: C y C++ en bancos separados

`python3 scripts/probar_ub_compiladores.py` ejecuta en QEMU **44 controles** con
`gcc`, `g++`, `clang` y `clang++`, C17/C++20 y ASan/UBSan. Cada fallo tiene su
version correcta: desbordamiento signado, division por cero, desplazamiento
fuera de anchura, uso tras liberar y acceso fuera de limites; C++ añade
`new[]` con `delete` incorrecto. Se guardan version, banderas, huella, diagnostico
y codigo de salida. UBSan de GCC puede detectar object-size antes de ASan en el
caso fuera de limites; se exige el diagnostico de esa causa.

Clang es el frontend de C/C++ de LLVM; LLVM no es otro lenguaje C++ del museo.
No se modificaron GCC ni LLVM, ni se aplicaron sanitizadores al escritorio o juegos.
Faltan muchas familias y caminos de UB, carreras, lifetime C++, aliasing y casos
negativos reales; no se infiere proteccion completa de estos controles.

Fuentes oficiales: [instrumentacion GCC](https://gcc.gnu.org/onlinedocs/gcc/Instrumentation-Options.html),
[AddressSanitizer](https://clang.llvm.org/docs/AddressSanitizer.html) y
[UndefinedBehaviorSanitizer](https://clang.llvm.org/docs/UndefinedBehaviorSanitizer.html).

## Criterio de cierre y continuidad

La ruta de autocompilacion necesita valores mutables en ejecucion, funciones
con parametros/retorno y marcos de pila, condicionales/bucles, buffers con longitud y
acceso comprobado, mas servicios Linux de lectura/escritura acotados. Cada extension
debe entrar a la vez en frontend, emisor, verificador y casos mal/bien.

Debe existir una fuente **del propio compilador escrita en CASM**, no un
programa que copie un ejecutable o que invoque GCC. La semilla externa construye
la etapa 1; esta construye la etapa 2 desde esa fuente; la etapa 2 vuelve a
construirse. Se exigen igualdad determinista de etapas 2/3, corpus correcto y
rechazos, bootstrap desde GCC y desde Clang, y ejecucion sin ambos presentes.
Ese criterio ya se cumplio para Native: `self_hosted` es `true` exclusivamente
para su perfil, sin atribuirlo al compilador metal64 de C. Metal64 necesita sus propios
servicios y la validacion de su ABI; no hereda los syscalls de Linux.
