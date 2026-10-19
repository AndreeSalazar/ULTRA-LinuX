# Que falta en los guardianes despues de la ronda 7

Inventario de investigacion, **2026-10-18**. Base: `702b50c`.
Origen: [recorrido historico y fuentes primarias](HISTORIA-C-Y-SEGURIDAD.md),
[cobertura actual](COBERTURA-UB.md) y lectura de los fuentes del proyecto.
Las prioridades y propuestas de prueba son criterio de este proyecto.
No son una lista normativa completa de UB ni trabajo ya implementado.

## Lo demostrado que conservamos

- [Campo C](../evidencias/campo-minas-ronda7.txt): 62 casos de ejecucion,
  53 VERDE, 1 NEUTRA y 8 AMARILLO; 36 fixtures rechazados al compilar.
- [Dominios ctype](../evidencias/dominios-ronda7.txt): 18 rechazos en dos
  perfiles; por perfil, 257 entradas validas para tres operaciones.
- [Caja y syscalls](../evidencias/syscalls-ronda7.txt): 20 controles reales
  del filtro, 17 de caja y retirada de identidad comprobada en el taller.
- [Equivalencia BPF](../evidencias/seccomp-formal-ronda7.txt): resultado
  condicionado al contrato y al modelo documentados.

Son resultados de la VM Arch. No sustituyen el arranque de ULTRA con estos
binarios. No se han vuelto a ejecutar durante esta investigacion documental.

## Primero: ocho huecos conocidos de produccion

| Caso ya existente | Lo que falta demostrar o construir |
|---|---|
| Uso tras free | Propiedad o vida util comprobada durante ejecucion normal. |
| Lectura fuera mediante puntero | Conservar limite de lectura al perder el tipo de arreglo. |
| Uso tras retorno | Impedir referencias a objetos automaticos finalizados. |
| Local sin inicializar | Seguir inicializacion por caminos; no confundir poner cero con validar el programa. |
| Cadena sin NUL | Lectura acotada con longitud explicita. |
| Uso tras bloque | Evitar que sobreviva un prestamo de una local. |
| Comparar punteros ajenos | Representar el orden del dominio sin relaciones entre reservas. |
| Restar punteros ajenos | Validar pertenencia al mismo objeto antes de obtener distancias. |

La prioridad es cerrar interfaces usadas por codigo real. Una biblioteca de
propiedad o vistas acotadas necesita comprobar sus propias reglas de uso;
envolver malloc/free no invalida automaticamente todos los alias.

## Ampliacion de C: 30 lineas de trabajo

P1: siguiente ciclo por impacto o cercania al codigo actual. P2: ampliar
cobertura y portabilidad. P3: migracion o investigacion especializada.
Las filas mezclan UB, contratos, errores logicos y restricciones de traduccion
deliberadamente: cada fixture futuro debera fijar su clasificacion exacta.

