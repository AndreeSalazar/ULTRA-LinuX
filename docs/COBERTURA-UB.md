# Cobertura de C17 y GCC

La [primera entrega de cadenas acotadas](CADENAS-ACOTADAS.md) agrega una API
con limites en ambos lados, rechazo sin modificar destino y soporte de alias.
Pasaron 10368 combinaciones por cada uno de cuatro perfiles en QEMU del taller,
mas controles de pagina protegida y del primer consumidor. La adopcion es
explicita; no se cierra globalmente el hueco de cadenas sin NUL ni se cambia
la evidencia historica de las 62 minas. A: aun no contiene esa nueva build.

La [organizacion de ronda 8](ORGANIZACION-GUARDIANES.md) separa produccion
y laboratorio, conserva las pruebas y asigna salidas independientes por ejecucion.

La [investigacion C para Wayland](WAYLAND-INVESTIGACION-C.md) separa doce
categorias y aporta una primera API de limites de buffers con pruebas de
produccion/diagnostico; no acredita integracion con el compositor.

La [investigacion historica, desde los origenes hasta 2026](HISTORIA-C-Y-SEGURIDAD.md)
recorre todas las revisiones publicadas y el trabajo en curso. Su
[inventario de pendientes](PENDIENTES-GUARDIANES.md) propone 30 lineas de C
y 12 de aislamiento; investigar una fila no acredita implementacion.

El [Museo viviente de C](MUSEO-UB.md) presenta 24 piezas con fragmentos, alternativas,
referencias y estados tomados del informe. El registro editable es `museo-ub.json`;
el generador rechaza nombres inventados y evidencia incompleta. Las fichas pendientes
se mantienen visibles y no se convierten automaticamente en casos VERDE.

Este catálogo organiza familias para ampliar el laboratorio; no enumera toda la
norma ni certifica ausencia de UB. Un caso VERDE demuestra solo ese caso en el
compilador, arquitectura, opciones y entradas probados. UBSan no instrumenta todo C.
El código de usuario se prueba en la VM del taller; esto no es cobertura del kernel.

Estado verificado de la ronda 7: 62 minas de ejecucion (53 VERDE, 1 NEUTRA,
8 AMARILLO) y 36 rechazos de compilacion, mas una suite separada de dominios
ctype con 18 rechazos y controles validos. Ver [ronda 7 y privilegios](GUARDIAN-PRIVILEGIOS.md).
Los casos son ejemplos deliberados
para desarrollar y comprobar reglas defensivas. El guardian no se entrena ni
aprende permisos automaticamente: cada regla es codigo revisable con una prueba
de rechazo y controles validos. El registro de garantias esta en
[GARANTIAS-RONDA6.md](GARANTIAS-RONDA6.md).

Los ocho huecos de produccion siguen identificados: uso tras free, lectura fuera
mediante puntero, uso tras retorno, lectura sin inicializar, cadena sin NUL,
uso tras bloque, comparacion y resta de punteros de reservas distintas. Encontrarlos
en diagnostico o revision no equivale a proteger cualquier programa al ejecutar.

