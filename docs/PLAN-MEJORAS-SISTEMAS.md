# Plan de mejoras: C, ASM, C++, graficos y juegos

Fecha: 2026-10-22. Estado: **cadenas, handles C y primer banco ASM verificados en el taller;
integracion de propiedad y las siguientes etapas pendientes**. Ver [resultado y limites](CADENAS-ACOTADAS.md).
La [puerta ELF](ELF-PRODUCCION.md) ya forma parte de build/instalacion;
generico #50 y PC #36 estan probados en QEMU y copiados en A:, con arranque
nativo pendiente de confirmacion.
La [entrega de hora peruana](HORA-PERU.md) incorpora America/Lima, LC_TIME
peruano y ajuste NTS puntual al abrir el escritorio.
El objetivo es conservar la experiencia de juego que el propietario confirma y
convertir problemas concretos en mejoras verificadas. No se da por terminada una
etapa por escribir este documento ni por agregar fichas al museo.

## Que significa mejorar

Cada cambio debe declarar su objetivo principal antes de probarse:

- Rendimiento: menos tiempo de CPU, espera, asignaciones, E/S o fotogramas lentos.
- Estabilidad: completar operaciones validas y recuperarse de fallos previstos.
- Seguridad: rechazar una operacion concreta fuera del contrato y limitar su alcance.
- Diagnostico: explicar un fallo con informacion suficiente y sin exportar sesiones.

Un chequeo puede tener coste de CPU y aun ser una mejora de seguridad. Un caso
que termina con una trampa esperada puede pasar su prueba defensiva sin mejorar
el rendimiento. No se publica un porcentaje de aceleracion por contar rechazos.

## Orden y dependencias

| Paso | Trabajo | Motivo y dependencia | Estado |
|---|---|---|---|
| 0 | Registrar la base y el protocolo | Permitir volver a la configuracion que ya funciona | Inventario documentado; nuevas mediciones pendientes |
| 1 | C: limites, propiedad y errores | C es la base propia de init, caja y herramientas | Cadenas y errores de instalacion del guardian probados en QEMU y copiados en A:; API de propiedad candidata, sin consumidor; arranque nativo pendiente |
| 2 | ASM: puentes C/ASM | Comprobar contratos de la frontera con el compilador | Banco de registros/alineacion verificado; puerta ELF obligatoria para cinco ejecutables propios; ampliar contratos de ASM sigue pendiente |
| 3 | C++: vida util y recursos | Ampliar las sondas propias sin trasladar supuestos de C | Pendiente; aprovecha controles de limites |
| 4 | OpenGL/EGL/GLES | Comprobar recursos, contexto y recuperacion | Pendiente; depende del manejo de recursos C++ |
| 5 | Vulkan | Comprobar sincronizacion y vida de recursos explicitas | Pendiente; compara con los controles CPU/graficos |
| 6 | Proton y juegos | Medir el efecto observable en la experiencia real | Pendiente; requiere almacenamiento y captura estable |
| 7 | Recuperacion del escritorio | Conservar datos y volver a una sesion utilizable | Pendiente; integra resultados de las etapas anteriores |

Cada etapa puede concluir con un resultado negativo: por ejemplo, un cambio mas
lento se descarta y se registra. La etapa 7 tiene pruebas iniciales de fallo de
clientes desde el laboratorio; su integracion final viene despues de 4 a 6.

## Paso 0: base y reglas comunes

Base funcional previa de juego: reconstruccion de almacenamiento del commit `1013461`.
El museo posterior `9f6c395` agrega investigacion documental, sin reemplazar los
binarios de A:. Conservar los manifiestos de ambas imagenes y el recibo de
[instalacion](../evidencias/almacen-c-2026-10-22/instalacion-A.json).
No promover una imagen experimental sobre esta referencia antes de evaluar su resultado.
La nueva [instalacion ELF](../evidencias/elf-produccion-2026-10-22/instalacion-A.json)
reemplaza esos archivos de A: tras validar los candidatos. Los dos kernels y
dos sistemas anteriores se conservan en el respaldo local bajo build/.
La [instalacion horaria posterior](../evidencias/hora-peru-2026-10-22/instalacion-A.json)
conserva otro respaldo de esos perfiles y el mismo datos.ext4.

El ultimo analisis disponible de la partida registra 120 muestras, RAM maxima
7,645 GiB, VRAM maxima 3364 MiB y 30 GiB libres al cerrar. El propietario
confirma una partida de Warframe sin tirones percibidos y unos 60 ms de ping.
No son una medicion de FPS, page faults ni comparacion controlada con Windows.
La muestra grafica no cubre necesariamente toda la sesion. Las futuras fichas
de partida distinguiran siempre observacion del usuario de dato instrumentado.

