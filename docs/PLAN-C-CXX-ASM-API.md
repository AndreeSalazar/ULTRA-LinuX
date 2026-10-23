# Metas de C, C++, ASM y APIs

Marco de trabajo: [Escritura, Caza UB y Fluidez](CATEGORIAS-GUARDIANES.md).
Cada entrega distingue correccion, coste e integracion en un consumidor.

2026-10-23, America/Lima. El trabajo combina correccion, endurecimiento,
aislamiento y medicion. Los guardianes ejecutan reglas definidas; las pruebas
no cambian automaticamente banderas ni permisos ni hacen que aprendan todo C.

| Prioridad | Area | Meta siguiente | Criterio de cierre |
| --- | --- | --- | --- |
| 1 | C/punteros | Capacidades externas explicitas | Rechazo sin modificar datos; controles validos y negativos en cuatro perfiles. API candidata probada en esta ronda. |
| 1 | C/propiedad | Identidad del pool en los tokens | API candidata probada: dos pools vivos rechazan tokens ajenos, conservando datos y deteccion de caducados. Agotamiento probado sin reciclaje. |
| 1 | C/integracion | Adoptar propietario y limites en un consumidor real | Revision de todos los aliases, errores de asignacion y liberaciones; regresiones y coste medido. Pendiente. |
| 1 | C++ | Integrar propiedad unica de recursos | Consumidor y revision fijados, destruccion/contexto correctos y resultado comprobado. No integrado en Hyprland. |
| 1 | ASM | Desenrollado, contratos inline y llamadas | Declarar registros/memoria alterados y comprobar preservacion, pila y casos soportados de desenrollado. Pendiente. |
| 1 | Medicion | Carga representativa antes/despues | Datos crudos, mismos resultados y condiciones comparables; no extrapolar microbenchmarks a FPS. Pendiente. |
| 2 | OpenGL/EGL | Fallos de inicializacion y recuperacion | Mecanismo soportado por el backend, recursos liberados y vuelta a dibujar. No forzar resets del escritorio. |
| 2 | Vulkan | Memoria no coherente y dependencias entre envios | Flush/invalidate, limites y esperas comprobados donde el backend lo permita. Pendiente. |
| 2 | Vulkan | Primer pipeline grafico y cache | Imagen esperada antes de medir cache y compatibilidad. El benchmark previo usa fill, no pipelines. |
| 2 | Concurrencia | Vida util compartida y retirada segura | Contrato de sincronizacion y pruebas de carreras separado del perfil ASan. No resuelto por los handles actuales. |

La siguiente promocion a produccion debe elegir un consumidor y reconstruir su
imagen con regresiones. La entrega actual de investigacion no modifica A: ni
los juegos. La revision estructural ELF ya desplegada sigue siendo distinta de
probar la logica y vida util de cada programa.

## Catalogo de choques de punteros por ampliar

| Clase | Problema | Defensa a investigar o mantener |
| --- | --- | --- |
| Limites espaciales | Copia mayor que origen/destino; indices y strides | Capacidades, operaciones aritmeticas comprobadas, tests de bordes y ASan |
| Caducidad | Uso tras free, retorno, scope o realloc | Propiedad y prestamos; no conservar aliases tras invalidacion; diagnostico instrumentado |
| Identidad | Token de otro pool; direccion reutilizada con otro objeto | Identidad/version de propietario; prueba de reutilizacion y agotamiento |
| Liberacion | Doble free o free de un puntero interior/ajeno | Propietario unico y API de liberacion controlada |
| NULL/alineacion | Acceso sin objeto o con alineacion incorrecta | Validar entradas; diagnosticos aplicables; no confundir no-NULL con valido |
| Procedencia | Restar/ordenar punteros de objetos distintos | Evitar esa aritmetica; perfil diagnostico especifico |
| Solapamiento | memcpy entre regiones solapadas | Rechazo donde corresponda o memmove con contrato de limites |
| Tipo/firma | Alias incompatible o funcion llamada con tipo incorrecto | Tipos correctos, ABI declarada y revision de casts |
| Concurrencia | Otro hilo libera o cambia el objeto durante el acceso | Sincronizacion que cubra acceso y destruccion; limite temporal de los prestamos |
| CPU/GPU | Puntero, mapeo o recurso reutilizado antes de terminar su uso | Propiedad, esperas y contrato de contexto/API; no basta una direccion coincidente |

No se comprueba un puntero dereferenciandolo "para ver si sirve". Que una
direccion siga mapeada o no sea NULL no demuestra que el objeto siga vivo.
Las capacidades suministradas deben corresponder a objetos reales: mentir
sobre ellas o conservar un puntero tras liberar sigue incumpliendo el contrato.