| Familia | Ejemplos del campo | Cobertura y huecos |
|---|---|---|
| Aritmética con signo | suma, resta, producto, -INT_MIN, abs(INT_MIN) | Trampas UBSan y operaciones ULTRA comprobadas; faltan combinaciones de tipos y optimizaciones |
| División y resto | cero, INT_MIN/-1, INT_MIN%-1 | Detenidos en producción; ampliar a más anchos y entradas |
| Desplazamientos | contador negativo, demasiado grande, base negativa | Casos C17 detenidos; no extrapolar reglas a otros dialectos |
| Conversión flotante a entero | 1e20 a int | `float-cast-overflow`; faltan NaN, infinito y extremos de otros tipos |
| Índices y subobjetos | índice negativo, fuera, cruce de un arreglo a su vecino | UBSan cuando conserva información del objeto; un puntero genérico puede eludirlo |
| Tamaños y copias | producto size_t, memcpy/memset fuera, signo convertido a tamaño | FORTIFY y ULTRA_MUL; unsigned que envuelve es definido, aunque provoque una reserva incorrecta |
| Solapamiento | memcpy solapado | Envoltorio propio y revisión; memmove solapado es un control válido |
| Vida útil | uso tras free, retorno y fin de bloque; doble free | ASan o analizador en varios casos; no hay garantía general de propiedad en producción |
| Cadenas | strlen sin NUL, copias truncadas | ASan en diagnóstico para strlen; FORTIFY no conoce todos los tamaños de lectura |
| Punteros | NULL, mala alineación, vuelta de dirección | UBSan en casos instrumentados; no certifica procedencia ni toda aritmética de punteros |
| Comparación/resta de punteros ajenos | dos malloc distintos | ASan con instrumentación de pares y opción runtime explícita; producción no detecta estos dos casos |
| Representaciones inválidas | bool con byte 7 | UBSan en el ejemplo; no garantiza todas las representaciones de todos los tipos |
| Alias y tipo efectivo | int visto mediante float | Resultado neutralizado bajo -fno-strict-aliasing en el ejemplo; no hace portable el código ni elimina otras UB |
| Valores indeterminados | local sin inicializar | Analizador/revisión; ASan no es un detector general de lecturas sin inicializar |
| Evaluación no secuenciada | modificar i dos veces | Mina de compilación; volatile no ordena expresiones de C ni arregla la UB |
| Llamadas y variádicas | cast de firma, prototipo implícito, formato no literal | Revisión de ejemplos; faltan llamadas indirectas y va_arg con tipo equivocado |
| Restricciones restrict | alias entre argumentos declarados restrict | Solapamiento constante de memcpy cubierto; el contrato restrict general está pendiente |
| Concurrencia | dos hilos modifican un int sin sincronización | TSan separado, con control atómico; no demuestra ausencia de carreras en otros caminos |
| Señales | comunicación con manejadores, funciones no seguras en señal | Pendiente de una familia dedicada y revisión de reglas de señales/atomics |
| setjmp/longjmp | locales modificadas, salto fuera de vida útil | Pendiente; no asumir cobertura por UBSan |
| Contratos de biblioteca | free inválido, nonnull, cadenas | Ejemplos cubiertos; pendientes alineación de reservas, parámetros de otras APIs e I/O tras cierre |
| Optimización y atributos GCC | unreachable, nonnull | Ejemplos cubiertos; no es cobertura de todos los atributos, builtins o extensiones |

Los permisos de memoria, RELRO y CET son protecciones adicionales: que bloqueen
un caso no significa que C sea seguro en general. El sandbox limita sus efectos;
el compilador, los sanitizers y las APIs comprobadas cubren fallos diferentes.

También hay comportamientos **definidos**, **no especificados** y **definidos por
la implementación**: no deben etiquetarse todos como UB. La reducción unsigned
módulo 2^N es definida; el orden de evaluación no siempre está especificado;
algunas conversiones a entero con signo dependen de la implementación. El dialecto
del proyecto es C17, no C23.

Para ampliar cada fila: reproducir el caso mínimo, fijar diagnóstico esperado,
añadir una entrada válida como control y ejecutar en aislamiento. Un timeout,
runtime que no arranca o error de compilación ajeno al caso no prueba detección.
Guardar el hueco cuando no haya protección; no borrar la mina para obtener verde.

Referencia: [instrumentación oficial de GCC](https://gcc.gnu.org/onlinedocs/gcc/Instrumentation-Options.html).

Ronda 11: [handles con propietario y banco ASM](VIDA-UTIL-Y-ASM.md) agregan
rechazo de handles caducados dentro de un pool vivo y controles de ABI. No cierran
los huecos globales de UAF, concurrencia ni punteros crudos del catalogo anterior.
