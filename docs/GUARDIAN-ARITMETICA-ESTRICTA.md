# Guardianes explicitos de aritmetica int

Estas reglas rechazan la operacion antes de ejecutarla. No constituyen un
aprendizaje automatico ni una prueba de ausencia de UB en el resto del programa.

| API | Rechaza antes de operar | Ejemplo valido |
| --- | --- | --- |
| `ULTRA_DIV_INT(a, b)` | `b == 0`, `INT_MIN / -1` | `-7 / 3 == -2` |
| `ULTRA_REM_INT(a, b)` | `b == 0`, `INT_MIN % -1` | `-7 % 3 == -1` |
| `ULTRA_SHL_INT(a, n)` | contador negativo o mayor/igual al ancho de int, base negativa, resultado superior a INT_MAX | `1 << 3 == 8` |

Se usan operandos `int`. Los argumentos se evaluan una sola vez. Como en
cualquier llamada C, el orden entre argumentos no esta definido: no se deben
pasar modificaciones incompatibles de la misma variable en ambos argumentos.
Una conversion a int o un acceso invalido para obtener el argumento ocurren
antes de entrar al guardian y no quedan protegidos por estas funciones.

La regla de desplazamiento es una politica conservadora para Linux/GCC C17:
exige un resultado no negativo representable como int. Rechaza tambien algunos
casos de desplazamiento al bit de signo que una implementacion puede definir.
El control de rango del contador sucede antes del desplazamiento que se usa
para validar el valor; la comprobacion no debe introducir su propia UB.

Se agregaron ocho minas independientes: dos de division, dos de resto y cuatro
de desplazamiento. El evaluador exige la bandera de la familia correspondiente;
un fallo generico, un timeout o una salida normal no acreditan proteccion.
Los controles validos comprueban INT_MIN dividido por uno, signos de division
y resto, desplazamientos en los extremos aceptados y evaluacion unica.

Las minas `ultra-*` conservan las API explicitas tambien en la variante llamada
cruda: esta variante retira las opciones generales del compilador, no elimina
las comprobaciones escritas en el programa. Por tanto, su deteccion cruda no
demuestra una propiedad del compilador ni aumenta la cobertura de C arbitrario.

Quedan fuera de estas API: enteros long/long long, aritmetica de punteros,
proveniencia, vida util, carreras, memoria sin inicializar y codigo C que use
directamente los operadores. Cada familia requiere reglas y pruebas propias.
El aislamiento limita consecuencias; no convierte UB en una operacion valida.