GCC ofrece analisis de rutas para algunos errores y diagnosticos instrumentados,
con cobertura limitada. Los checks pointer-compare/subtract requieren ASan y
activacion explicita mediante `detect_invalid_pointer_pairs=2`; no son un modo
universal de validar direcciones. Fuentes primarias:
[instrumentacion GCC](https://gcc.gnu.org/onlinedocs/gcc/Instrumentation-Options.html),
[analizador GCC](https://gcc.gnu.org/onlinedocs/gcc/Static-Analyzer-Options.html).

## Primera auditoria y cambio concreto

Se reviso `ultra_owner.c` y se muestrearon las rutas `match_aliases`/`load_wanted`
de `hardware.c` y el escritor `put` de `efi.c`. Esto no es una auditoria completa
del kernel, de los drivers ni de todos los fuentes C del proyecto.

- **Owner:** su read/write anterior acota el objeto interno, pero recibe un
  buffer externo sin capacidad. Se agregan `ultra_owner_read_bounded` y
  `ultra_owner_write_bounded`: rechazan size > capacity antes de copiar.
  Se mantienen las funciones anteriores por compatibilidad y con su contrato.
- **Owner:** se reprodujo que los primeros handles de dos pools pueden coincidir
  y ser aceptados por el pool equivocado. Ya estaba excluido por el contrato;
  no se presenta como un fallo nuevo dentro de el. Falta identidad del pool.
- **Hardware:** las lineas y las dependencias apuntan al buffer leido. En las
  rutas muestreadas se conserva hasta terminar el procesamiento y esperar a
  los hijos; no se confirmo ahi un uso tras free. Mantener ese orden al migrar.
- **EFI:** `put` depende de len <= cap. El escritor se inicializa con len=0,
  comprueba capacidad restante y solo incrementa tras una copia aceptada.
  No se confirmo un desbordamiento en esa ruta bajo ese invariante.

## Evidencia de esta ronda

`evidencias/punteros/08be52d1570044768c87e30e7740198c/` conserva compilador,
fuentes, resultados y diagnosticos. `scripts/campo-punteros.sh` reproduce la
ronda en el taller, usando carpetas de salida nuevas y procesos bwrap sin red
ni discos personales, con limites de CPU, tiempo, archivos y pila.

Pasan **656023 comprobaciones C por perfil**, en prod-O2, prod-O3, diag-O1 y
diag-O2, mas 1000 ciclos ASM de cuatro controles por perfil. Son las mismas
pruebas repetidas en cuatro configuraciones. No son millones de clases de UB.
Tambien pasan la revision GCC -fanalyzer y los avisos como errores del banco.

Los tres procesos negativos terminan como se esperaba con ASan:

1. API antigua + destino de 3 bytes para copiar 8: stack-buffer-overflow.
2. API bounded + buffer externo liberado, aun declarando capacidad 8: heap-use-after-free.
3. API bounded + pool destruido: heap-use-after-free.

Estos diagnosticos son controles deliberados del laboratorio, no fallos
inesperados de la regresion ni protecciones que se desplieguen en los juegos.
El caso cruce-pool devuelve el objeto del otro pool sin diagnostico ASan,
mostrando que memoria accesible y propiedad correcta son problemas distintos.

## Segunda ronda: identidad aplicada, aun sin consumidor desplegado

La evidencia anterior corresponde a la version previa. Ahora el handle opaco
de 64 bits contiene generacion de slot (32), identidad de pool (28) e indice
(4). Lookup compara la identidad antes del acceso al slot. Read/write,
incluyendo bounded, y free rechazan el handle ajeno con UO_STALE.

Las identidades se asignan con un contador atomico por proceso y no se reciclan
al destruir. Tras 268435455 pools creados, create devuelve NULL; los vivos
siguen funcionando. La sincronizacion del contador permite crear pools
distintos en paralelo; NO permite acceder o destruir un mismo pool sin
sincronizacion. No hay atomicos nuevos en read/write/free.

El espacio de identidades requiere una unica implementacion enlazada. No
intercambiar tokens entre copias independientes en bibliotecas dinamicas, otros
procesos o ramas tras fork. No serializar ni interpretar el token. No es un
secreto ni una capacidad contra codigo hostil dentro del mismo proceso.
El pool y los buffers externos siguen necesitando vida util valida.

[Evidencia de identidad](../evidencias/punteros/punteros-identidad-1cf58586de094aad9d8050fff12069bf/informe.txt):
GCC 16.2.1, taller QEMU sin discos personales; **661036 comprobaciones por
perfil**, las mismas en cuatro configuraciones. Se comprueban las cinco
operaciones con un handle ajeno y 1000 recreaciones. Un ejecutable separado
acerca el contador privado al limite y la generacion a UINT32_MAX para
comprobar agotamiento y retirada, sin agregar hooks a la API publica.
Los cuatro perfiles pasan tambien ASM y agotamiento; los tres controles ASan
invalidos siguen diagnosticando los limites que permanecen abiertos.

Esto no cambia Hyprland, Proton ni los juegos, ni reconstruye A:. La integracion
en un consumidor y la medida de coste siguen pendientes.

## Fluidez y seguridad: objetivos relacionados, medidas distintas

Eliminar un UB confirmado corrige un defecto; no demuestra por si solo menos
tirones. Sanitizadores y analisis estatico sirven tanto para calidad como para
seguridad: limites, vida util y aislamiento reducen fallos y superficie de
ataque, con alcances distintos. Los perfiles de diagnostico no son el perfil
para medir rendimiento de juego.

Para evaluar fluidez: fijar escena y duracion, versiones, resolucion, ajustes,
temperatura y estado de caches; guardar tiempos por fotograma, percentiles
p95/p99, peores picos y RAM/VRAM junto con actividad de disco/CPU/GPU. Repetir
antes/despues separando ejecucion fria y caliente. Investigar compilacion de
shaders, lecturas, presion de memoria y esperas segun la evidencia. No declarar
una ganancia de FPS o ausencia de tirones a partir de estas pruebas de owner.
