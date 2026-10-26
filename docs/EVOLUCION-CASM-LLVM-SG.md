# CASM, LLVM y SUPER GRAFO: crecer con evidencia

## Lo revisado el 25 de octubre de 2026

SUPER GRAFO ya reconoce el compilador de `compile_commands.json`, usa Clang
para diagnosticos y analisis estatico, y conserva las revisiones de archivos y
cabeceras. Su perfil de guardianes es especifico de GCC/C: no se debe pasar tal
cual a Clang ni anunciarlo como cobertura de C++.

Clang es el frontend de C/C++; LLVM aporta la infraestructura y el backend.
No hay que separar dos lenguajes ocultos dentro de LLVM ni mantener un fork de
LLVM para empezar. Hay que separar dialecto, opciones, herramientas, casos y
evidencias. Referencia: [toolchain de Clang](https://clang.llvm.org/docs/Toolchain.html).

## Primera ampliacion implementada

`src/guardian/laboratorio/llvm/probar.py` ejecuta solo el corpus deliberado de
`casos.json`, como usuario sin privilegios en QEMU. Cada compilacion y programa
pasa por Bubblewrap con namespaces separados, sin red del anfitrion, sin montar
el proyecto ni `/home` y con las herramientas/bibliotecas del sistema en solo
lectura. Solo su carpeta temporal `/work` es persistente y escribible.
No es un lanzador de proyectos arbitrarios ni una VM por cada caso.

El banco prueba C17 y C++20, O0 y O2, variantes mal/bien. Separa ASan y UBSan:
overflow con signo, desplazamientos, alineacion, NULL, heap fuera de limite,
uso tras free/retorno y downcast C++ con vptr. Exige el diagnostico de la familia
esperada y salida fallida para `mal`; salida limpia y cero para `bien`. Un crash
cualquiera no acredita deteccion. Timeout, fallo de compilacion, control fallido
y deteccion ausente son estados distintos. Sin aislamiento no ejecuta el banco.

La medicion inicial cumplio 51 de 60 casos. Se identificaron dos causas concretas:
Clang no aplica checks `null`/`alignment` a punteros a tipos `volatile`, y el grupo
`undefined` no incluye `vptr`. Se activa este ultimo explicitamente en C++.
Se conservan las variantes `volatile` como ocho `known_gap` separados, nunca como
`passed`, y se agregan sus controles no volatile. Referencia:
[exclusiones de UBSan](https://clang.llvm.org/docs/UndefinedBehaviorSanitizer.html#volatile).

Resultado final con Clang 23.1.3: 68 casos cumplidos y ocho huecos conocidos en
76 combinaciones. SUPER GRAFO pasa 57 pruebas, con una medida optativa ignorada.
Evidencias: `evidencias/llvm-sg/2026-10-25/`. El porcentaje de ese corpus no mide
la fraccion de todo el lenguaje que esta protegida.

```
python3 src/guardian/laboratorio/llvm/probar.py
ultra-carpetas --consulta cobertura_ub /ruta/al/proyecto '{"limite":5}'
```

El banco guarda cada ejecucion en `build/llvm-ub/run-*` y su informe actual en
`build/llvm-ub/current.json`. La consulta nueva del SUPER GRAFO lee ese informe,
agrupa por lenguaje/perfil, prioriza fallos y limita su salida; no compila ni
ejecuta minas desde MCP. La evidencia es historica del corpus, no una revision
de todo el proyecto abierto. No verifica una firma del autor del informe.
Las rutas de logs se conservan en el informe, no se abren automaticamente.

## Herramientas y limites confirmados en documentacion oficial

| Carril | Que aporta | Limite y decision |
|---|---|---|
| [ASan](https://clang.llvm.org/docs/AddressSanitizer.html) | Memoria fuera de limites y varias vidas utiles invalidas | Solo codigo/entradas ejecutados; activa uso tras retorno y scope. No limitar AS virtual como si fuese RAM fisica: usa shadow muy grande |
| [UBSan](https://clang.llvm.org/docs/UndefinedBehaviorSanitizer.html) | Familias de operaciones invalidas instrumentadas | `undefined` no incluye todos los checks. Una conversion con perdida o wrap unsigned no es automaticamente UB |
| [TSan](https://clang.llvm.org/docs/ThreadSanitizer.html) | Carreras de datos entre hilos | Banco separado; no confundir con ASan ni ejecutarlo sobre una sesion de juego como configuracion de produccion |
| [MSan](https://clang.llvm.org/docs/MemorySanitizer.html#handling-external-code) | Uso de datos sin inicializar | Necesita instrumentar dependencias; no certificar C++ con libstdc++ ajena sin revisar ese contrato |
| Analizador estatico Clang | Sospechas y recorridos sin ejecutar el programa | Ya integrado en el grafo; falsos positivos y limites de modelos/bibliotecas siguen presentes |

Los runtimes de ASan, MSan y TSan son herramientas de diagnostico; LLVM no los
presenta como runtimes endurecidos de produccion. El laboratorio no reemplaza
las APIs comprobadas, ownership, permisos de procesos ni pruebas de rendimiento.
No se promete eliminar todo UB o toda sorpresa en C/C++.

## Orden propuesto para crecer

| Prioridad | Pieza | Motivo | Criterio de cierre |
|---|---|---|---|
| 1 | Banco LLVM por lenguaje/perfil | Evitar extrapolar GCC/C a Clang/C++ | Pares mal/bien, dos optimizaciones, herramientas y banderas registradas; errores visibles en SG. Primera ronda implementada |
| 2 | Identidad de una revision del proyecto | Un verde viejo no debe liberar codigo nuevo | Hash de fuente + cabeceras + compilador + flags + entradas; invalidar resultado al cambiar cualquiera; pruebas de cabecera modificada y fallo de herramienta |
| 3 | AST y vida util C++ | El texto no entiende RAII, moves, templates y alias | Recuperar AST con el dialecto real; casos de dangling view/span, vector invalidado, captura de lambda y move con controles validos |
| 4 | TSan y MSan separados | Cerrar concurrencia e inicializacion sin falsear cobertura | Entornos instrumentados reproducibles; controles atomicos/inicializados; incompatible/ausente nunca se vuelve verde |
| 5 | Fuzzing con cobertura | Ejercitar caminos que no se pensaron a mano | Harnesses acotados de APIs propias, corpus persistente, minimizar cada fallo y convertirlo en regresion; no ejecutar cualquier entrada desde MCP |
| 6 | Puerta de entrega | Impedir publicar ante evidencia fallida o caducada | Reglas por componente y dialecto, fallar cerrado al faltar una prueba requerida; medir latencia, RAM y tiempos de cuadro aparte |
| 7 | CASM: mapa fuente a bytes Native | Ver por que existe cada instruccion en SG | Rangos de fuente y bytes verificados, CFG, trampas y ABI; no reutilizar el mapa del frontend metal64 anterior |
| 8 | CASM: modulos e imports versionados | Generar mas programas sin perder control | Simbolos/tipos/relocaciones acotados y verificador ampliado; casos adversos de firma, destino y permisos |
| 9 | CASM sobre BMO-X | Independencia real del anfitrion Linux | Cargador con paginas RX/RW/NX y #UD aislado; ABI de archivos/buffers; repetir autocompilacion alli sin Linux |

Agregar floats, punteros libres, concurrencia o optimizaciones agresivas a CASM
antes de esos contratos multiplica el espacio de errores. La siguiente ampliacion
del lenguaje debe incluir especificacion, emisor, verificador, casos correctos y
rechazos en la misma entrega. CASM Native conserva su backend propio; esta ronda
no lo reemplaza por LLVM ni modifica sus bytes autocompilados.

## Produccion ULTRA

Esta ronda cambia el banco y el codigo de la consulta de SUPER GRAFO. No parchea
LLVM, el kernel, el compositor ni los juegos. A: requiere una entrega posterior
si se quiere instalar esta nueva consulta en el escritorio arrancable.
Los resultados del corpus no demuestran mejoras de FPS ni de temperatura.
