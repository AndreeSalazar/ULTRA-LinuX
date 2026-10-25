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
   Rendimiento, 4 Elegancia), cuatro de ellas nuevas (R-001, R-002, E-001, E-002). Solo
   la Seguridad cuenta como fallo. Detalle y medidas en
   [ULTRA-CARPETAS.md](ULTRA-CARPETAS.md#el-triángulo-seguridad-rendimiento-y-elegancia).

## CASM: el lenguaje que exporta `.hex`

CASM vive aislado en [`CASM/`](../CASM/README.md): construido solo con GCC, solo para
x86-64 y con su verificador aparte. Su rumbo como samurái de la CPU está en
[CASM/SAMURAI.md](../CASM/SAMURAI.md). El SUPER GRAFO es su lupa: muestra cómo nace cada
byte, dónde corta cada katana y qué compilaciones salieron verdes o rojas
([Taller CASM](ULTRA-CARPETAS.md#taller-casm-cómo-nace-el-hex-super-grafo)).
