# CASM en ULTRA LinuX (aislado, GCC, x86-64)

CASM es el lenguaje propio que emite x86-64 **exacto** a `.hex`: fuente con forma de C,
IR acotada, codificador propio, verificador separado y paquete FAGRAW64 para el puente
EFI (`metal64`). Esta carpeta es una **copia aislada** para construir BMO-X2 desde este
Linux. La definicion del lenguaje esta en [LANGUAGE.md](LANGUAGE.md).

## Origen

Copiado sin cambios de [TALK-CPU-x86-64](https://github.com/AndreeSalazar/TALK-CPU-x86-64),
commit `b5b8e88`, del mismo autor. No se modifico ni se envio nada a ese repositorio.

| Archivo | SHA-256 en el origen |
|---|---|
| src/casm.h | 07cb3fa40ab6baa0dcfd590b7180c85a3f3c1d618a868353c936d6be99677302 |
| src/frontend.c | 7ded0c98400b1e411644526971cc4df31f9238c566fb378be11c57c28bf4d2e2 |
| src/emitter.c | 416c2cfaf0f2c68a45def3656a4872308fc4ac69b883f5dd69ff04f2dcb95ea6 |
| src/main.c | 47dc776884a682cac3e4ffc259cbb25bd25faec93be2680ca401a3ee287796a1 |
| verify.py | 1b9eb76530dfacdeb57f84b3755c810c17201b9a97ec701e56ed67ed08b5966e |
| LANGUAGE.md | 01be4029a26fa05c5f0009a9bf8d952e5af6800d3ed53b3bfcc6fcc295193b94 |
| examples/hello.casm | 63bb72f6eaa4722b7f91133ab8b7635d1e55290d276673ca42ebf2bc26711547 |

Solo `test.py` cambia una linea: el compilador es `build/CASM/host/casmc` (Linux) en vez
de `casmc.exe`. Los scripts de PowerShell y la prueba QEMU del puente no se copiaron.

## Aislado

- **Solo GCC.** `construir.sh` se niega si `CC` no es GCC (sin LLVM). Flags: `-std=c11
  -Wall -Wextra -Werror -pedantic -O2 -fanalyzer`, mas `_FORTIFY_SOURCE=3`, stack
  protector, CET, PIE y RELRO. Guarda version, flags y huellas en `build/CASM/host/toolchain.txt`.
- **Solo x86-64 / metal64.** Es el unico destino del codificador y del verificador.
- **Sin dependencias externas:** C11 con la libc y Python 3 con su biblioteca estandar.
- **Solo escribe en `build/CASM/`** (ignorado por git). No toca discos, entradas de
  arranque ni paquetes instalados.

## Uso

```bash
CASM/probar.sh                      # 39 casos + paquete de oro (identico al .hex a mano)
CASM/compilar.sh                    # examples/hello.casm -> build/CASM/run-*/INICIO.X86
CASM/compilar.sh mi-programa.casm
```

Comprobado aqui con GCC 13.3: 39/39 casos, y `hello.x86` con SHA-256
`b3fd9814bcce6881f9cf3ad10b299d759f4f5b801a6526c59131a4d1c84e4b33`, el mismo que las
evidencias de TALK-CPU para el `.hex` escrito a mano.
