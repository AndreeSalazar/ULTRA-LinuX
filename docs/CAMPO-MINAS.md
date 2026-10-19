# Campo de minas de C

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
