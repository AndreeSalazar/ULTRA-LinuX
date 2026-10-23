# Guardianes: escritura, caza de UB, rendimiento y custodia

2026-10-23, America/Lima. ANTI3M es el nombre personal del enfoque del autor;
no se le atribuye aqui una expansion ni una garantia tecnica universal.
Objetivo compartido: conservar resultados correctos, contener fallos y mejorar
la fluidez con evidencia reproducible. Prevenir sorpresas requiere declarar
tambien lo no comprobado.

## Cuatro responsabilidades

| Categoria | Nombre corto | Responsabilidad | Evidencia para aceptar | Limite |
| --- | --- | --- | --- | --- |
| Guardian de escritura | Escritura | Revisar al desarrollar y construir: tipos, limites, propiedad, errores, liberaciones y contratos de API/ABI; comprobar las operaciones integradas | Revision y compilacion relevantes; entradas validas e invalidas; rechazo sin cambios parciales donde el contrato lo prometa | No intercepta toda escritura de memoria del sistema. Solo las rutas que integran sus controles |
| Guardian de caza de UB | Caza UB | Buscar, reproducir y clasificar defectos o limites de cobertura en el laboratorio; conservar el caso minimo | Fuente y revision, compilador, perfil, salida esperada/real, aislamiento y repeticion; regresion que falla antes y pasa despues cuando corresponda | Un diagnostico ausente no prueba ausencia de UB. Un control invalido esperado no es un fallo inesperado |
| Guardian de rendimiento | Fluidez | Medir coste, latencia y consumo; localizar cuellos de botella y comparar cambios manteniendo el resultado | Datos crudos antes/despues, carga y entorno comparables, repeticiones, p95/p99 y picos; variabilidad y coste de la proteccion | No promete cero tirones ni elimina controles para obtener un resultado favorable. Una prueba funcional no acredita una mejora de velocidad |
| Guardian de recursos y prestamos | Custodia | Autorizar titular, derechos, presupuesto y vida util; retirar nuevas operaciones y cerrar prestamos de forma controlada | Pruebas de referencia ajena/caducada, cuotas, cierre y trabajo en vuelo; aislamiento y coste medidos | Categoria definida, gestor pendiente. Un handle no crea autoridad del kernel ni permite recuperar corrupcion arbitraria |

Escritura incluye tanto las reglas al escribir codigo como los controles de
escritura ejecutados por una API integrada. Debe distinguirse cual de las dos
cosas se verifica en cada entrega. Caza UB tambien registra defectos logicos
que no son UB: por ejemplo, un token ajeno que apunta a memoria accesible.
No se fuerza la etiqueta UB sobre todo fallo de propiedad o de API.

## Dominios compartidos, sin duplicar implementaciones

Las cuatro responsabilidades se aplican por dominio; no exigen cuatro copias del
codigo ni cambian la separacion fisica entre produccion y laboratorio.

| Dominio | Escritura | Caza UB / defectos | Fluidez |
| --- | --- | --- | --- |
| C | Spans, aritmetica de tamanos, propiedad, handles y errores | Bordes, caducidad, aliases, agotamiento y carreras con perfil apropiado | Coste de copias, asignaciones, controles y esperas |
| C++ | Propiedad unica, prestamos y destruccion; contexto de recursos | Recursos movidos/destruidos, excepciones y vida util compartida | Asignaciones, copias y coste de gestion de recursos |
| ASM | ABI, pila, registros y clobbers de memoria declarados | Violaciones de contrato y desenrollado; revision de accesos no instrumentados | Ciclos y latencia bajo carga comparable, sin extrapolar a juegos |
| OpenGL/EGL y Vulkan | Contextos, handles, limites, estados y sincronizacion | Uso invalido de API, recursos caducados y dependencias; validacion disponible | Tiempos CPU/GPU, fotogramas, shaders, transferencias y VRAM |
| Disco y red | Errores de E/S, limites y permisos de las rutas propias | Entradas invalidas, errores y estados incompletos aislados | Latencia, colas, caudal y perdidas/reintentos bajo condiciones declaradas |

