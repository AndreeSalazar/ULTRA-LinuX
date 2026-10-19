# Campo de minas de C

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
