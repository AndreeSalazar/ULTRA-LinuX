# CASM Native 0.1: autocompilacion x86-64 en Linux

**Este perfil ya se autocompila.** `compiler.casm` contiene el analizador, tablas de
simbolos, generador de instrucciones y escritor ELF del propio compilador, escritos
en CASM Native. Emite un ejecutable ELF64 x86-64 y su representacion `.hex` exacta.
No lee ni copia su ejecutable: su unica entrada de compilacion es el texto fuente.

El perfil **metal64 0.2** de `CASM/src/*.c` conserva su frontend C. Native no afirma
compilar ese C, ni todo C/C++, ni los programas metal64 de la gramatica anterior.
Es la primera ruta de autocompilacion acordada: Linux primero, metal64 despues.

Ahora existe ademas una [salida freestanding x86-64](FREESTANDING.md): modulos
de calculo y buffers sin runtime Linux. El compilador anfitrion sigue en Linux;
no se declara autocompilado en BMO-X ni arrancable ese nuevo formato.

## Prueba de autocompilacion

`test.py` construye la misma fuente CASM como semilla externa C compatible con GCC
y con Clang. `seed.py` solo prepara esas semillas; no participa en las generaciones
nativas. Cada semilla produce una generacion nativa; esta produce otra desde la
misma fuente y esta vuelve a compilarla. Las tres son identicas byte por byte y
ambos caminos llegan al mismo ELF/HEX. Tambien se comprueban semillas ASan/UBSan.

`sealed.sh`, exclusivamente en el taller QEMU, crea una raiz que solo contiene el
compilador, las fuentes y un directorio de salida. Lo ejecuta como UID 1000, sin
interprete dinamico, libc, GCC, Clang, Python, ensamblador ni enlazador. Alli produce
dos generaciones identicas, compila una fuente diferente y ejecuta su resultado.
Los privilegios de root solo preparan el chroot del banco; el compilador no los usa.

Evidencias: [`evidencias/casm-native/2026-10-25`](../../evidencias/casm-native/2026-10-25/).

## Uso dentro de Linux

El kit portable conserva esta estructura y añade en `build/CASM/native/` el
compilador autocompilado y `self-host.json`. No necesita GCC/Clang para compilar:

```bash
bash CASM/compilar-native.sh CASM/native/ejemplo.casm
```

El publicador usa Python para comprobar las huellas, el ELF y sus instrucciones.
Guarda un directorio nuevo `build/CASM/native/run-*`, con fuente congelada,
`registro.txt`, `program.hex`, `program.elf` y `verified-native.json`. Solo publica
`program.elf` y `program.hex` tras pasar el verificador. No ejecuta el
programa compilado. El ejemplo, al ejecutarlo, imprime `50`.

El compilador tambien funciona solo, sin Python (la verificacion externa se hace
despues; elegir nombres de salida nuevos):

```bash
build/CASM/native/casmc-native.elf CASM/native/compiler.casm generacion.elf generacion.hex
chmod +x generacion.elf
./generacion.elf CASM/native/ejemplo.casm ejemplo.elf ejemplo.hex
```

Para rehacer el banco desde las fuentes dentro de QEMU, como usuario sin privilegios:

```bash
python3 CASM/native/seed.py generar  # solo si se cambio body/runtime y se reviso
python3 CASM/native/test.py
sudo bash CASM/native/sealed.sh
```

`compiler.casm` se genera uniendo las constantes del runtime propio y
`compiler.body.casm`; se guarda y revisa completo en Git. No se genera desde el
binario. `seed.py` comprueba que esas fuentes coinciden antes de construir la semilla.

## Lenguaje implementado

Un unico tipo escalar: `u64`. Es unsigned de 64 bits, con suma, resta,
multiplicacion y desplazamiento izquierdo comprobados; no hay vuelta silenciosa.
Division/modulo por cero y anchuras de desplazamiento de 64 o mas terminan en trampa.
Comparaciones unsigned, `!`, `&`, `|`, `^`, `<<`, `>>` y parentesis. No hay negativos,
promociones, floats, punteros, `&&`/`||`, preprocesador ni FFI.

- Entrada: `u64 casm_main()`. Su retorno es el estado de salida Linux.
- Funciones con cero a ocho parametros `u64`, retorno por valor, llamadas adelantadas
  y recursion limitada. Sin `return` explicito al final, devuelve cero.
- Variables locales inicializadas al comienzo de cada funcion; sin sombreado,
  declaraciones dentro de bloques ni direcciones de variables que puedan escapar.
- `if`/`else`, `while`, bloques, `break`, asignaciones y `return expresion;`.
- Globals escalares y arrays `u64` de capacidad fija, inicializados a cero o con
  literales. Cada acceso comprueba el indice antes de calcular la direccion.
- `print(valor)` imprime decimal; `panic(codigo)` termina el proceso.
- `input(array)` lee argv[1] en las celdas (un byte por celda).
- `output(array,n)` escribe bytes en argv[2]; `hexout(array,n)` escribe HEX en argv[3].
  Longitud y rango de byte se comprueban. Los archivos se crean exclusivamente, sin
  seguir enlaces ni sobrescribir los existentes. No hay ejecucion de comandos externos.

Limites de este perfil: fuente 65536 bytes; nombres 63 bytes; 128 funciones; ocho
parametros; 128 simbolos locales por funcion; 256 globals; 262144 celdas por array;
8 MiB para globals; inicializacion no nula dentro de los primeros 64 KiB del
segmento de datos; 4096 referencias a funciones y 4096 `break`; expresiones anidadas a 16 y
sentencias a 32; pila dinamica de llamadas hasta 128 marcos. ELF completo <=262144
bytes. El publicador limita CPU, archivos, pila y espacio virtual del compilador.

Errores de fuente/sintaxis: 65; nombre ausente: 69. Trampas del programa: overflow
o byte fuera de rango: 70; division por cero: 71; indice/longitud: 72; profundidad:
73; E/S: 74. Las señales Linux conservan su comportamiento normal (incluido SIGPIPE).
No son excepciones generales de C ni una emulacion de todas las semanticas metal64.

## El verificador de HEX/ELF

`verify.py` no confia en que los bytes sean correctos por venir de CASM. Comprueba:

- ELF64 little endian, maquina x86-64, direcciones y tamanos acotados;
- texto RX, datos RW, pila NX; sin interprete dinamico ni dependencias compartidas;
- runtime propio exacto, salvo el desplazamiento comprobado de llamada a la entrada;
- marcos/aridad, acceso a locales y parametros y sus limites;
- guardas de overflow, division, desplazamiento e indices junto a sus instrucciones;
- saltos a fronteras de instrucciones, llamadas a funciones/servicios reconocidos;
- flujo de pila de expresiones, capacidades de escritura de arrays y lectura de
  locales solo despues de inicializarse. No permite syscalls arbitrarios en el codigo
  del usuario ni instrucciones ajenas al conjunto cerrado de plantillas.

El ELF usa direcciones fijas, sin PIE ni propiedad de activacion CET. El runtime es
una plantilla compartida auditada; esto no es una prueba formal de ausencia de bugs
del compilador, una demostracion general de memoria segura ni un verificador de
ASM arbitrario. Los 60 casos y 12 mutaciones cubren el alcance publicado; ampliar el
lenguaje exige ampliar y volver a probar el verificador.
