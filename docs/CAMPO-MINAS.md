# Campo de minas de C

## Ronda 6: terminacion estricta de PID 1

El guardia imprime su bandera y sale inmediatamente cuando getpid() devuelve 1,
para no reejecutar un fallo bajo las reglas especiales de senales del init de un
namespace. Rechazo explicito y trampa se comprobaron por separado. Se repitieron
las 62 minas: 53 VERDE, 1 NEUTRA, 8 AMARILLO, 0 ROJO; 36 rechazos de compilacion.
Evidencia en `evidencias/campo-minas-ronda6.txt` y `guardian-pid1-*-ronda6.txt`.
Las barreras y garantias delimitadas del frente syscall estan en
[GARANTIAS-RONDA6.md](GARANTIAS-RONDA6.md).

## Ronda 5: reglas explicitas antes de operar

62 minas de ejecucion: **53 VERDE, 1 NEUTRA, 8 AMARILLO, 0 ROJO**; 36 minas de
compilacion rechazadas. Se mantienen cuatro perfiles validos de cruces y el
rechazo esperado de produccion O0; TSan detecta la carrera y acepta el control atomico.

Ocho casos nuevos ejercitan `ULTRA_DIV_INT`, `ULTRA_REM_INT` y `ULTRA_SHL_INT`:
las funciones comprueban divisor, extremos y desplazamientos **antes de operar**.
El evaluador exige la bandera correspondiente, y los controles positivos
comprueban extremos validos y evaluacion unica de argumentos. Estas API son
explicitas y para int; no transforman operadores C ajenos ni validan UB al
calcular un argumento. Detalles en [GUARDIAN-ARITMETICA-ESTRICTA.md](GUARDIAN-ARITMETICA-ESTRICTA.md).

Evidencias actuales: `evidencias/campo-minas-ronda5.txt`,
`cruces-guardianes-ronda5.txt`, `hilos-ronda5.txt` y `aislamiento-ronda5.txt`.
El frente de permisos de kernel, probado por separado, esta en
[GUARDIAN-SYSCALLS.md](GUARDIAN-SYSCALLS.md).

## Ronda 4: nuevas familias y aislamiento por caso

Se ejecutó en QEMU, con GCC 16.2.1, desde `probar-guardianes.ps1`:

- 36 minas de compilación rechazadas.
- 54 minas de ejecución: **45 VERDE, 1 NEUTRA, 8 AMARILLO, 0 ROJO**.
- Diez casos nuevos: producto con signo, negación de INT_MIN, resto INT_MIN/-1,
  VLA cero, índice negativo, límite de subobjeto, cadena sin NUL, uso tras bloque,
  comparación y resta de punteros de reservas distintas.
- Diagnóstico ampliado con `pointer-compare`, `pointer-subtract` y
  `detect_invalid_pointer_pairs=2`. Estas comprobaciones necesitan ASan.
- Hilos en un campo aparte: TSan detectó una carrera real; el control con
  contador atómico dio exactamente 20000 y terminó sin avisos. La carrera es
  AMARILLO: esta instrumentación no está en producción.
- Se comprobó el rechazo de ASan+TSan y de producción O0 con FORTIFY. Los cuatro
  perfiles válidos de los cruces pasaron sus controles dentro del aislamiento.

`scripts/aislar-mina.sh` ejecuta **cada mina y cada control de ejecución** en un
Bubblewrap nuevo dentro de la VM: user/PID/mount/net/IPC/UTS/cgroup namespaces,
sin compartir la red del taller, sin montar `/home`, `/run` privado, `/usr` y
artefactos en solo lectura, `/tmp` privado de 16 MiB, entorno limpio, capacidades
eliminadas y `no_new_privs`. Se aplican límites de CPU (10 s), tiempo total (15 s),
archivos escritos (1 MiB por archivo), descriptores, procesos y pila; sin core dumps.
Bubblewrap se instaló como paquete oficial Arch en el taller.

Antes de las minas, `control-aislamiento.c` comprueba: ausencia del home, escritura
rechazada con EROFS, solo PID 1/reaper y PID 2/control visibles, ausencia de rutas
IPv4, entorno sin variables heredadas, CapEff=0, NoNewPrivs=1 y límites CPU/core.
Si falla, no se ejecutan las minas. Un fallo del sandbox, agotamiento declarado de
memoria o timeout es ROJO de infraestructura: no puede quedar AMARILLO por una gemela.

Esto contiene estas pruebas de usuario, **no constituye aislamiento absoluto**:
los procesos comparten el kernel de la VM, no hay lista seccomp ni cgroup de memoria
por caso y los rlimits no equivalen a cuotas agregadas. La compilación se ejecuta
como builder en el taller; no se introduce código arbitrario de terceros en este flujo.
El límite externo sigue siendo la VM. No se instala ningún kernel ni se endurecen
automáticamente los programas del sistema al ejecutar esta auditoría.