Para cada ejecucion registrar commit de fuentes, hashes de binarios, compilador,
flags, dialecto, CPU, kernel, driver, API/renderer real, entradas y resultado.
Los casos invalidos se ejecutan como procesos separados en una VM desechable,
sin discos personales. Para C/ASM/C++ empezar con 512 MiB, 64 tareas y 30 s por
caso; ampliar solo cuando el fixture lo justifique. No aplicar estos limites
a juegos completos. Las pruebas con acceso a GPU fisica requieren un banco
dedicado y recuperacion independiente: bwrap no contiene un fallo del driver.

Separar ejecucion normal, ASan/UBSan y TSan segun sus compatibilidades. Los
sanitizadores no se usan para comparar el rendimiento normal de produccion.
Todo caso invalido necesita un control valido y una expectativa precisa:
diagnostico de compilacion, rechazo de API, senal/trampa o resultado incorrecto.
Un caso que no se manifiesta se registra como no detectado, no como protegido.

Para rendimiento: fijar carga, resolucion, refresco, VSync/limite de FPS,
versiones, caches y programas simultaneos. Comenzar con calentamiento y cinco
pasadas de 120 s por variante, alternando el orden de base/candidato; guardar
los datos crudos y su dispersion. Una microprueba puede necesitar otro protocolo
explicito. No usar maximos aislados ni una partida de red distinta como equivalente.

## 1. C: limites y propiedad antes de tocar memoria

**Motivo:** el perfil actual no sigue la vida de todos los punteros. El
[inventario de cobertura](COBERTURA-UB.md) mantiene ocho huecos conocidos.
[ultra_buffer.h](../src/guardian/produccion/ultra_buffer.h) comprueba aritmetica
de layout, pero no valida un puntero ni su vida util.

Primera tanda, en este orden:

1. Extender controles de layouts y vistas: cero, uno, limite exacto, capacidad
   insuficiente y overflow. Verificar que el destino permanezca intacto al fallar.
2. Proponer una interfaz de bytes/cadenas con capacidad explicita: origen sin
   NUL, destino corto, rango vacio y solapamiento definido o rechazado antes de copiar.
3. Prototipar propiedad de buffers solo dentro de una API delimitada: propietario,
   vistas y fin de vida. Detectar handles caducados dentro de esa API; no prometer
   invalidar alias C crudos obtenidos fuera de ella.
4. Crear fixtures de liberacion interior, VLA no positivo y firma indirecta
   incompatible, enlazados a C-001 a C-003 del museo. En el perfil de produccion,
   -Wvla rechaza tambien VLA validos de ISO C: ese rechazo es politica, no prueba
   de deteccion especifica del VLA invalido.
5. Simular fallo de reserva y de las operaciones del propio guardian. No anunciar
   una proteccion instalada si su inicializacion fallo.

**Medir:** coste por operacion valida, asignaciones, bytes copiados y RSS/PSS;
mantener entradas identicas al comparar. Antes de optimizar una comprobacion,
demostrar que la alternativa preserva el contrato.

**Terminado cuando:** cada caso tiene control aceptado, el fallo conserva el
estado prometido, los perfiles pertinentes tienen evidencia y se actualiza
la cobertura sin convertir detecciones de laboratorio en garantias globales.
La primera promocion sera una interfaz pequena usada por un consumidor propio.

**Salida:** fuentes y pruebas en `src/guardian/laboratorio/`; API aprobada en
`src/guardian/produccion/`; una carpeta nueva por ejecucion en `evidencias/`.

## 2. ASM: comprobar la frontera con C

**Motivo:** el [area ASM](../ASM/README.md) es documental; no hay aun una suite
propia que acredite pila, registros o contratos inline. El objetivo inicial es
la ABI System V AMD64 de Linux, sin extender el resultado a Windows x64.

Primera tanda:

1. Escribir un puente C/ASM valido y comprobar resultado, rsp y alineacion de
   entrada para una firma ordinaria fijada. El harness debe respetar la ABI.
2. Comprobar conservacion de rbx y r12-r15 con centinelas. Incorporar despues
   variantes incorrectas, cada una en un hijo aislado, y comprobar que el harness
   realmente las distingue. Documentar reglas de rbp y desenrollado por separado.
3. Comparar operands read/write, matching constraints, clobbers cc y early-clobber
   donde corresponda, con presion de registros y O0/O2. Revisar el ensamblado
   generado: un contrato incorrecto puede aparentar funcionar en ejecucion.
4. Estudiar DIV/IDIV con dividendo ancho preparado, divisor cero y cociente fuera
   de rango; validar casos correctos antes de clasificar la excepcion del SO.
5. Contrastar memory clobber, atomicos y barreras hardware sin confundir sus
   niveles. No interpretar una prueba de estres como demostracion del algoritmo.

