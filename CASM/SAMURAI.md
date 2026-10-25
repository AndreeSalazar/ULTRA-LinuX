# CASM, el samurái de la CPU

Inspirado en el plan [INTI SAMURAI](https://github.com/AndreeSalazar/BMO-X-x86-64/blob/main/docs/plan/PLAN_INTI_SAMURAI.md)
de BMO-X: *emitir a la CPU, preciso, y nada más; cortar sin piedad lo que no es suyo*.

CASM toma su forma de C para leerse fácil, pero **no es C**. Su única tarea es
**preparar y exportar `.hex` exacto para x86-64**. Lo que no sabe hacer seguro, lo
**corta**: no se emite nada, o no se publica el paquete.

## Lo que es de CASM (se queda)

| Pieza | Dónde | Por qué es del samurái |
|---|---|---|
| El frontend con sus reglas | `src/frontend.c` | Rechaza antes de emitir: es lo que hace que cortar rápido no sea cortar a ciegas |
| La IR acotada | `src/casm.h` | Lo único que el codificador recibe |
| El codificador x86-64 propio | `src/emitter.c` | El único que nombra la máquina; ni LLVM, ni ensamblador, ni linker generan el destino |
| El verificador separado | `verify.py` | Reconstruye cada byte permitido y solo entonces publica el paquete |
| El paquete FAGRAW64 / `metal64` | `INICIO.X86` | Lo que llega a la CPU por el puente EFI |
| Las katanas | Las reglas `RED [...]` | Dónde corta cada regla, con su caso de prueba |

## Lo que no entra (cortado por diseño)

- **Otra arquitectura.** Solo x86-64; sin hardware donde probar, una regla no entra.
- **Otro generador del destino.** Ni LLVM, ni ensamblador, ni linker: los bytes son del
  codificador de CASM y los comprueba su verificador.
- **Lo que es de aplicación** (objetos, montón, ventanas, sonido). Como en BMO-X, eso es
  de otra capa (allí, TITAN++); CASM habla con la CPU.
- **«Ser compatible con todo C».** CASM adopta de C lo que puede hacer seguro y lo dice
  en [LANGUAGE.md](LANGUAGE.md); lo demás se rechaza con su regla.
- **El compilador de CASM con otro compilador que no sea GCC** (`construir.sh` se niega).

## CASM 0.2: enteros exactos

`const uint64_t grande = 4000000000 * 4;` y `metal_put_int(grande);` ([LANGUAGE.md](LANGUAGE.md#exact-integers-casm-02)).

- **Tipos con la anchura de la CPU:** `uint8_t`…`uint64_t` e `int8_t`…`int64_t`.
- **Nada da la vuelta.** Cada literal, cada referencia y **cada paso intermedio** debe
  caber en el tipo declarado, también en los sin signo (en C dan la vuelta; aquí cortan).
- **De COBOL** se toma el número exacto de tamaño declarado y su `DISPLAY`
  (`metal_put_int`). **No** se toma su truncado silencioso cuando falta `ON SIZE ERROR`:
  en CASM el corte es obligatorio.
- **Sin trampas de C:** `010` no es octal (se rechaza), la división trunca hacia cero,
  `x % 0` corta y `MIN % -1` es exactamente 0.
- **Dos implementaciones que deben coincidir:** el compilador (C, GCC) calcula cada valor;
  el verificador (Python, enteros sin límite) lo **recalcula** desde la forma postfija de
  la IR con las mismas reglas. Si no da lo mismo, no se publica el paquete. Se probaron
  los bordes: `UINT64_MAX`, `INT64_MIN`, `0xFF`, `MIN % -1 = 0`, `-7 / 2 = -3`,
  `-7 % 2 = -1` y `7 % -2 = 1`.
- **El `.hex`:** `metal_put_int(x)` es el mismo `LEA`/`CALL` de 11 bytes que
  `metal_puts`, apuntando a las cifras exactas de `x`. Aún no hay código de trampa:
  CASM no tiene valores en ejecución, así que el compilador lo calcula todo antes. Las
  trampas en el binario llegan con el primer valor de ejecución.

### El museo en CASM 0.2

| Ficha del museo | En C | En CASM 0.2 |
|---|---|---|
| UB-001 el entero con signo que se sale | UB | Imposible: `INT-OVERFLOW` corta al compilar |
| NOUB-001 unsigned que da la vuelta | Definido (da la vuelta) | Más estricto: también corta |
| UB-002 división por cero y `MIN / -1` | UB | Imposible: `INT-DIV-ZERO` / `INT-OVERFLOW` |
| UB-003 desplazar fuera del ancho | UB | No existe `<<`: no se puede escribir |
| UB-016 flotante que no cabe en int | UB | No hay flotantes |
| UB-020 leer una local sin inicializar | UB | Todo entero es `const` y nace inicializado |

## Las katanas hoy (medidas por el SUPER GRAFO)

El panel **Taller CASM** de ULTRA Carpetas lee las fuentes de CASM y saca cada regla
que corta. En `src/` y `verify.py` hay **61 reglas que cortan** (48 en CASM 0.1 y 13
nuevas con los enteros), repartidas entre anfitrión, frontend, codificador y
verificador; algunas, como `INT-OVERFLOW`, cortan en dos etapas, y cada etapa se prueba
por separado.

**Las 61 tienen su caso aislado** en `test.py` (108 casos en total, contando los que
deben aceptarse con su valor exacto):

- **Frontend y codificador:** una fuente mínima que debe cortar con esa regla (27).
- **Anfitrión:** fuente que no existe (`HOST-READ`), salida que es una carpeta
  (`HOST-WRITE`) y memoria limitada a 256 KiB con `RLIMIT_DATA` (`HOST-MEMORY`).
- **Verificador:** se toma una emisión buena, se altera **una sola cosa** (un byte, un
  desplazamiento, un nombre, la ABI…) y debe cortar con **su** regla. También por línea
  de comandos, sin publicar paquete (`RED [VERIFY] CODE-OFFSET`), más sus dos reglas
  como programa (`OUTPUT-ALIAS` y un archivo que falta).

Ningún caso vale con «algo falló»: tiene que cortar la katana que nombra. El Taller
marca «sin caso aislado» cualquier regla nueva hasta que tenga el suyo.

## El camino

1. ~~**Un caso aislado por katana.**~~ Hecho: 48 de 48. Cada regla nueva entra con su caso.
2. **Katanas en el binario** (empezado, en [traspaso](traspaso/LEEME.md)), como la sección `BKAT` de INTI. Cuando CASM tenga aritmética,
   cada trampa emitida (desbordamiento, división por cero) se declara en una tabla del
   paquete. El verificador comprueba que en ese desplazamiento hay de verdad ese bloque
   de trampa. *Declarar sin comprobar es propaganda; comprobar sin declarar es adivinar.*
3. **Crecer el lenguaje.** Hecho el primer tramo: enteros exactos con aritmética
   comprobada (CASM 0.2). Siguen el primer valor en ejecución (con sus trampas), las
   variables, `if`, bucles y funciones. Cada pieza entra con sus katanas, y el museo dice
   qué UB de C deja imposible.
4. **Autoalojamiento.** CASM prepara y exporta su propio compilador. La semilla es el
   `.hex` escrito a mano, que una persona puede revisar, y GCC queda como testigo, no
   como dependencia.

## El SUPER GRAFO como lupa

- **Taller CASM** (icono de la katana):
  - la cadena fuente → frontend → IR → codificador → HEX → verificador → `INICIO.X86`,
    con el paso donde cortó la última compilación en rojo;
  - las katanas con su sitio exacto;
  - las compilaciones con sus bytes, su huella y sus avisos.
- **Editor:** junto a cada línea de un `.casm`, sus bytes exactos y su desplazamiento.
  Solo aparecen si hay una compilación **verificada de ese texto exacto**: no se enseñan
  bytes de otra versión.
- **Play** en cada `.casm`: compila con `compilar.sh` (sin shell). Cada compilación
  queda en `build/CASM/run-*` con su `registro.txt`.