| ID | Prioridad | Familia | Proxima evidencia propuesta |
|---|---|---|---|
| C-01 | P1 | Promociones y tipos mixtos | Bordes signed/unsigned, char/short y tipos de resultado; controles del rango valido. |
| C-02 | P1 | Flotante a entero | NaN, infinitos y valores adyacentes a limites de varios destinos. |
| C-03 | P1 | Contratos variadicos | Promociones, va_arg incompatible, agotamiento y va_copy; controles de protocolos correctos. |
| C-04 | P1 | Formatos tipados | Fixtures de incompatibilidad real, ademas de formato no literal. |
| C-05 | P1 | Restrict | Dos interfaces equivalentes con y sin alias; separar contrato general de memcpy. |
| C-06 | P1 | Firmas indirectas | Punteros de funcion, parametros y retorno incompatibles; revisar opciones de diagnostico. |
| C-07 | P1 | Realloc | Fallo sin perder propietario, movimiento, alias antiguo y politica explicita para cero. |
| C-08 | P1 | Objetos y alineacion | Reservas sobrealineadas, tamano de reserva y liberador correcto. |
| C-09 | P1 | Estado indeterminado | Separar valor de objeto, copia de representacion y padding; documentar dudas normativas. |
| C-10 | P1 | Senales | Interrupciones durante estado mutable y escritura; verificar restricciones ISO C y POSIX por separado. |
| C-11 | P1 | Salud del guardian | Simular rechazo de sigaltstack/sigaction y exigir que no se anuncie proteccion instalada. |
| C-12 | P1 | Setjmp/longjmp | Contextos, vida util del destino y locales modificadas, con control valido. |
| C-13 | P1 | Tamano y longitud | Fuentes acotadas para copia/append, truncamiento y suma de capacidades. |
| C-14 | P2 | Procedencia de punteros | Conversion a entero y vuelta, fin de vida y subobjetos; separar C17 de TS 6010. |
| C-15 | P2 | Puntero one-past | Formacion valida frente a acceso invalido; limites de ptrdiff_t. |
| C-16 | P2 | Tipo efectivo y uniones | Comparar copiar con memcpy, acceder por char y acceder por tipo incompatible. |
| C-17 | P2 | Arreglos flexibles | Reserva y tamano real frente al tipo declarado; varios niveles de optimizacion. |
| C-18 | P2 | Const y volatile | Revision de contratos; no tratar volatile como sincronizacion ni chequeo de vida. |
| C-19 | P2 | Atomicos | Ordenes de memoria y compare-exchange; propiedad del algoritmo ademas de ausencia de avisos TSan. |
| C-20 | P2 | Recursos de hilos | Vida de mutex/condiciones, join y cancelacion; etiquetar lo POSIX aparte. |
| C-21 | P2 | Streams e I/O | Uso tras cierre, cambio lectura/escritura y resultados de operaciones fallidas. |
| C-22 | P2 | Cadenas anchas y locale | Estado multibyte y contratos de wctype; no extrapolar los tres wrappers ctype. |
| C-23 | P2 | Conversion textual | Rango y fin de entrada, errno y parsing parcial con contratos explicitos. |
| C-24 | P2 | Ordenacion y busqueda | Comparadores sin overflow ni efectos incompatibles; control de consistencia. |
| C-25 | P2 | Traduccion y enlace | Tipos incompatibles entre unidades; probar con y sin LTO. |
| C-26 | P2 | Preprocesador y nombres | Concatenacion de tokens, macros reservadas y diferencias de dialecto. |
| C-27 | P2 | Builtins y atributos | Unreachable, nonnull, assume/alineacion; clasificar extensiones y precondiciones. |
| C-28 | P2 | ABI y arquitectura | AArch64 u otro destino; tamanos, alineacion y endian. No inferirlo de x86-64. |
| C-29 | P2 | Matriz de compiladores | GCC/Clang, versiones y O0/O1/O2/O3/LTO en suites pequenas; no exigir un resultado de UB cruda. |
| C-30 | P3 | Migracion C23/C2y | stdckdint, nuevos contratos de biblioteca y cambios adoptados/propuestos; mantener C17 estable. |

