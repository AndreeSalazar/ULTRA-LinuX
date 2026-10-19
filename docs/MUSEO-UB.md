# Museo viviente de C y sus guardianes

Generado desde `museo-ub.json`; las fichas se estudian y revisan, no se ejecutan desde este documento.

Dialectos: C17 / GCC; las referencias N1570 son del borrador C11 y N3155 trata C23: no intercambiar sus numeraciones.

Coleccion inicial: **24 piezas** (21 de UB y 3 de contraste). Enlaza 26 de las 62 minas actuales; una pieza no equivale a una sola mina.

Los fragmentos son originales y didacticos: algunos presuponen declaraciones o contexto. La alternativa puede ser una pauta de diseno, no un reemplazo completo compilable. Una alternativa valida puede rechazar una operacion imposible para el tipo; no implica obtener un resultado numerico.

## Como leer el museo

- ESTUDIADA / SIN MINA: hay explicacion y referencia; no hay deteccion acreditada.
- VINCULADA: se muestran resultados de casos concretos, sin generalizarlos a toda la familia.
- REVISION: el fixture existe; esta pagina no convierte su presencia en prueba de rechazo.
- Las piezas NOUB distinguen reglas del lenguaje de politicas mas estrictas del proyecto.

No es un catalogo completo de la norma ni un modelo que aprenda permisos automaticamente. Una extension de WG14 o GCC no se convierte por ello en una regla ISO C17. Los estados pertenecen al informe enlazado: generar esta pagina no vuelve a ejecutar las minas ni demuestra que un binario posterior conserve esos resultados.

## Indice

