# ASM: laboratorio por arquitectura

Las [fichas ASM del museo](../Museo/README.md#asm) comienzan con x86-64 Linux,
System V AMD64 y GNU extended asm. No trasladar los contratos a Windows x64,
AArch64 o interrupciones sin revisar sus especificaciones.

Se estudian cuatro capas: instrucciones/excepciones de la ISA, llamadas y
registros de la ABI, operands/clobbers de GCC y sincronizacion del algoritmo.
El inline ASM tambien puede romper las suposiciones del codigo C/C++ que lo
contiene. Que el ensamblador acepte el texto no valida esas cuatro capas.

Los seis casos iniciales del museo siguen siendo fichas conceptuales. Ahora
[laboratorio](laboratorio) contiene un puente C/ASM ejecutable y controles de
registros y alineacion, verificados en QEMU en cuatro perfiles. No instala
ensamblador en la imagen de produccion ni ejecuta casos invalidos sobre discos
personales. Ver [resultado, contrato y limites](../docs/VIDA-UTIL-Y-ASM.md).

El primer banco registra toolchain y separa controles validos e invalidos. UBSan/ASan
no instrumentan automaticamente el interior de una rutina ASM arbitraria.

Fuentes: [GCC extended asm](https://gcc.gnu.org/onlinedocs/gcc/Extended-Asm.html),
[System V AMD64 psABI](https://gitlab.com/x86-psABIs/x86-64-ABI),
[manuales de arquitectura Intel](https://www.intel.com/content/www/us/en/developer/articles/technical/intel-sdm.html).
