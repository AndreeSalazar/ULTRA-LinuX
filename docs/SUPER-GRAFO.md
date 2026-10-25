# SUPER GRAFO: decisiones y camino

El SUPER GRAFO (SG) de ULTRA Carpetas creció por capas:

- **1D:** gestionar archivos.
- **2D:** nodos.
- **3D:** la Ciudad.

La siguiente capa es que **se vuelva inteligente**: que cace con criterio y que se
pueda medir cuánto acierta. Este documento fija con qué criterio. El detalle de lo
que ya hace está en [ULTRA-CARPETAS.md](ULTRA-CARPETAS.md#super-grafo-el-museo-en-tu-código-y-el-fallo-conectado).

## El triángulo: Seguridad, Rendimiento y Elegancia

Los tres importan, pero no son del mismo tipo y no se intercambian:

| Eje | Qué es | Cómo se juzga |
|---|---|---|
| **Seguridad** | Una regla: la llave cierra o no cierra | Sí o no. Nunca se cambia por rendimiento: un programa rápido que se puede romper no cumple su tarea |
| **Rendimiento** | El motor y a dónde va | Con un presupuesto medido (como los carriles con semáforo). Sin medición no hay afirmación |
| **Elegancia** | Simplificar la vista y la lectura | Que una persona lo lea y revise fácil, sin perder los otros dos |

Orden: primero seguro; luego rápido sin romper lo seguro; luego simple sin perder ninguno.

## Alcance: C con GCC en x86-64

- **Compilador: GCC.** Es el compilador con el que se construye este Linux, y el museo
  ya fija el dialecto «C17 / GCC». Las fichas de ASM son de GNU extended asm, que es
  sintaxis de GCC. Un UB se juzga contra un compilador concreto (qué avisa, qué
  optimiza y qué instrumenta): la caza y la evidencia usan el mismo.
- **Arquitectura: x86-64** (System V AMD64). No hay hardware RISC-V ni ARM64 donde
  probar; una regla que no se puede comprobar no entra. Si algún día lo hay, se añade
  como alcance nuevo, con sus propias fichas.
- **LLVM: fuera del alcance del cazador.** Sus herramientas (clang-tidy, Clang Static
  Analyzer, Infer) tienen equivalente en GCC para lo que necesita el SG. Nota: el código
  Rust de este repositorio sí pasa por LLVM, porque es el backend de `rustc`. Eso no
  cambia el cazador de C, que es de GCC.

## Referencias (todas sin LLVM)

De ellas se toman ideas y casos de prueba, no código: cada una tiene su licencia.

| Referencia | Para qué |
|---|---|
| **Juliet Test Suite (NIST SARD)** | El examen: casos de C/C++ por CWE con versión mala y buena (el mismo formato `mal`/`bien` del museo). Se compila con GCC |
| **SEI CERT C** | Justificar cada ficha de seguridad (el museo ya cita STR37-C) |
| **GCC `-fanalyzer`** | Segunda opinión que sigue valores por el programa, para las sospechas «revisar» |
| **GCC `-fsanitize=undefined,address`** | La prueba en el laboratorio: confirma que una sospecha es UB de verdad |
| **Coccinelle, sparse, smatch** | Cómo caza bugs el kernel: patrones por la forma del C, no por texto |

## Camino

1. **Examen con Juliet compilado con GCC.** El SG saca su nota por CWE: cuánto caza y
   cuántas falsas alarmas da. Cada mejora tiene que subir la caza sin subir las
   falsas alarmas.
2. **Botón «probar esta ficha».** Compila el caso `mal`/`bien` con GCC, UBSan y ASan en
   el laboratorio (nunca sobre discos personales). Guarda la versión de GCC, los flags y
   el binario como evidencia, como ya exige el museo.
3. **`-fanalyzer` como segunda opinión** para lo que el SG marca «revisar».
4. **Cada ficha con su eje** (Seguridad, Rendimiento o Elegancia) y el SG coloreado por
   eje en el grafo y en la Ciudad. **Hecho:** 42 fichas con su eje (35 Seguridad, 3
   Rendimiento, 4 Elegancia; hoy 43 con E-003, la de los ciclos), cuatro de ellas nuevas (R-001, R-002, E-001, E-002). Solo
   la Seguridad cuenta como fallo. Detalle y medidas en
   [ULTRA-CARPETAS.md](ULTRA-CARPETAS.md#el-triángulo-seguridad-rendimiento-y-elegancia).
5. **Combinar: la selección grupal.** Varios edificios (Ctrl+clic o Shift+arrastrar en la
   Ciudad) y el panel dibuja cómo se combinan: directas, lo que comparten, quién los junta
   y los sueltos, con cables también sobre la Ciudad. **Hecho:** 0,34 ms con 42 miembros
   en Chromium. Detalle en
   [ULTRA-CARPETAS.md](ULTRA-CARPETAS.md#combinar-la-selección-grupal-en-vivo).

## Cómo lo usa una IA: preguntar, no volcar

Con miles de millones de líneas nadie lee el código entero, tampoco una IA. El reparto:

| Pieza | Papel |
|---|---|
| **SUPER GRAFO** | El músculo: recorre todo, cuenta, conecta y ordena, en su hilo y sin gastar tokens |
| **Respuesta pequeña (JSON o texto)** | Lo exacto: rutas completas, líneas, cifras y su motivo |
| **Imagen** | La forma: patrones espaciales («todo lo rojo está en `allocator`») |
| **La IA** | Pregunta, decide y va al código solo donde importa |

**Regla:** cada respuesta tiene un **tamaño acotado**, sea cual sea el tamaño del proyecto.

1. Primero las cifras y lo peor: «12 ciclos; los 5 más grandes son…».
2. Bajar solo si hace falta: «los motivos del ciclo 3».
3. Nunca una lista sin tope: si hay más, se dice cuántos quedan (como «comparten 26»
   enseñando 8).

Cálculo de hoy (estimado), con el camino de 12 pasos en Chromium:

- Una captura de 1600 × 1000 son unos 1 500 tokens fijos, pero los nombres salen
  cortados y solo se leen 2 pasos.
- El texto completo de los 12 pasos son unos 600 tokens.
- Un resumen acotado son unos 250.
- Lo caro fueron las repeticiones: unas 10 capturas (unos 15 000 tokens) contra una
  consulta.

Volcar todo en JSON costaría más que cualquier captura: lo que ahorra es que el SUPER
GRAFO conteste la pregunta, no el formato.

**Orden de madurez:**

1. ~~**Ciclos.**~~ Hecho.
2. **Consultas sin ventana** (`--consulta camino|ciclos|peso … --json`, con tope).
3. **Comparar con `gcc -M`**, para medir cuánto acierta el mapa léxico.
4. **Índice incremental en disco**, el paso hacia los miles de millones de líneas.

## Siguientes ideas para educar al SUPER GRAFO

Hecho también: **del grupo a los nodos y al editor**. Lo que señalan los archivos del
grupo, en el orden para arreglar, con lo que enseña cada ficha; «Siguiente señal» abre el
editor en cada una; la cadena de `#include` hasta la causa; y «Ver en el grafo» con los
cables del grupo. Detalle en
[ULTRA-CARPETAS.md](ULTRA-CARPETAS.md#del-grupo-a-los-nodos-y-al-editor-lo-que-señalan-cómo-arreglarlo-y-por-dónde-llega).
También **aislar el grupo entero** (botón en Combinar, o **A** con varios seleccionados).
Siguiente: las **consultas sin ventana**.

Ordenadas por lo que más enseñan con menos riesgo. Ninguna toca CASM.

1. ~~**Camino más corto entre dos elegidos.**~~ **Hecho:** con motivos (la línea de cada
   paso), la onda de recompilación animada y los pasos imprescindibles (dónde cortar).
   Detalle en [ULTRA-CARPETAS.md](ULTRA-CARPETAS.md#el-camino-entre-dos-elegidos-motivos-onda-y-dónde-cortar).
2. ~~**Ciclos.**~~ **Hecho:** Tarjan, la vuelta más corta con sus motivos, dónde cortar,
   el anillo animado y la ficha E-003. Además el SUPER GRAFO aprendió a leer los `#if`:
   los 2 ciclos de Chromium `base/` solo existen al compilar para Windows. Detalle en
   [ULTRA-CARPETAS.md](ULTRA-CARPETAS.md#ciclos-de-cabeceras-e-003-eje-elegancia).
3. **Peso del cambio.** Si cambia este grupo, cuántos archivos hay que recompilar (quien
   los usa, directa o indirectamente): una cifra de Rendimiento medible con GCC.
4. **Fronteras de carpeta.** Contar cuántas dependencias cruzan de un distrito a otro: un
   módulo que todos usan, o uno que usa a todos.
5. **Grupos guardados.** Ponerle nombre a un grupo («red», «arranque») y volver a él.


## CASM: el lenguaje que exporta `.hex`

CASM vive aislado en [`CASM/`](../CASM/README.md): construido solo con GCC, solo para
x86-64 y con su verificador aparte. Su rumbo como samurái de la CPU está en
[CASM/SAMURAI.md](../CASM/SAMURAI.md). El SUPER GRAFO es su lupa: muestra cómo nace cada
byte, dónde corta cada katana y qué compilaciones salieron verdes o rojas
([Taller CASM](ULTRA-CARPETAS.md#taller-casm-cómo-nace-el-hex-super-grafo)).
