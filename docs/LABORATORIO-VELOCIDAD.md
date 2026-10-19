# Laboratorio de velocidad (pendiente)

Estado: **diseñado, sin implementar.** En esta sesión, el asistente que construyó el resto del proyecto vio cortadas varias respuestas por un filtro de seguridad automático mientras escribía este laboratorio y las minas de memoria. Se decidió dejar el diseño por escrito para implementarlo con otra herramienta o a mano. Nada de este archivo se ha medido todavía.

## Pregunta

"La UB da más velocidad." Es en parte cierto: el compilador supone que la UB nunca ocurre y optimiza a partir de esa suposición. Con Anti3M esa velocidad no se acepta a ciegas: cada caso se convierte en una **bandera de revisión con números**.

Para cada guardián se quiere saber:
1. Cuánto cuesta en una carga concreta, en ns por elemento y en % frente a la versión sin guardianes.
2. Si existe una forma **definida** de escribir el mismo código que recupere esa velocidad sin UB.
3. Un veredicto con su evidencia.

## Variantes de compilación

La misma carga se compila con un solo guardián cada vez, para aislar su coste:

| Variante | Opciones (sobre una base `-O2` sin protector de pila, FORTIFY, CET ni RELRO) |
|---|---|
| sin guardianes | solo la base (`ULTRA_CRUDO` de `scripts/guardianes.sh`) |
| trampa de UB | `-fsanitize=undefined,float-cast-overflow -fsanitize-trap=all` |
| límites | `-fsanitize=bounds-strict -fsanitize-trap=all` |
| sin alias estricto | `-fno-strict-aliasing` |
| variables a cero | `-ftrivial-auto-var-init=zero` |
| FORTIFY | `-D_FORTIFY_SOURCE=3` |
| memcpy vigilado | FORTIFY 3 + el envoltorio de `ultra_guard.h` |
| protector de pila | `-fstack-protector-strong -fstack-clash-protection` |
| CET | `-fcf-protection=full`, ejecutado con el `GLIBC_TUNABLES` del proyecto |
| desbordamiento definido | `-fwrapv`: otra forma de definir el desbordamiento |
| producción | `ULTRA_WARN` + `ULTRA_PROD` completos |

## Cargas propuestas (versión con UB o suposición / versión definida)

| Carga | Qué depende de la UB o de la suposición | Forma definida a comparar |
|---|---|---|
| Recorrido con paso 2 sobre un arreglo | Índice `int`: el desbordamiento con signo es UB y GCC ensancha el índice | Índice `size_t`; comparar también con `unsigned` y con `-fwrapv` |
| Sumar un valor leído por puntero a un arreglo `float` | El alias estricto permite leer el valor una sola vez | Copiar el valor a una variable local, o usar `restrict` |
| Búfer local grande, de uso parcial | Coste de poner a cero cada llamada | Atributo explícito `__attribute__((uninitialized))` en ese búfer, como bandera de revisión |
| Muchas copias pequeñas de tamaño variable | Coste de FORTIFY y de la comprobación de solape | Ninguna: solo medir |
| Tabla consultada con índices | Coste de la comprobación de límites | Índice enmascarado (`& 255`), que demuestra el límite |
| Muchas llamadas pequeñas con arreglos locales | Coste del protector de pila y de CET | Ninguna: solo medir |
| Suma comprobada (`ULTRA_ADD`) en un bucle | Coste del guardián explícito, que impide vectorizar | Comprobar una sola vez el rango antes del bucle |

## Reglas de medición

- **Entradas en tiempo de ejecución**, desde `argv` o datos generados, para que el compilador no precalcule el resultado.
- **Mediana** de al menos 7 rondas, alternando las variantes.
- **Suma de control obligatoria:** si una variante es más rápida pero da otro resultado, no cuenta.
- **Ruido:** WHPX añade variación, así que conviene repetir en el metal (Ryzen 5600X) antes de decidir.
- **Veredicto propuesto:** menos de 3 % → VERDE (guardián gratis); de 3 % a 15 % → AMARILLO (coste medible); más de 15 % → ROJO-VELOCIDAD (buscar una forma definida). Un ROJO-VELOCIDAD nunca se resuelve apagando el guardián en silencio. Se resuelve con una forma definida, o con una excepción explícita y documentada en la revisión.

## Dónde encaja

Se ejecutaría en el taller, como `scripts/campo-minas.sh`, y guardaría su informe en `evidencias/`. Sus resultados alimentarían `docs/CAMPO-MINAS.md` y las opciones de `scripts/guardianes.sh`.
