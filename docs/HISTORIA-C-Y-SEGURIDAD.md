# C, UB y aislamiento: recorrido historico para los guardianes

Investigacion consultada el **18 de octubre de 2026**, America/Lima.
Base experimental del proyecto: commit `702b50c`, ronda 7.

Este recorrido cubre las grandes etapas desde el origen de C hasta el trabajo
de WG14 en 2026. Es una sintesis con fuentes primarias, no una revision de
todos los articulos, defectos, compiladores ni vulnerabilidades publicados.
El [inventario de trabajo pendiente](PENDIENTES-GUARDIANES.md) convierte los
hallazgos en pruebas propuestas; ninguna investigacion documental aumenta
por si sola la cobertura experimental.

## Antes de ISO: por que C se parece a la maquina sin ser ensamblador

Dennis Ritchie situa su formacion entre 1969 y 1973, con especial actividad
en 1972. C deriva de B y BCPL, pero incorpora tipos y cambia la representacion
de arreglos. La descripcion K&R aparece en 1978; la portabilidad de Unix
impulsa cambios entre 1977 y 1979. Esas practicas anteriores a ISO no deben
recibir automaticamente las etiquetas normativas de C17.
[Relato de Ritchie, presentado en 1993](https://www.nokia.com/bell-labs/about/dennis-m-ritchie/chist.html).

La consecuencia para nuestro laboratorio es registrar siempre dialecto,
compilador, ABI y entorno. Que un acceso coincida con la direccion esperada en
x86-64 no demuestra que el programa cumpla las reglas del lenguaje.

## Todas las revisiones publicadas y la revision en curso

| Etapa | Referencia de la revision | Accion para este proyecto |
|---|---|---|
| C89 | ANSI X3.159-1989 | Conservar contexto del codigo antiguo. |
| C90 | ISO/IEC 9899:1990 | Separar normalizacion internacional de un dialecto nuevo. |
| C95 | Enmienda de 1995 a C90 | Incluir caracteres y biblioteca ampliada en futuras revisiones. |
| C99 | ISO/IEC 9899:1999 | Revisar restrict, VLA, tipos y conversiones en su contexto. |
| C11 | ISO/IEC 9899:2011 | Revisar atomicos, hilos y alineacion. |
| C17 | ISO/IEC 9899:2018 | Mantenerlo como dialecto explicitamente elegido hoy. |
| C23 | ISO/IEC 9899:2024 | Preparar una matriz aparte antes de migrar. |
| C2y | Revision en desarrollo, sin norma publicada en la tabla consultada | Registrar propuestas sin aplicarlas a C17. |

Las fechas e identificadores proceden de [WG14: proyectos y revisiones](https://www.open-std.org/jtc1/sc22/wg14/www/projects.html).
Los nombres C17 y C23 no coinciden con el ano de publicacion ISO.
La pagina identifica N3220 y N2310 como borradores tempranos de revisiones
posteriores: no conviene anunciarlos como copias exactas de las normas anteriores.
Tambien registra corrigenda de C90 en 1994/1996, de C99 en 2001/2004/2007
y de C11 en 2012. Sus cambios deben revisarse antes de reutilizar una
interpretacion antigua. La TS 17961 tiene ademas una correccion de 2016.

Para cambios concretos, GCC ofrece una [matriz de soporte por revision](https://gcc.gnu.org/projects/c-status.html).
C17 introduce correcciones y aclaraciones; C23 incorpora cambios mayores.
La disponibilidad en el compilador no implica que la libc de cada destino
proporcione toda la biblioteca. El proyecto fija `-std=c17`: actualizar GCC
no autoriza a reinterpretar automaticamente sus ejemplos como C23.

## Hitos de fallos, herramientas y aclaraciones

| Fecha | Hecho documentado | Leccion propuesta para el guardian |
|---|---|---|
| 1988; analisis de 1989 | El gusano de Internet aprovecho un desborde de entrada de fingerd mediante gets. | Transportar capacidad y longitud; conservar controles de limites. [Spafford](https://www.cs.cmu.edu/~dga/15-712/F07/papers/Spafford89.pdf). |
| 1998 | StackGuard presenta defensas para cambios del retorno. | Comprobar corrupcion que no toca el canario y danos a datos. [Publicacion original](https://www.usenix.org/legacy/publications/library/proceedings/sec98/full_papers/cowan/cowan_html/cowan.html). |
| 2011 | LLVM explica la relacion entre UB y optimizaciones. | Comparar compilaciones y localizar la primera operacion invalida. [LLVM, parte 1](https://blog.llvm.org/2011/05/what-every-c-programmer-should-know.html). |
| 2011-2012 | DR 400 estudia diferencias entre bibliotecas para realloc de tamano cero. | Mantener reglas por version y un contrato propio para tamano cero. [DR 400](https://www.open-std.org/jtc1/sc22/wg14/www/docs/dr_400.htm). |
| GCC 4.8 | Se integra AddressSanitizer para fallos de memoria. | Mantener diagnostico separado y reconocer sus limites. [Notas oficiales](https://gcc.gnu.org/gcc-4.8/changes.html). |
| 2013 | Se publica TS 17961 sobre reglas de codigo seguro. | Ampliar la revision mas alla de las operaciones que instrumenta UBSan. [WG14](https://www.open-std.org/jtc1/sc22/wg14/www/projects.html), [borrador N1717](https://www.open-std.org/jtc1/sc22/wg14/www/docs/n1717.pdf). |
| 2013-2014 | DR 451 expone problemas de interpretacion de valores indeterminados. | No etiquetar toda copia de bytes o padding como la misma lectura de una local. [DR 451](https://www.open-std.org/jtc1/sc22/wg14/www/docs/dr_451.htm). |
| 2014 | Heartbleed: falta una comprobacion de limites en TLS heartbeat. | Probar lecturas y divulgacion, ademas de escrituras y control de flujo. [Aviso de OpenSSL](https://openssl-library.org/news/secadv/20140407.txt). |
| 2018 | N2222 estudia preguntas de punteros y presenta propuestas tentativas. | Registrar preguntas abiertas sin convertir una propuesta en regla vigente. [N2222](https://www.open-std.org/JTC1/SC22/WG14/www/docs/n2222.htm). |
| C23 | El borrador incluye stdckdint.h y cambia el tratamiento de realloc con tamano cero para un puntero no nulo. | Crear controles especificos de migracion. [N3096, 7.20 y 7.24.3.7](https://www.open-std.org/jtc1/sc22/wg14/www/docs/n3096.pdf). |
| 2025 | WG14 registra publicada la TS 6010 sobre procedencia de punteros. | Estudiar el modelo sin atribuir implementacion a GCC ni a nuestras APIs. [Estado WG14](https://www.open-std.org/jtc1/sc22/wg14/www/projects.html), [borrador N3231](https://www.open-std.org/jtc1/sc22/wg14/www/docs/n3231.pdf). |
| 2026 | Continua la revision de UB, senales, promociones y limites. | Conservar una lista fechada de asuntos para investigar. [Registro WG14](https://www.open-std.org/jtc1/sc22/wg14/www/wg14_document_log.htm). |

Son hitos seleccionados, no una cronologia exhaustiva de incidentes. Un CVE
es un fallo de una implementacion concreta; una regla de UB pertenece a un
dialecto. Un error de protocolo, permisos o logica puede existir sin UB.

## Mas catalogos utiles, con distintos papeles

- **Anexo J.2 de un borrador:** indice informativo que remite a clausulas;
  no reemplaza el texto de la regla. El museo actual usa
  [N1570, borrador C11](https://www.open-std.org/jtc1/sc22/wg14/www/docs/n1570.pdf)
  como referencia con esa etiqueta, no como texto literal de C17.
- **N3155:** material de ejemplos con apartados pendientes; sirve para proponer
  vitrinas, no para contar pruebas realizadas.
  [Examples of Undefined Behavior for Annex J.2](https://www.open-std.org/jtc1/sc22/wg14/www/docs/n3155.pdf).
- **TS 17961 / N1717:** reglas y requisitos de diagnostico de codigo inseguro;
  su alcance no equivale a un sanitizer universal.
  [Borrador de reglas](https://www.open-std.org/jtc1/sc22/wg14/www/docs/n1717.pdf).
- **Defectos y aclaraciones:** conservar pregunta, respuesta, revision afectada
  y fecha. [Registro C11/C17](https://www.open-std.org/JTC1/SC22/WG14/issues/c11c17/log.html),
  [registro C23, N3966](https://www.open-std.org/jtc1/sc22/wg14/www/docs/n3966.htm).
- **N3957, estado 2026-08-23:** analisis preliminar que organiza entradas
  1-87 del lenguaje, 88-98 del preprocesador y 99-221 de biblioteca,
  tomando N3220 como base y agregando casos. Distingue cambios, chequeos
  dinamicos y problemas costosos de memoria.
  [Ghosts and Demons](https://www.open-std.org/jtc1/sc22/wg14/www/docs/n3957.pdf).

N3957 informa que cambios para retirar UB en 45 entradas habian sido votados para C2y
hasta agosto de 2026. Esto no significa 45 protecciones disponibles en C17,
ni que 221 sea el numero universal de UB de toda la historia de C. Su tabla
tambien contiene planes y recomendaciones del autor.

Un ejemplo especialmente historico es **N3972**, fechado 2026-09-26:
propone quitar una regla de lectura de locales ligada, segun su autor,
a los registros NaT de Itanium. El registro lo incorpora en octubre.
Es una propuesta para C2y, no permiso para leer locales sin inicializar en
nuestro codigo C17. [Documento completo](https://www.open-std.org/jtc1/sc22/wg14/www/docs/n3972.pdf).

## La otra historia: autoridad y llamadas al sistema

| Mecanismo | Hito verificable | Aplicacion actual o pendiente |
|---|---|---|
| Seccomp estricto | Linux 2.6.12; luego filtrado en Linux 3.5 | Entender la diferencia entre modos. [Historia por Michael Kerrisk](https://man7.org/conf/ndcsecurity2023/Linux_Seccomp--NDC-Security-2023-Kerrisk.pdf). |
| no_new_privs | Linux 3.5 | Evitar adquisicion de privilegios por exec; no elimina los ya existentes. [Kernel](https://kernel.org/doc/html/v5.18/userspace-api/no_new_privs.html). |
| Syscall seccomp | Linux 3.17 | No confundir la fecha de la syscall con el origen del mecanismo. [seccomp(2)](https://man7.org/linux/man-pages/man2/seccomp.2.html). |
| Landlock | Incorporado en Linux 5.13 | Comprobar ABI y derechos disponibles, no solo presencia. [Kernel](https://www.kernel.org/doc/html/latest/userspace-api/landlock.html). |

La caja combina estas capas; ninguna deduce la validez de los objetos C.
Seccomp inspecciona numeros y argumentos escalares, pero no sigue punteros
para comprobar el contenido de una ruta o estructura. La separacion x86-64/x32
y el filtrado de argumentos requieren atencion especifica.
[Contrato y advertencias de seccomp](https://man7.org/linux/man-pages/man2/seccomp.2.html).

Los pendientes propios incluyen recursos agregados, almacenamiento temporal,
vida del supervisor y sesion sin privilegios. La historia de C y la historia
del kernel aportan preguntas distintas; no hay un numero de syscalls que
demuestre cubrir todas sus combinaciones.

## Reglas para mantener este recorrido

1. Registrar fecha del documento y de consulta; una fecha de rastreo no es
   la fecha del acontecimiento.
2. Distinguir norma publicada, borrador, propuesta, interpretacion e incidente.
3. No copiar ejemplos ajenos como si fueran codigo propio ni ejecutarlos
   automaticamente. Disenar fixtures originales y aislados.
4. Adjuntar a cada futura prueba una regla, un rechazo esperado, un control
   valido y el alcance exacto del resultado.
5. Mantener visible lo pendiente cuando el sanitizer no lo detecte.

La investigacion cubre la secuencia historica de revisiones; queda pendiente
la trazabilidad exhaustiva, entrada por entrada y por dialecto, de los
catalogos y sus aclaraciones. Tampoco se han auditado todas las bibliotecas,
arquitecturas, CVE ni extensiones GCC. El siguiente documento delimita tareas
concretas para avanzar sin confundir esa meta con una garantia ya conseguida.