Un fallo de Vulkan o de ABI no equivale automaticamente a UB del lenguaje C.
Una copia de driver o biblioteca externa tampoco queda cubierta solo por usar
un perfil GCC en los programas propios. Kernel y terceros conservan sus
contratos y procesos de compilacion especificos.

Custodia cruza esos dominios: regiones y prestamos en C/C++, recursos de
contexto en graficos, archivos/colas en E/S y limites del intercambio con ASM.
No declara integracion universal. El [contrato de Custodia](GUARDIAN-CUSTODIA.md)
define estados, prioridades, revocacion y diferencia entre expulsar una caja
y responder a un fallo del propio kernel.

## Como colaboran y deciden

1. Escritura fija el contrato y el resultado esperado antes del cambio.
2. Caza UB aporta casos de fallo, controles validos y regresiones aisladas.
3. Escritura incorpora una correccion concreta y comprueba sus errores.
4. Fluidez compara la version corregida con la referencia. El perfil de
   diagnostico se identifica por separado del usado para medir el juego.
5. Se revisan correccion y coste antes de integrar un consumidor, reconstruir
   la imagen y validar el arranque o uso nativo cuando corresponda.
6. Cuando haya prestamos, Custodia exige titular y presupuesto comprobados,
   rechazo de pedidos nuevos tras retirar y cierre seguro del trabajo en vuelo.

Si falla una comprobacion requerida, el cambio no se promueve. Si faltan datos
de rendimiento, se puede registrar una correccion como tal, con rendimiento
pendiente; no se declara una optimizacion. Si la medida no distingue la
variabilidad, el resultado es inconcluso. Un coste adicional se documenta y se
decide explicitamente; no se relajan permisos o controles automaticamente.

Estados separados por responsabilidad: pendiente, probado en laboratorio,
integrado en consumidor, validado en hardware, fallido o inconcluso. No se
asigna un porcentaje global de seguridad o fluidez sumando pruebas repetidas.

## Ficha minima por regla o cambio

- Identificador, categoria responsable y dominios afectados.
- Contrato, entradas y resultado esperado; supuestos y exclusiones.
- Fuente/revision, compilador y perfil, entorno y consumidor concreto.
- Caso valido, caso invalido y resultado real de cada comprobacion requerida.
- Ruta a evidencias y estado independiente de correccion e integracion.
- Medida antes/despues, repeticiones y coste, o rendimiento pendiente.
- Condicion para promover y procedimiento de vuelta a la version anterior.

## Mapa actual: herramientas existentes y trabajo pendiente

| Categoria | Base actual | Estado y siguiente paso |
| --- | --- | --- |
| Escritura | `src/guardian/produccion/perfil-gcc.sh`, ultra_string, ultra_buffer y ultra_owner | Revision GCC y APIs con contratos. Owner probado como candidato, aun sin consumidor desplegado |
| Caza UB | `src/guardian/laboratorio/`, `scripts/campo-punteros.sh` y banco ASM | Casos y evidencias existentes; ampliar cobertura por contrato, sin declarar todos los huecos cerrados |
| Fluidez | Informes existentes y protocolo del plan C/C++/ASM/API | Hay observaciones del sistema; falta la comparacion controlada del coste de owner y su consumidor. Este documento no instala un servicio de medicion |
| Custodia | Contrato de recursos y prestamos; owner como referencia limitada de vida util | Definicion, sin gestor desplegado ni autoridad nueva del kernel. Primer consumidor acotado pendiente |

Ejemplo owner: Escritura rechaza capacidad insuficiente y handle de otro pool;
Caza UB conserva las regresiones y demuestra que un buffer externo liberado
sigue fuera del contrato; Fluidez mantiene pendiente el coste en un consumidor
real. Sus 661036 comprobaciones por perfil no son una medida de FPS.

Esta organizacion es una definicion y mapa de trabajo: no activa demonios,
banderas globales, nuevas politicas de kernel ni cambios en A:.

Referencias internas: [produccion y laboratorio](ORGANIZACION-GUARDIANES.md),
[prioridades y evidencias de punteros](PLAN-C-CXX-ASM-API.md),
[cobertura y limites](COBERTURA-UB.md),
[diseno del guardian de fallos del kernel](GUARDIAN-FALLOS-KERNEL.md).
