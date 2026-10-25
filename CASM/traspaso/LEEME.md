# Traspaso: CASM 0.3 (primer valor en ejecución y trampas en el binario)

Estado: **trabajo en curso, fuera del árbol principal.** CASM en `CASM/` es la 0.2
(enteros exactos calculados al compilar, 108/108 casos). La 0.3 empezada está en
[`casm-0.3-wip.patch`](casm-0.3-wip.patch), que se aplica limpio sobre la 0.2:

```bash
git apply CASM/traspaso/casm-0.3-wip.patch
CASM/construir.sh            # compila sin avisos (-Werror, -fanalyzer)
```

## Objetivo de la 0.3

- `metal_tics()`: lee el contador de tiempo de la CPU (`rdtsc`). Es el primer valor que
  el compilador **no** puede calcular, así que se emite código x86-64 de verdad.
- La aritmética sobre valores de ejecución usa las mismas reglas que la 0.2 (nada da
  la vuelta). Cada operación que puede desbordar emite un salto a un **bloque de
  trampa**, y la IR declara una **tabla de trampas** (regla, desplazamiento, longitud,
  línea), como la sección `BKAT` de INTI. El verificador debe comprobar cada entrada
  contra los bytes: *declarar sin comprobar es propaganda*.

## Lo que hace el parche

- **Alcance deliberadamente pequeño:** enteros de ejecución solo `uint64_t`, una sola
  operación por expresión (`+`, `-` o `*`), operandos `metal_tics()`, otro entero o un
  literal. División en ejecución: aún no (katana propia).
- **Frontend** (`frontend.c`): decide si una declaración es de ejecución mirando el
  inicializador por adelantado. Si lo es, genera un flujo ordenado de operaciones
  (`casm.h`: `CasmOp`). `metal_puts` y `metal_put_int` también entran en ese flujo.
- **Emisor** (`emitter.c`): si el programa no tiene valores de ejecución, emite
  **exactamente como la 0.2** (el saludo sigue idéntico al `.hex` de oro). Si los tiene,
  emite código con un marco propio:
  - `r13` es la base del marco; se conservan RBP, R15 y la pila, como pide el puente;
  - la pila queda alineada a 16 en cada llamada a `puts`;
  - el bloque de trampa detiene la CPU;
  - la IR pasa al **esquema 3**, con `Frame`, `Ops` y `Traps`.
- **Comprobado:** se compiló un ejemplo y se desensambló con `objdump`. Las instrucciones
  son las previstas: prólogo, `rdtsc`, suma con salto a la trampa, conversión a decimal
  con `rdx` a cero antes de `div`, y `puts`.
- **No comprobado:** no se ejecutó en QEMU ni en el Ryzen.

## Lo que falta (en este orden)

1. **`verify.py`, esquema 3.** El parche solo añade la rama que llama a
   `verify_runtime(source, body, ir)`, que **aún no existe**. Hay que escribirla
   reconstruyendo cada byte desde `Ops` con las mismas plantillas del emisor. Debe
   comprobar:
   - `Frame` según el número de enteros de ejecución;
   - los datos (ascii y decimales) igual que en la 0.2;
   - que cada entrada de `Traps` apunta a un salto real al bloque de trampa;
   - que todo el cuerpo coincide.

   **Hasta entonces, ningún programa de la 0.3 debe publicar paquete.**
2. **Casos aislados de las katanas nuevas.** El Taller del SUPER GRAFO exige uno por
   regla, y la prueba del núcleo falla si falta alguno. Las reglas nuevas son:
   - `INT-RUNTIME-TYPE` (no es `uint64_t`);
   - `INT-RUNTIME-DIV` (división en ejecución);
   - `EXPR-RUNTIME` (más de una operación);
   - `OPS-LIMIT`. Con los límites actuales (128 enteros y 128 llamadas, como mucho 768
     operaciones de 1024) **no se puede alcanzar**: quitarla o justificarla.
   - Las del verificador del esquema 3 (`TRAP-TABLE`, `TRAP-SITE`, `FRAME`…), cada una
     con su mutación «una sola cosa cambiada».
3. **SUPER GRAFO.** `carpetas-nucleo/src/casm.rs` acepta los esquemas 1 y 2: añadir el 3
   (mostrar las operaciones por línea y la tabla de trampas en el Taller).
4. **Prueba en QEMU/OVMF** con el puente de TALK-CPU antes de cualquier prueba física.

## Reglas que no se negocian

- Solo GCC (`construir.sh` se niega con otro compilador) y solo x86-64 `metal64`.
- Los programas sin `metal_tics` deben seguir saliendo idénticos byte a byte: el
  saludo de oro (`b3fd9814…4e4b33`) es la prueba.
- Ninguna regla nueva sin su caso aislado; ningún caso vale con «algo falló», tiene que
  cortar la regla que nombra.