**Medir:** llamadas por lote y coste del puente frente a C equivalente solo
despues de probar resultados correctos; inspeccionar tambien el codigo emitido.
No introducir ASM en produccion solo por ser mas cercano al hardware.

**Terminado cuando:** los controles conservan la ABI, el harness detecta sus
variantes demostrables y cada limite de deteccion queda explicito. Una rutina
ASM se promociona solo si resuelve una necesidad concreta con ventaja medida.

**Salida prevista:** `ASM/laboratorio/`, actualmente sin crear; evidencias
independientes de las minas ISO C. Ninguna instruccion privilegiada en el host.

## 3. C++: vida util, contenedores y recursos

**Motivo:** las sondas propias ya tienen contratos y perfiles separados, pero
no son proteccion global de Hyprland, de la biblioteca estandar o de Proton.

Primera tanda: vista de string temporal frente a propietario vivo; vector con
realocacion forzada frente a referencia recuperada; new[]/delete incorrecto
frente a RAII correcto; errores durante construccion de recursos; carrera no
atomica frente al estado protegido. Mantener TSan en un banco separado.
Agregar el contraste de `f(i++, i++)` por edicion: C17 y C++17+ no tienen las
mismas reglas; no convertir un warning heredado en clasificacion normativa.

**Medir:** copias, movimientos, asignaciones y coste de consultas validas.
Comparar cambios de representacion/propiedad antes que banderas globales.
No aplicar -fno-exceptions, -fno-rtti o cambios de ABI a dependencias por defecto.

**Terminado cuando:** las fichas CXX-001 a CXX-005 tienen fixtures y controles,
se fijan edicion y toolchain, y los recursos del prototipo se liberan tambien
en caminos de error. Mantener separados los resultados de cada sanitizador.

**Salida:** `C++/laboratorio/`; contratos adoptados por fuentes propias en
`C++/produccion/`. La adopcion por un consumidor real es un paso explicito.

## 4. OpenGL / EGL / GLES: recursos y recuperacion

**Motivo:** ampliar las sondas que ya existen en software antes de cambiar el
renderer del escritorio diario. Un error GL y un UB de C++ son categorias distintas.

Primera tanda: shader invalido con programa anterior conservado; contexto
ausente o no current; limites de textura y capacidad de buffers; pixel-store
y PBO; destruccion en orden y repeticion de crear/destruir. Simular perdida de
contexto solo mediante un mecanismo soportado por el banco y documentar si no
esta disponible. No forzar resets de la GPU del escritorio.

**Medir:** tiempo CPU de llamadas, tiempo GPU cuando el mecanismo sea compatible,
compilacion de shaders, bytes subidos, memoria y conteo de recursos. Para caches
comparar frio/caliente por separado. Examinar consultas redundantes, cambios
de estado y recompilaciones solo despues de medirlos.

**Terminado cuando:** hay resultado de pixel esperado para controles validos,
rechazos atribuidos, recuperacion a un dibujo valido y ausencia de crecimiento
injustificado en bloques repetidos de ciclos. Validar primero en software;
NVIDIA fisica es otra evidencia. Un fork de Hyprland requiere revision fija,
construccion propia y regresiones; instalar un paquete no acredita ese fork.

**Salida:** sondas existentes de `C++/laboratorio/grafica/` y evidencia por backend.

## 5. Vulkan: sincronizacion y vida de recursos

**Motivo:** sus condiciones de uso no son identicas a OpenGL; las capas de
validacion ayudan a detectar incumplimientos concretos, no certifican toda la API.

Primera tanda: rangos/alineacion de buffers, memoria vinculada correctamente,
recursos aun en uso, fences/semaforos, reutilizacion de command buffers y
propiedad de colas. Empezar con VUID precisos y controles de finalizacion en
software. Una simulacion de VK_ERROR_DEVICE_LOST prueba al llamador; no demuestra
recuperacion de un driver real. No ejecutar trabajo invalido sobre la RTX diaria.

**Medir:** coste de grabacion y envio, esperas, numero de envios, transferencias
y memoria. Comparar agrupacion/reutilizacion solo cuando conserve orden y vida
de recursos. Medir rendimiento sin capas de validacion y correccion con ellas.

**Terminado cuando:** cada uso invalido seleccionado tiene VUID esperado o limite
de deteccion documentado, los controles producen resultados correctos y las
esperas tienen tiempo acotado. Preservar y explicar los avisos de fugas previos;
no silenciarlos para obtener una salida verde.

**Salida:** `sonda-vulkan.cpp` y nuevas pruebas junto a ella; evidencia separada
por cargador, driver y mecanismo de validacion.

## 6. Proton y juegos: medir la experiencia