Los ocho AMARILLO son los cuatro anteriores y: cadena sin NUL, uso tras bloque,
comparación y resta de punteros ajenos. El campo de hilos añade otra limitación
separada. Más pruebas encontradas no significan menos protección.

Informes: `evidencias/campo-minas-ronda4.txt`,
`evidencias/cruces-guardianes-ronda4.txt`, `evidencias/hilos-ronda4.txt`.
Diagnósticos: `evidencias/hilos-carrera-ronda4.txt`,
`evidencias/hilos-control-ronda4.txt`, `evidencias/aislamiento-ronda4.txt`.
Catálogo de familias y huecos: [COBERTURA-UB.md](COBERTURA-UB.md).

Fuentes: [opciones de GCC](https://gcc.gnu.org/onlinedocs/gcc/Instrumentation-Options.html)
y [diseño de Bubblewrap](https://github.com/containers/bubblewrap/blob/main/README.md).

## Ronda 3: evidencia y cruces

Ejecutada en el taller QEMU con GCC 16.2.1. Informes:
`evidencias/campo-minas-ronda3.txt` y `evidencias/cruces-guardianes-ronda3.txt`.

- Compilación: 36 rechazos esperados, 0 ROJO.
- Ejecución: 44 casos; 39 VERDE, 1 NEUTRA, 4 AMARILLO, 0 ROJO.
- Casos nuevos: resta con signo fuera de rango, desplazamiento negativo,
  ULTRA_SUB, ULTRA_MUL con signo y producto de tamaños sin signo.
- El evaluador exige un diagnóstico compatible con la mina. Un código de error
  genérico, señal sin bandera, ejecutable ausente o timeout ya no cuenta como VERDE.
  La neutralización necesita su marcador explícito y salida cero. La lista de casos
  debe generarse correctamente y contener entradas.
- Cada ejecución tiene un límite de 15 segundos; no se generan volcados de memoria.
- Cuatro combinaciones aceptan controles válidos: producción O2/O3 y diagnóstico
  O1/O2. Se comprueban extremos de aritmética, índice evaluado una vez, copia exacta,
  regiones adyacentes, memmove solapado y truncamiento informado.
- La quinta combinación, producción O0, **debe rechazarse**: FORTIFY requiere
  optimización. El resultado correcto es ese rechazo por el motivo esperado.
  No es una combinación autorizada para distribuir.

Desde Windows, con el taller iniciado:

```powershell
.\probar-guardianes.ps1
```

El lanzador copia las pruebas a `auditoria-guardianes` dentro de la VM, ejecuta
las regresiones del evaluador, los cruces y el campo, y recupera ambos informes
solo si todo termina correctamente. No recompila ni instala el kernel.

Los cuatro AMARILLO siguen abiertos: uso tras free, lectura fuera por puntero,
uso tras retorno y lectura sin inicializar. La ronda no prueba ausencia de toda UB,
seguridad de programas ajenos ni todos los cruces del kernel. Las banderas de señal
y los diagnósticos genéricos tampoco demuestran por sí solos la causa exacta.
El laboratorio de velocidad continúa pendiente.

Referencia de opciones y restricciones:
[documentación oficial de GCC](https://gcc.gnu.org/onlinedocs/gcc/Instrumentation-Options.html).

## Método y resultados históricos

Cada mina es una forma clásica de romper un programa en C. Una mina "con bandera" es una que, al pisarla, despierta un guardián que dice **qué** pasó y **por qué** antes de detenerse. El campo crece con ULTRA LinuX: cada clase de fallo nueva que aparezca entra primero como mina, y solo después se le busca guardián.

```bash
bash scripts/campo-minas.sh
```

Se ejecuta en el taller (necesita GCC) y escribe `build/minas/informe.txt`. La copia de esta ronda está en `evidencias/campo-minas-ronda2.txt`.

## Cómo se mide

**Minas de compilación** (`src/guardian/minas-compilacion/*.c`): código peligroso que **no debe compilar**. Cada archivo lleva su `BANDERA:` y pasa por `ultra_revisar`: avisos como errores en `-O2` y analizador en `-O0`.

**Minas de ejecución** (`src/guardian/minas.c`): cada una se ejecuta de tres formas.

| Construcción | Opciones | Para qué |
|---|---|---|
| Sin guardianes | `ULTRA_CRUDO`: sin protector de pila, FORTIFY, CET ni RELRO | Ver qué hace la mina cuando nadie vigila |
| Producción | `ULTRA_PROD`: `-fhardened`, trampas UBSan y CET activo | Lo que se distribuye. Es la columna que decide |
| Diagnóstico | `ULTRA_DIAG`: ASan + UBSan completos | El motivo exacto, con línea |

Estados posibles:
- **VERDE:** producción detiene la mina con una bandera.
- **NEUTRA:** la mina no se puede ver al ejecutar, pero un guardián la vuelve comportamiento definido y comprobado.
- **AMARILLO:** solo la para el diagnóstico, o solo la revisión de compilación (su "gemela").
- **ROJO:** nadie la para. El campo falla.

## Resultado de la ronda 2

- **Compilación:** 36 de 36 VERDE.
- **Ejecución:** 34 VERDE, 1 NEUTRA, 4 AMARILLO y 0 ROJO, de 39 minas.
- **Sin guardianes:** 33 de 39 minas explotan en silencio o mueren sin explicar por qué.

Ejemplos de la misma mina sin y con guardianes:

| Mina | Sin guardianes | Producción |
|---|---|---|
| `formato-n` | sigue como si nada | `*** %n in writable segments detected ***` |
| `secuestrar-arranque` | sigue: el arranque queda secuestrable | `SEGV_ACCERR`: RELRO lo bloquea |
| `retorno-secuestrado` | ejecuta código ajeno | `SEGV_CPERR`: la pila sombra lo bloquea |
| `recursion-infinita` | muere sin explicación (señal 11) | `SEGV_MAPERR`: página de guarda |
| `alias-estricto` | **devuelve 1**, un valor viejo | devuelve 0, la memoria real (`-fno-strict-aliasing`) |
| `memcpy-solapado` | sigue con datos corruptos | `FRONTERA-SOLAPE minas.c:126` |

## Hallazgos de esta ronda

1. **`-ftrivial-auto-var-init=zero` no protege en `-O2`.** En GCC 16, `int x; if (set) x = 5; return x;` se compila como `return 5`: el optimizador usa la UB antes de que cuente la inicialización a cero. En `-O0` sí devuelve 0. La mina queda AMARILLO, porque solo la para la revisión en `-O0` (su gemela `sin-inicializar`). Lección: la revisión de compilación no es opcional.
2. **Un `memcpy` de tamaño constante no lo vigila nadie.** GCC lo expande en movimientos en línea y no llama a la biblioteca, así que ni ASan lo ve. Además, glibc nunca comprueba el solape. Nuevo guardián: `ultra_guard.h` envuelve `memcpy`. Comprueba **primero** el tamaño del destino, para que un desborde no se confunda con un solape, y después el solape. Su gemela `memcpy-solapado-constante` la para `-Wrestrict`.
3. **Un cast explícito calla al guardián de compilación** (`(size_t)-1`). FORTIFY lo atrapa al ejecutar (`signo-tamano`).
4. **glibc vigila siempre el heap.** `heap-subdesborde` y `free-pila` se detienen incluso sin guardianes. No es mérito de nuestras opciones, y el informe lo muestra así.

Guardianes nuevos de esta ronda: los venenos `strncpy`, `strncat`, `atoi`, `atol` y `atoll`; los avisos `-Walloca`, `-Wcast-qual`, `-Wcast-align=strict` y `-Wtrampolines`; `-fsanitize=bounds-strict`; el `memcpy` vigilado; y banderas de SIGSEGV que distinguen `MAPERR`, `ACCERR` y `CPERR`.

## AMARILLO pendientes (huecos conocidos)

| Mina | Qué la para hoy | Idea para cerrarla |
|---|---|---|
| `uso-tras-free` | ASan y el analizador (gemela) | Allocator endurecido con cuarentena, o etiquetas de memoria |
| `lectura-fuera` | ASan | Punteros con límites (`ULTRA_AT`, o tipos *slice* propios) |
| `uso-tras-retorno` | ASan y `-Wreturn-local-addr` (gemela) | Reglas de propiedad en la revisión |
| `sin-inicializar-ejecucion` | La revisión en `-O0` (gemela) | Mantener la revisión obligatoria; probar el patrón en cada GCC nuevo |

## Añadir una mina

1. Se reproduce el fallo en el archivo más pequeño posible.
2. **Si debe impedirse al compilar:** se añade `src/guardian/minas-compilacion/NOMBRE.c` con su línea `/* BANDERA: por qué es peligroso */`.
3. **Si ocurre al ejecutar:** se añade una función `mine_*` y su entrada en `mines[]`. Va con `'D'` si debe detenerse o `'N'` si debe neutralizarse, y opcionalmente con su gemela de compilación. Se usa `volatile` para que GCC no la pliegue al compilar.
4. Se ejecuta el campo. Si sale ROJO, se busca el guardián (opción de GCC, veneno, envoltorio en `ultra_guard.h`, opción del kernel) y se repite hasta que deje de serlo. Nunca se oculta.