| Pieza | Sala | Clase | Vinculo |
|---|---|---|---|
| [UB-001 — El entero con signo que se sale de su rango](#ub-001) | Aritmetica | UB | VINCULADA |
| [UB-002 — Division y resto tienen dos bordes](#ub-002) | Aritmetica | UB | VINCULADA |
| [UB-003 — Desplazar fuera del ancho](#ub-003) | Aritmetica | UB | VINCULADA |
| [UB-004 — El vecino no pertenece al arreglo](#ub-004) | Memoria | UB | VINCULADA |
| [UB-005 — La direccion sobrevive, el objeto no](#ub-005) | Vida util | UB | VINCULADA |
| [UB-006 — Una variable que ya salio de escena](#ub-006) | Vida util | UB | VINCULADA |
| [UB-007 — El alias de otro tipo](#ub-007) | Tipos y evaluacion | UB | VINCULADA |
| [UB-008 — La direccion no esta alineada](#ub-008) | Tipos y evaluacion | UB | VINCULADA |
| [UB-009 — Dos modificaciones sin secuenciar](#ub-009) | Tipos y evaluacion | UB | VINCULADA |
| [UB-010 — La promesa restrict incumplida](#ub-010) | Contratos | UB | ESTUDIADA / SIN MINA |
| [UB-011 — El char negativo de ctype](#ub-011) | Biblioteca | UB | ESTUDIADA / SIN MINA |
| [UB-012 — va_arg pide el tipo equivocado](#ub-012) | Biblioteca | UB | ESTUDIADA / SIN MINA |
| [UB-013 — El formato no coincide con el argumento](#ub-013) | Biblioteca | UB | ESTUDIADA / SIN MINA |
| [UB-014 — longjmp vuelve a un valor indeterminado](#ub-014) | Vida util | UB | ESTUDIADA / SIN MINA |
| [UB-015 — NULL no designa un int](#ub-015) | Memoria | UB | VINCULADA |
| [UB-016 — El flotante que no cabe en int](#ub-016) | Aritmetica | UB | VINCULADA |
| [UB-017 — strlen sigue buscando el cero](#ub-017) | Biblioteca | UB | VINCULADA |
| [UB-018 — memcpy no permite solapamiento](#ub-018) | Biblioteca | UB | VINCULADA |
| [UB-019 — Comparar direcciones de reservas distintas](#ub-019) | Memoria | UB | VINCULADA |
| [UB-020 — Leer una local sin inicializar](#ub-020) | Tipos y evaluacion | UB | VINCULADA |
| [UB-021 — volatile no sincroniza hilos](#ub-021) | Concurrencia | UB | VINCULADA |
| [NOUB-001 — Unsigned que da la vuelta](#noub-001) | No todo es UB | DEFINIDO | VINCULADA |
| [NOUB-002 — Orden de llamadas no especificado](#noub-002) | No todo es UB | NO_ESPECIFICADO | ESTUDIADA / SIN MINA |
| [NOUB-003 — Desplazamiento derecho de un negativo en C17](#noub-003) | No todo es UB | IMPLEMENTACION | ESTUDIADA / SIN MINA |

## UB-001

**El entero con signo que se sale de su rango** — UB

La suma matematica no cabe en int.

Fragmento de estudio (ver las precondiciones y la clase de la pieza):

```c
int x = INT_MAX; x = x + 1;
```

Alternativa o pauta valida:

```c
int x = ULTRA_ADD(INT_MAX, 1); /* rechaza antes de desbordar */
```

Regla del guardian: UBSan y ULTRA_ADD comprueban casos de desbordamiento.

Limite: La API explicita no transforma operadores escritos fuera de ella.

Referencia: [WG14 N1570: borrador C11, anexo J.2 informativo y clausulas del lenguaje](https://www.open-std.org/jtc1/sc22/wg14/www/docs/n1570.pdf), 6.5 p5.

- Caso `entero-desborde`: **VERDE** en [evidencias/campo-minas-ronda6.txt](../evidencias/campo-minas-ronda6.txt).
- Caso `ultra-suma`: **VERDE** en [evidencias/campo-minas-ronda6.txt](../evidencias/campo-minas-ronda6.txt).

## UB-002

**Division y resto tienen dos bordes** — UB

El divisor es cero o el cociente con signo no es representable.

Fragmento de estudio (ver las precondiciones y la clase de la pieza):

```c
int q = INT_MIN / -1;
```

Alternativa o pauta valida:

```c
int q = ULTRA_DIV_INT(INT_MIN, -1); /* rechazo controlado */
```

Regla del guardian: Validar antes de dividir o calcular el resto.

Limite: Las funciones nuevas estan limitadas a int.

Referencia: [WG14 N1570: borrador C11, anexo J.2 informativo y clausulas del lenguaje](https://www.open-std.org/jtc1/sc22/wg14/www/docs/n1570.pdf), 6.5.5 p5-p6.

- Caso `division-cero`: **VERDE** en [evidencias/campo-minas-ronda6.txt](../evidencias/campo-minas-ronda6.txt).
- Caso `int-min-entre-menos-uno`: **VERDE** en [evidencias/campo-minas-ronda6.txt](../evidencias/campo-minas-ronda6.txt).
- Caso `resto-int-min`: **VERDE** en [evidencias/campo-minas-ronda6.txt](../evidencias/campo-minas-ronda6.txt).
- Caso `ultra-division-min`: **VERDE** en [evidencias/campo-minas-ronda6.txt](../evidencias/campo-minas-ronda6.txt).
- Caso `ultra-resto-min`: **VERDE** en [evidencias/campo-minas-ronda6.txt](../evidencias/campo-minas-ronda6.txt).
- REVISION: [division-constante.c](../src/guardian/minas-compilacion/division-constante.c): **VERDE** en el informe; no equivale a deteccion en produccion.

## UB-003

**Desplazar fuera del ancho** — UB

Un contador negativo o igual al ancho promovido invalida el desplazamiento.

Fragmento de estudio (ver las precondiciones y la clase de la pieza):

```c
int x = 1 << (sizeof(int) * CHAR_BIT);
```

Alternativa o pauta valida:

```c
int x = ULTRA_SHL_INT(1, 3);
```

Regla del guardian: Comprobar contador y rango antes de operar.

Limite: Nuestra politica de desplazamiento con signo es conservadora.

Referencia: [WG14 N1570: borrador C11, anexo J.2 informativo y clausulas del lenguaje](https://www.open-std.org/jtc1/sc22/wg14/www/docs/n1570.pdf), 6.5.7 p3-p4.

- Caso `desplazamiento`: **VERDE** en [evidencias/campo-minas-ronda6.txt](../evidencias/campo-minas-ronda6.txt).
- Caso `desplazamiento-negativo`: **VERDE** en [evidencias/campo-minas-ronda6.txt](../evidencias/campo-minas-ronda6.txt).
- Caso `ultra-shl-ancho`: **VERDE** en [evidencias/campo-minas-ronda6.txt](../evidencias/campo-minas-ronda6.txt).
- REVISION: [desplazamiento-constante.c](../src/guardian/minas-compilacion/desplazamiento-constante.c): **VERDE** en el informe; no equivale a deteccion en produccion.

## UB-004

**El vecino no pertenece al arreglo** — UB

Estar dentro de la estructura contenedora no autoriza un indice fuera del subarreglo.

Fragmento de estudio (ver las precondiciones y la clase de la pieza):

```c
struct { int a[2], b[2]; } s = {0}; int x = s.a[2];
```

Alternativa o pauta valida:

```c
int x = ULTRA_AT(s.a, 1);
```

Regla del guardian: Conservar limites y comprobar indices.

Limite: Un puntero generico puede perder la informacion de longitud.

Referencia: [WG14 N1570: borrador C11, anexo J.2 informativo y clausulas del lenguaje](https://www.open-std.org/jtc1/sc22/wg14/www/docs/n1570.pdf), 6.5.2.1 y 6.5.6.

- Caso `indice-fuera`: **VERDE** en [evidencias/campo-minas-ronda6.txt](../evidencias/campo-minas-ronda6.txt).
- Caso `subobjeto-fuera`: **VERDE** en [evidencias/campo-minas-ronda6.txt](../evidencias/campo-minas-ronda6.txt).
- Caso `lectura-fuera`: **AMARILLO** en [evidencias/campo-minas-ronda6.txt](../evidencias/campo-minas-ronda6.txt).
- REVISION: [indice-constante.c](../src/guardian/minas-compilacion/indice-constante.c): **VERDE** en el informe; no equivale a deteccion en produccion.

## UB-005

**La direccion sobrevive, el objeto no** — UB

Liberar la reserva termina la vida del objeto.

Fragmento de estudio (ver las precondiciones y la clase de la pieza):

```c
int *p = malloc(sizeof *p); if (p) { free(p); *p = 7; }
```

Alternativa o pauta valida:

```c
int *p = malloc(sizeof *p); if (p) { *p = 7; free(p); p = NULL; }
```

Regla del guardian: Propiedad explicita, analizador y ASan.

Limite: Poner un alias a NULL no invalida los otros aliases.

Referencia: [WG14 N1570: borrador C11, anexo J.2 informativo y clausulas del lenguaje](https://www.open-std.org/jtc1/sc22/wg14/www/docs/n1570.pdf), 6.2.4 y 7.22.3.

- Caso `uso-tras-free`: **AMARILLO** en [evidencias/campo-minas-ronda6.txt](../evidencias/campo-minas-ronda6.txt).
- REVISION: [analizador-uso-tras-free.c](../src/guardian/minas-compilacion/analizador-uso-tras-free.c): **VERDE** en el informe; no equivale a deteccion en produccion.

## UB-006

**Una variable que ya salio de escena** — UB

El puntero escapa del bloque o de la funcion donde vivia su objeto.

Fragmento de estudio (ver las precondiciones y la clase de la pieza):

```c
int *p; { int local = 8; p = &local; } int x = *p;
```

Alternativa o pauta valida:

```c
int x; { int local = 8; x = local; }
```

Regla del guardian: No prestar direcciones mas alla de su vida util.

Limite: ASan diagnostica los ejemplos, no resuelve toda propiedad en produccion.

Referencia: [WG14 N1570: borrador C11, anexo J.2 informativo y clausulas del lenguaje](https://www.open-std.org/jtc1/sc22/wg14/www/docs/n1570.pdf), 6.2.4.

- Caso `uso-tras-scope`: **AMARILLO** en [evidencias/campo-minas-ronda6.txt](../evidencias/campo-minas-ronda6.txt).
- Caso `uso-tras-retorno`: **AMARILLO** en [evidencias/campo-minas-ronda6.txt](../evidencias/campo-minas-ronda6.txt).
- REVISION: [retorna-local.c](../src/guardian/minas-compilacion/retorna-local.c): **VERDE** en el informe; no equivale a deteccion en produccion.

## UB-007

**El alias de otro tipo** — UB

Acceder al valor por un tipo incompatible puede violar las reglas de alias.

Fragmento de estudio (ver las precondiciones y la clase de la pieza):

```c
float f = 1.0f; int x = *(int *)(void *)&f;
```

Alternativa o pauta valida:

```c
unsigned char bytes[sizeof f]; memcpy(bytes, &f, sizeof f);
```

Regla del guardian: Expresar conversion o copia de representacion sin reinterpretar mediante lvalues incompatibles.

Limite: -fno-strict-aliasing es una politica GCC; no hace portable cualquier cast.

Referencia: [WG14 N1570: borrador C11, anexo J.2 informativo y clausulas del lenguaje](https://www.open-std.org/jtc1/sc22/wg14/www/docs/n1570.pdf), 6.5 p6-p7.

- Caso `alias-estricto`: **NEUTRA** en [evidencias/campo-minas-ronda6.txt](../evidencias/campo-minas-ronda6.txt).

## UB-008

**La direccion no esta alineada** — UB

En el objetivo x86-64, sumar un byte a esta base alineada rompe la alineacion exigida por int.

Fragmento de estudio (ver las precondiciones y la clase de la pieza):

```c
_Alignas(int) unsigned char b[2 * sizeof(int)] = {0}; int *p = (int *)(b + 1);
```

Alternativa o pauta valida:

```c
int x; memcpy(&x, b, sizeof x);
```

Regla del guardian: Usar objetos alineados o copiar bytes a un objeto del tipo correcto.

Limite: Los bytes copiados deben representar un valor valido; el ejemplo alternativo presupone esa entrada.

Referencia: [WG14 N1570: borrador C11, anexo J.2 informativo y clausulas del lenguaje](https://www.open-std.org/jtc1/sc22/wg14/www/docs/n1570.pdf), 6.3.2.3 p7.

- Caso `desalineado`: **VERDE** en [evidencias/campo-minas-ronda6.txt](../evidencias/campo-minas-ronda6.txt).
- REVISION: [cast-desalineado.c](../src/guardian/minas-compilacion/cast-desalineado.c): **VERDE** en el informe; no equivale a deteccion en produccion.

## UB-009

**Dos modificaciones sin secuenciar** — UB

Dos incrementos del mismo escalar no estan secuenciados entre si.

Fragmento de estudio (ver las precondiciones y la clase de la pieza):

```c
int i = 0; int x = i++ + i++;
```

Alternativa o pauta valida:

```c
int i = 0; int a = i++; int b = i++; int x = a + b;
```

Regla del guardian: Separar efectos y usar avisos de compilacion.

Limite: volatile no introduce la secuenciacion que falta.

Referencia: [WG14 N1570: borrador C11, anexo J.2 informativo y clausulas del lenguaje](https://www.open-std.org/jtc1/sc22/wg14/www/docs/n1570.pdf), 6.5 p2.

- REVISION: [punto-secuencia.c](../src/guardian/minas-compilacion/punto-secuencia.c): **VERDE** en el informe; no equivale a deteccion en produccion.

## UB-010

**La promesa restrict incumplida** — UB

Dos accesos restringidos incompatibles alcanzan el objeto que se modifica.

Fragmento de estudio (ver las precondiciones y la clase de la pieza):

```c
void f(int *restrict a, int *restrict b) { *a = 1; *b = 2; } /* f(&x, &x); */
```

Alternativa o pauta valida:

```c
/* Admitir alias: retirar restrict y definir el orden de las escrituras. */
```

Regla del guardian: Demostrar el contrato de no alias antes de prometerlo.

Limite: Pendiente de una mina dedicada; detectar memcpy solapado no cubre restrict general.

Referencia: [WG14 N1570: borrador C11, anexo J.2 informativo y clausulas del lenguaje](https://www.open-std.org/jtc1/sc22/wg14/www/docs/n1570.pdf), 6.7.3.1.

**ESTUDIADA / SIN MINA:** no hay deteccion experimental acreditada para esta pieza.

## UB-011

**El char negativo de ctype** — UB

ctype requiere EOF o un valor representable como unsigned char.

Fragmento de estudio (ver las precondiciones y la clase de la pieza):

```c
int c = -2; int x = isalpha(c); /* fuera del dominio si c != EOF */
```

Alternativa o pauta valida:

```c
char c = 'A'; int x = isalpha((unsigned char)c);
```

Regla del guardian: Distinguir el int devuelto por fgetc de un char almacenado.

Limite: No convertir EOF a unsigned char antes de comprobarlo. Pendiente de guardia dedicado.

Referencia: [SEI CERT C STR37-C: dominio de funciones de caracteres](https://cmu-sei.github.io/secure-coding-standards/sei-cert-c-coding-standard/rules/characters-and-strings-str/str37-c/), STR37-C; contraste N1570 7.4 p1.

**ESTUDIADA / SIN MINA:** no hay deteccion experimental acreditada para esta pieza.

## UB-012

**va_arg pide el tipo equivocado** — UB

Los argumentos variadicos aplican promociones; leer como float no recupera el double pasado.

Fragmento de estudio (ver las precondiciones y la clase de la pieza):

```c
/* Tras pasar un double: */ float x = va_arg(ap, float);
```

Alternativa o pauta valida:

```c
double x = va_arg(ap, double);
```

Regla del guardian: Respetar los tipos promovidos y el protocolo de argumentos.

Limite: Pendiente de mina; el compilador puede diagnosticar el caso, pero no todo protocolo variadico.

Referencia: [WG14 N1570: borrador C11, anexo J.2 informativo y clausulas del lenguaje](https://www.open-std.org/jtc1/sc22/wg14/www/docs/n1570.pdf), 7.16.1.1.

**ESTUDIADA / SIN MINA:** no hay deteccion experimental acreditada para esta pieza.

## UB-013

**El formato no coincide con el argumento** — UB

El especificador exige un tipo diferente al argumento real.

Fragmento de estudio (ver las precondiciones y la clase de la pieza):

```c
printf("%d", 2.5);
```

Alternativa o pauta valida:

```c
printf("%.1f", 2.5);
```

Regla del guardian: Formatos literales y avisos de tipos.

Limite: La mina de formato no literal no demuestra por si sola esta incompatibilidad concreta.

Referencia: [WG14 N1570: borrador C11, anexo J.2 informativo y clausulas del lenguaje](https://www.open-std.org/jtc1/sc22/wg14/www/docs/n1570.pdf), 7.21.6.1 p9.

**ESTUDIADA / SIN MINA:** no hay deteccion experimental acreditada para esta pieza.

## UB-014

**longjmp vuelve a un valor indeterminado** — UB

Una local automatica no volatile modificada entre setjmp y longjmp puede quedar indeterminada.

Fragmento de estudio (ver las precondiciones y la clase de la pieza):

```c
/* Local en la funcion de setjmp: */ int x = 0; /* cambiar x, longjmp, luego leer x */
```

Alternativa o pauta valida:

```c
/* Guardar el estado fuera de esas locales, o redisenar el flujo sin longjmp. */
```

Regla del guardian: Revisar vida util y estado preservado al realizar saltos no locales.

Limite: Ficha conceptual, aun sin fixture ejecutable. No atribuir cobertura a UBSan.

Referencia: [WG14 N1570: borrador C11, anexo J.2 informativo y clausulas del lenguaje](https://www.open-std.org/jtc1/sc22/wg14/www/docs/n1570.pdf), 7.13.2.1 p3.

**ESTUDIADA / SIN MINA:** no hay deteccion experimental acreditada para esta pieza.

## UB-015

**NULL no designa un int** — UB

La desreferencia no designa un objeto valido.

Fragmento de estudio (ver las precondiciones y la clase de la pieza):

```c
int *p = NULL; *p = 4;
```

Alternativa o pauta valida:

```c
int x = 0; int *p = &x; *p = 4;
```

Regla del guardian: Comprobar precondiciones y resultados de reserva.

Limite: Un puntero no nulo aun puede ser colgante o no tener espacio suficiente.

Referencia: [WG14 N1570: borrador C11, anexo J.2 informativo y clausulas del lenguaje](https://www.open-std.org/jtc1/sc22/wg14/www/docs/n1570.pdf), 6.5.3.2.

- Caso `puntero-nulo`: **VERDE** en [evidencias/campo-minas-ronda6.txt](../evidencias/campo-minas-ronda6.txt).
- REVISION: [analizador-nulo.c](../src/guardian/minas-compilacion/analizador-nulo.c): **VERDE** en el informe; no equivale a deteccion en produccion.

## UB-016

**El flotante que no cabe en int** — UB

El valor truncado no es representable en el entero destino.

Fragmento de estudio (ver las precondiciones y la clase de la pieza):

```c
double d = 1e20; int x = (int)d;
```

Alternativa o pauta valida:

```c
/* Para un int de 32 bits y double: comprobar finitud y rango antes del cast. */
```

Regla del guardian: Validar finitud y limites del tipo, con float-cast-overflow en diagnostico.

Limite: No copiar limites de int a long long: la precision del flotante tambien importa.

Referencia: [WG14 N1570: borrador C11, anexo J.2 informativo y clausulas del lenguaje](https://www.open-std.org/jtc1/sc22/wg14/www/docs/n1570.pdf), 6.3.1.4.

- Caso `float-a-int`: **VERDE** en [evidencias/campo-minas-ronda6.txt](../evidencias/campo-minas-ronda6.txt).

## UB-017

**strlen sigue buscando el cero** — UB

La busqueda termina leyendo fuera del objeto sin terminador.

Fragmento de estudio (ver las precondiciones y la clase de la pieza):

```c
char s[3] = {'a', 'b', 'c'}; size_t n = strlen(s);
```

Alternativa o pauta valida:

```c
char s[4] = "abc"; size_t n = strlen(s);
```

Regla del guardian: Conservar longitud y capacidad; ASan para lecturas fuera del objeto.

Limite: FORTIFY no demuestra la terminacion de toda cadena recibida por puntero.

Referencia: [WG14 N1570: borrador C11, anexo J.2 informativo y clausulas del lenguaje](https://www.open-std.org/jtc1/sc22/wg14/www/docs/n1570.pdf), 7.1.1 y 7.24.6.3.

- Caso `cadena-sin-nul`: **AMARILLO** en [evidencias/campo-minas-ronda6.txt](../evidencias/campo-minas-ronda6.txt).

## UB-018

**memcpy no permite solapamiento** — UB

Origen y destino se solapan en la region copiada.

Fragmento de estudio (ver las precondiciones y la clase de la pieza):

```c
char s[8] = "abcde"; memcpy(s + 1, s, 3);
```

Alternativa o pauta valida:

```c
char s[8] = "abcde"; memmove(s + 1, s, 3);
```

Regla del guardian: Envoltorio de memcpy o memmove cuando el solape es intencionado.

Limite: Comprobar solape no comprueba por si solo la vida util de ambos objetos.

Referencia: [WG14 N1570: borrador C11, anexo J.2 informativo y clausulas del lenguaje](https://www.open-std.org/jtc1/sc22/wg14/www/docs/n1570.pdf), 7.24.2.1 y 7.24.2.2.

- Caso `memcpy-solapado`: **VERDE** en [evidencias/campo-minas-ronda6.txt](../evidencias/campo-minas-ronda6.txt).
- REVISION: [memcpy-solapado-constante.c](../src/guardian/minas-compilacion/memcpy-solapado-constante.c): **VERDE** en el informe; no equivale a deteccion en produccion.

## UB-019

**Comparar direcciones de reservas distintas** — UB

La comparacion relacional no tiene la relacion de objetos que C exige.

Fragmento de estudio (ver las precondiciones y la clase de la pieza):

```c
/* a y b son dos malloc distintos: */ int x = a < b;
```

Alternativa o pauta valida:

```c
/* Ordenar por un identificador o indice de dominio, no por direcciones ajenas. */
```

Regla del guardian: ASan con pointer-compare/pointer-subtract y su opcion runtime.

Limite: No confundir < y resta con la comparacion de igualdad permitida en otros casos.

Referencia: [WG14 N1570: borrador C11, anexo J.2 informativo y clausulas del lenguaje](https://www.open-std.org/jtc1/sc22/wg14/www/docs/n1570.pdf), 6.5.8 y 6.5.6.

- Caso `comparar-punteros-ajenos`: **AMARILLO** en [evidencias/campo-minas-ronda6.txt](../evidencias/campo-minas-ronda6.txt).
- Caso `restar-punteros-ajenos`: **AMARILLO** en [evidencias/campo-minas-ronda6.txt](../evidencias/campo-minas-ronda6.txt).

## UB-020

**Leer una local sin inicializar** — UB

El ejemplo lee un int automatico sin valor inicial.

Fragmento de estudio (ver las precondiciones y la clase de la pieza):

```c
int x; int y = x;
```

Alternativa o pauta valida:

```c
int x = 0; int y = x;
```

Regla del guardian: Inicializar y revisar caminos con el analizador.

Limite: Las reglas de valores indeterminados tienen matices; no reducirlas a cualquier lectura de bytes.

Referencia: [WG14 N1570: borrador C11, anexo J.2 informativo y clausulas del lenguaje](https://www.open-std.org/jtc1/sc22/wg14/www/docs/n1570.pdf), 6.3.2.1 p2 y 6.7.9.

- Caso `sin-inicializar-ejecucion`: **AMARILLO** en [evidencias/campo-minas-ronda6.txt](../evidencias/campo-minas-ronda6.txt).
- REVISION: [sin-inicializar.c](../src/guardian/minas-compilacion/sin-inicializar.c): **VERDE** en el informe; no equivale a deteccion en produccion.

## UB-021

**volatile no sincroniza hilos** — UB

Hilos modifican el mismo escalar sin sincronizacion.

Fragmento de estudio (ver las precondiciones y la clase de la pieza):

```c
volatile int count; /* dos hilos hacen count++ sin sincronizar */
```

Alternativa o pauta valida:

```c
atomic_int count; /* inicializar y usar atomic_fetch_add */
```

Regla del guardian: Atomicos o mutex con protocolo claro; TSan separado de ASan.

Limite: Los atomicos requieren inicializacion y un orden de memoria adecuado al algoritmo.

Referencia: [WG14 N1570: borrador C11, anexo J.2 informativo y clausulas del lenguaje](https://www.open-std.org/jtc1/sc22/wg14/www/docs/n1570.pdf), 5.1.2.4 p25.

- Evidencia externa (ver su alcance propio): [evidencias/hilos-ronda6.txt](../evidencias/hilos-ronda6.txt).

## NOUB-001

**Unsigned que da la vuelta** — DEFINIDO

La aritmetica unsigned reduce modulo el rango del tipo.

Fragmento de estudio (ver las precondiciones y la clase de la pieza):

```c
unsigned int x = UINT_MAX; x++; /* definido, no es una mina UB */
```

Alternativa o pauta valida:

```c
/* Si el dominio lo prohibe, usar una suma comprobada. */
```

Regla del guardian: Separar semantica C de errores de logica o de tamanos.

Limite: Puede conducir a una reserva demasiado pequena y a una UB posterior.

Referencia: [WG14 N1570: borrador C11, anexo J.2 informativo y clausulas del lenguaje](https://www.open-std.org/jtc1/sc22/wg14/www/docs/n1570.pdf), 6.2.5 p9.

- Caso `tamano-producto`: **VERDE** en [evidencias/campo-minas-ronda6.txt](../evidencias/campo-minas-ronda6.txt).

## NOUB-002

**Orden de llamadas no especificado** — NO_ESPECIFICADO

C no fija cual operando se evalua primero en este ejemplo sin efectos compartidos.

Fragmento de estudio (ver las precondiciones y la clase de la pieza):

```c
/* f y g solo devuelven constantes: */ int x = f() + g();
```

Alternativa o pauta valida:

```c
int a = f(); int b = g(); int x = a + b;
```

Regla del guardian: No depender de un orden que no se especifica.

Limite: Agregar efectos incompatibles no secuenciados sobre el mismo escalar cambia el problema.

Referencia: [WG14 N1570: borrador C11, anexo J.2 informativo y clausulas del lenguaje](https://www.open-std.org/jtc1/sc22/wg14/www/docs/n1570.pdf), 6.5 y 6.5.2.2.

**ESTUDIADA / SIN MINA:** no hay deteccion experimental acreditada para esta pieza.

## NOUB-003

**Desplazamiento derecho de un negativo en C17** — IMPLEMENTACION

Con contador valido, el resultado para un operando con signo negativo depende de la implementacion.

Fragmento de estudio (ver las precondiciones y la clase de la pieza):

```c
int x = -3 >> 1; /* C17: resultado definido por la implementacion */
```

Alternativa o pauta valida:

```c
/* Definir la operacion matematica deseada y evitar asumir que equivale a dividir. */
```

Regla del guardian: Consultar el dialecto y la documentacion de la implementacion.

Limite: No transferir esta etiqueta sin revisar las reglas de otra revision de C.

Referencia: [WG14 N1570: borrador C11, anexo J.2 informativo y clausulas del lenguaje](https://www.open-std.org/jtc1/sc22/wg14/www/docs/n1570.pdf), 6.5.7 p5.

**ESTUDIADA / SIN MINA:** no hay deteccion experimental acreditada para esta pieza.

## Lo que aun no tiene vitrina propia

Estas minas existen en el campo, pero esta coleccion inicial no las explica por separado:

`abs-int-min`, `alloca-gigante`, `bool-invalido`, `copia-pila`, `desborde-heap`, `desplazar-negativo`, `doble-free`, `ejecutar-pila`, `escribir-literal`, `formato-n`, `free-medio`, `free-pila`, `heap-subdesborde`, `inalcanzable`, `indice-negativo`, `negar-int-min`, `nonnull`, `pila-destruida`, `producto-desborde`, `puntero-vuelta`, `recursion-infinita`, `resta-desborde`, `retorno-secuestrado`, `secuestrar-arranque`, `signo-tamano`, `tamano-vuelta`, `ultra-division-cero`, `ultra-indice`, `ultra-producto`, `ultra-resta`, `ultra-resto-cero`, `ultra-shl-contador`, `ultra-shl-desborde`, `ultra-shl-negativo`, `vla-cero`, `vla-negativa`.

## Fuentes y mantenimiento

- [WG14 N1570: borrador C11, anexo J.2 informativo y clausulas del lenguaje](https://www.open-std.org/jtc1/sc22/wg14/www/docs/n1570.pdf).
- [WG14 N3155: Examples of Undefined Behavior for Annex J.2, documento educativo con pendientes](https://www.open-std.org/jtc1/sc22/wg14/www/docs/n3155.pdf).
- [GCC: Instrumentation Options, alcance de los sanitizers](https://gcc.gnu.org/onlinedocs/gcc/Instrumentation-Options.html).
- [SEI CERT C STR37-C: dominio de funciones de caracteres](https://cmu-sei.github.io/secure-coding-standards/sei-cert-c-coding-standard/rules/characters-and-strings-str/str37-c/).

J.2 es un indice informativo; la explicacion debe remitirse a la clausula correspondiente. N3155 es material educativo con TODOs, no una certificacion de exhaustividad ni la norma C17.

Evidencia de ejecucion: `evidencias/campo-minas-ronda6.txt`; SHA-256 del texto normalizado a LF: `d91126a14a84023589b764b86e43586e22a8822300ec4ba3b01623073f1dcd7f`.

Para ampliar: editar el JSON, justificar la regla, vincular un fixture y un control valido, ejecutar pruebas aisladas y regenerar. Una pieza pendiente no se pinta de verde.

Con Python 3.10 o posterior, desde el proyecto:

```text
python scripts/museo-ub.py --evidencia evidencias/campo-minas-ronda6.txt
python scripts/museo-ub.py --evidencia evidencias/campo-minas-ronda6.txt --check
```
