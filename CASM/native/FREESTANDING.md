# Native fuera de Linux: modulo x86-64 v1

Native ahora tiene un adaptador de salida sin servicios Linux. El compilador
autocompilado se ejecuta en Linux y produce su ELF auditado; `freestanding.py`
verifica ese ELF y reemplaza todo su runtime Linux antes de publicar el modulo.
El frontend y los bytes de las funciones no cambian. Este paso no porta el
compilador anfitrion ni sustituye el perfil anterior metal64/FAGRAW64.

```bash
bash CASM/compilar-native.sh CASM/native/ejemplo-puro.casm
# Usar program.elf del directorio run-* indicado por el publicador:
python3 CASM/native/freestanding.py build/CASM/native/run-XXXX/program.elf calculo.module.json
```

El ejemplo devuelve `20` en RAX; no imprime ni termina el OS anfitrion.

La salida contiene HEX de texto y datos, direcciones, permisos, trampas y huellas.
`provenance_elf_hex` conserva el ELF original exclusivamente para repetir su
auditoria y derivar de nuevo el resultado; **el cargador no debe mapear ni ejecutar
esa procedencia**. Solo carga `segments`. `validate()` rechaza cualquier diferencia
respecto al modulo derivado, incluyendo permisos, direcciones, bytes y campos extra.
SHA-256 comprueba integridad; no autentica al autor de un archivo recibido.

## Contrato con un OS propio

1. CPU en long mode x86-64; DF limpio. Mapear el texto en `0x400000` como RX.
2. Reservar el segmento de datos en `0x10000000`, con `8454144` bytes RW/NX,
   inicialmente a cero, y copiar sus bytes inicializados. No compartir esa region
   entre instancias. Las direcciones son fijas: no hay reubicacion/PIE.
3. Proporcionar al menos 1 MiB de pila RW/NX y alineacion de llamada SysV x86-64:
   RSP modulo 16 debe ser 8 al entrar. El adaptador conserva RBX, RBP, R12-R15
   y RSP; devuelve `u64` en RAX. El ABI interno CASM sigue siendo privado.
4. Invocar `entry` por llamada, no saltar sin direccion de retorno. No hay argc,
   argv, libc, syscalls Linux, interrupcion software de servicios ni hipervisor.
5. Instalar un manejador de excepcion #UD. Overflow, division por cero, indices
   y profundidad terminan en UD2 en las direcciones de `trap_addresses`.
   El OS debe terminar/aislar esa instancia; **no** continuar despues del fallo.
6. No ejecutar simultaneamente ni reentrar en una instancia: globals y contador
   de llamadas son mutables. Una segunda llamada conserva sus globals;
   para reiniciar, volver a inicializar datos desde la imagen y ceros.

El modulo conserva funciones, `u64`, if/while, arrays y guardas de Native.
Rechaza `print`, `panic`, `input`, `output` y `hexout`, incluso en funciones no
alcanzadas. No convierte silenciosamente una llamada Linux en un servicio BMO-X.
El runtime eliminado se rellena con INT3; las cuatro trampas admitidas usan UD2.
El punto de entrada es un stub nuevo; el encabezado ELF anterior no se ejecuta.

El cargador propio debe imponer los permisos y el aislamiento del proceso.
No conviene estrenar este banco en ring 0: una #UD sin manejar puede detener el
kernel. No es un kernel arrancable, una imagen EFI ni un cargador universal.

## Caza de errores y prueba

```bash
# Usuario sin privilegios dentro del taller QEMU:
python3 CASM/native/test.py
python3 CASM/native/test_freestanding.py
```

El segundo banco compara 64 calculos deterministas con Python, comprueba u64
maximo y funciones/buffers, y exige SIGILL en nueve casos de trampas. Ejecuta cada
modulo en un proceso distinto, con MAP_FIXED_NOREPLACE, texto RX y datos RW/NX.
Son 75 ejecuciones, seis rechazos de servicios y 12 mutaciones del contrato.
El arnes Linux usa mmap/mprotect para representar el contrato; el modulo no los
invoca. Esto es una prueba de ejecucion bajo QEMU/Linux, **no un arranque bare metal**.

No hay punteros generales, lifetime dinamico ni C/C++ completo en este perfil.
Los rechazos/trampas cubren el contrato definido, no todos los UB posibles ni
una prueba formal de correccion. El verificador sigue limitado a plantillas
cerradas. No se modifica el kernel ULTRA ni se atribuye mejor rendimiento sin medir.

## Siguiente tramo para BMO-X

- Cargador BMO-X que compruebe el contrato y pruebe retorno/#UD en un proceso aislado.
- ABI versionado de servicios con capacidades, buffers y errores explicitos;
  mantener el modo puro sin servicios como caso base reproducible.
- Backend anfitrion para lectura/escritura en ese ABI: solo entonces portar
  el compilador CASM y repetir las generaciones sin Linux presente.
- Despues, tipos adicionales y memoria dinamica con contratos de ownership;
  ampliar a la vez emisor, verificador y casos adversos.