**Motivo:** el propietario confirma Warframe jugable; falta cuantificar el
comportamiento y distinguir disco, shaders, CPU, GPU y red. El museo propio
no instrumenta automaticamente los binarios del juego ni todo Proton.

Primera tanda: registrar version de juego, Proton y driver, resolucion y ajustes;
elegir una escena repetible; identificar API y backend efectivos; comprobar
espacio de actualizacion y separar cache fria/caliente. Comparar una variable
por vez, con prefijos de prueba y forma de volver a la version funcional.
No copiar sesiones a Git ni borrar prefijos o caches personales para preparar
un benchmark. Si una prueba requiere sesion personal, la ejecuta el propietario
en su equipo y se exportan solo mediciones revisadas.

**Medir:** tiempo de arranque, frametimes p50/p95/p99, percentiles lentos,
RSS/PSS, VRAM, CPU, E/S y fallos de pagina menores/mayores. Definir el calculo
de 1% low si se publica. Medir ping, variacion y perdidas aparte: 60 ms no es
una medida de FPS ni de compilacion de shaders.

**Terminado cuando:** existe captura repetible base/candidato con dispersion,
sin errores de acceso, integridad o guardado. Conservar el candidato solo si
la mejora supera el ruido observado y no empeora estabilidad o compatibilidad.
Si no mejora, conservar la base y registrar el resultado. No quitar validacion
de certificados o integridad para ganar velocidad.

**Salida:** receta reproducible por juego y datos anonimizados por sesion.
Ninguna nueva recomendacion universal de Proton a partir de una sola partida.

## 7. Recuperacion del escritorio: integrar sin perder datos

**Motivo:** un cliente puede fallar sin que deba perderse toda la sesion o sus
escrituras. Es distinto de sobrevivir a un fallo del kernel/driver.

Primera tanda: cliente testigo junto a cliente que aborta; falta de espacio en
un ext4 ficticio; desconexion y reconexion de red; salida del compositor y
retorno al menu; recuperacion del siguiente arranque con datos persistentes.
Probar montajes y desmontajes ordenados en discos ficticios, incluyendo el
contenedor en C: y la vista Windows de solo lectura. No llenar discos reales.

**Medir:** tiempo hasta menu o sesion utilizable, procesos huerfanos, bloqueos
retenidos, integridad de archivos de prueba, estado de montajes y registros.
Antes de reinicios automaticos definir limite de reintentos y mensajes claros;
no crear bucles que llenen logs o relancen repetidamente un cliente roto.

**Terminado cuando:** cada fallo de cliente previsto tiene resultado acotado,
el testigo sigue funcionando cuando corresponde y el siguiente arranque conserva
los datos esperados. Cuelgues del kernel y resets de GPU quedan en un banco
distinto; no se prometen recuperados por el supervisor de usuario.

**Salida:** pruebas de arranque y, solo con evidencia, cambios puntuales en
supervision/diagnostico. Reempaquetar e instalar A: como paso de promocion,
con respaldo y verificacion de hashes, conservando el almacenamiento personal.

## Organizacion de entregas y cierre

No mover todo el repositorio ni crear carpetas vacias. Mantener una entrada en
[Museo](../Museo/README.md), fuentes junto al area responsable y una carpeta nueva
de evidencia por ronda. Los logs de perfiles, cuentas y discos personales van
en `build/` o `privado/`, ignorados por Git; publicar solo resumen revisado.

Cada entrega tendra: problema concreto, regla, fixture invalido y control,
versiones/hashes, resultado, coste medido cuando corresponda, limites conocidos,
consumidor de produccion si existe y forma de volver atras. Un commit no mezcla
un cambio de ABI, un cambio de driver y una optimizacion del juego.

La primera entrega ejecutable sera **C: limites/cadenas y errores**, seguida
de **ASM: puente valido, pila y registros**. Las siete etapas quedan planificadas;
su implementacion y ejecucion se cerraran individualmente con evidencia.

## Referencias de reglas ya investigadas

- [Cobertura y limites de C](COBERTURA-UB.md), [museo C](MUSEO-UB.md), [catalogo nuevo](../Museo/catalogo.json).
- [Perfil C++ y limites](../C++/README.md), [plan grafico previo](PLAN-GUARDIANES-GRAFICOS.md), [pruebas Vulkan/EGL](GRAFICA-Y-NVME.md).
- [GNU extended asm](https://gcc.gnu.org/onlinedocs/gcc/Extended-Asm.html), [System V AMD64 psABI](https://gitlab.com/x86-psABIs/x86-64-ABI).
- [Khronos: validacion Vulkan](https://docs.vulkan.org/guide/latest/validation_overview.html), [Proton upstream](https://github.com/ValveSoftware/Proton).