Esta tabla es una agenda original, no una reproduccion de J.2. Los puntos de
partida normativos se encuentran en los catalogos del recorrido historico.
Para conocer lo que instrumenta cada perfil se debe consultar la
[documentacion GCC](https://gcc.gnu.org/onlinedocs/gcc/Instrumentation-Options.html).
ASan no equivale a un detector general de valores sin inicializar; los
sanitizers de hilos y memoria tampoco demuestran por si solos un algoritmo.

Observacion de fuente para C-11: `ultra_guard_install` descarta hoy los retornos
de sigaltstack y sigaction. Es un pendiente identificado por inspeccion, no un
fallo provocado en este estudio. La revision de funciones permitidas en un
manejador debe usar [signal-safety(7)](https://man7.org/linux/man-pages/man7/signal-safety.7.html)
y conservar las diferencias entre POSIX e ISO C.

## Aislamiento y syscalls: 12 lineas de trabajo

| ID | Prioridad | Pendiente | Criterio de cierre propuesto |
|---|---|---|---|
| S-01 | P1 | Arranque con la ronda 7 | Recompilar ULTRA y recuperar evidencia de su kernel y binarios exactos. |
| S-02 | P1 | Tiempo total | Caja bloqueada o dormida termina por reloj externo y sus descendientes desaparecen. |
| S-03 | P1 | Recursos agregados | Presupuesto por caja para memoria, procesos y CPU; probar bajo limites pequenos. |
| S-04 | P1 | Temporal por caja | Dos cajas simultaneas no comparten archivos temporales y hay cuota verificable. |
| S-05 | P1 | Vida del supervisor | Probar su muerte antes/despues de fork y exec; comprobar ausencia de procesos residuales. |
| S-06 | P1 | Menor autoridad del supervisor | Separar preparacion de vigilancia, retirar lo innecesario y comprobar credenciales. |
| S-07 | P1 | Sesion cotidiana sin root | Diseñar el acceso a operaciones administrativas antes de cambiar la shell. |
| S-08 | P1 | Canales heredados | Definir que archivos/pipes/terminales puede recibir cada programa; probar concesiones accidentales. |
| S-09 | P2 | Argumentos de las 46 concesiones | Priorizar open/openat, ioctl/fcntl y creacion de procesos; probar combinaciones y equivalencias ABI. |
| S-10 | P2 | Caminos y carreras | Pruebas de enlaces, renombrados y aperturas relativas; usar reglas reales, no comparar texto de rutas. |
| S-11 | P2 | Compatibilidad y actualizaciones | Nuevos kernel/libc/BusyBox y nuevas syscalls siguen denegados hasta revision; no aprender permisos de strace automaticamente. |
| S-12 | P2 | Limites del contrato formal | Mutaciones adicionales y revisiones independientes del modelo BPF; separar equivalencia del filtro de aislamiento completo. |

Fuentes para disenar esas pruebas: [cgroup v2](https://www.kernel.org/doc/html/latest/admin-guide/cgroup-v2.html)
para cuotas agregadas; [openat2(2)](https://man7.org/linux/man-pages/man2/openat2.2.html)
para entender controles de resolucion; [Landlock](https://www.kernel.org/doc/html/latest/userspace-api/landlock.html)
para derechos y ABI. Estas referencias no anuncian mecanismos ya integrados.
No se concedera openat2 ni ninguna otra syscall solo por aparecer aqui.

Los cinco conjuntos de capacidades del programa se comprueban antes de exec;
eso no elimina la autoridad del supervisor. Los 20 controles del filtro no
ejercitan todos los argumentos de todas las syscalls permitidas. Los pipes
concedidos pueden comunicar con procesos de fuera; denegar socket no convierte
esa concesion en una garantia de confidencialidad.

## Como convertir una fila en una garantia concreta

Una ficha futura debe guardar dialecto y regla, entrada minima, control valido,
instrumentacion, resultado esperado y evidencia de aislamiento. La propiedad
debe expresarse sobre una interfaz y un dominio precisos. Ejemplo: rechazar un
entero fuera de dominio en ULTRA_TOLOWER no protege toda la biblioteca ctype.

Para cerrar una fila: implementar, ejecutar, conservar el resultado aunque
revele un hueco, revisar el alcance y solo entonces enlazarla desde el museo.
Un resultado silencioso en codigo con UB no se convierte en comportamiento
correcto porque varios compiladores coincidan. Una prueba negativa debe
separar rechazo esperado de fallo de infraestructura o de otro guardian.

Orden de trabajo recomendado: S-01 a S-08 para comprobar y limitar efectos;
C-03/C-04/C-05 y C-10/C-11 para familias sin prueba dedicada y salud del
guardian; despues ampliar vida util y limites de lectura. Las 42 filas son
un inventario inicial de pendientes, no 42 nuevas protecciones ni una promesa
de agotar toda la semantica de C.
