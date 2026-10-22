# Museo de C, C++ y ASM

Entrada comun de investigacion. Generada desde `catalogo.json`; no mueve ni modifica el sistema de juegos.

## Mapa de carpetas

| Area | Donde estudiar y comprobar |
|---|---|
| C existente | [24 fichas y sus evidencias](../docs/MUSEO-UB.md), [cobertura](../docs/COBERTURA-UB.md), [laboratorio](../src/guardian/laboratorio/) |
| Ampliacion C, C++ y ASM | Este indice y su catalogo; propuestas pendientes de fixtures |
| C++ propio | [Produccion y laboratorio separados](../C++/README.md) |
| ASM | [Alcance x86-64 y reglas](../ASM/README.md) |
| Generar y validar | `python scripts/museo-sistemas.py`; `--comprobar` exige indice actualizado |

Ampliacion: **14 fichas estudiadas**: C=3, C++=5, ASM=6. No suman nuevas detecciones a las 62 minas del banco C existente.

## Estados y alcance

Todas las fichas nuevas son ESTUDIADA: explicacion y proxima prueba, sin fixture nuevo ni rechazo acreditado. El catalogo rechaza etiquetas de aprobado sin evidencia. Las pruebas del generador validan datos y clasificacion, no ejecutan UB.

UB exige lenguaje, edicion y precondiciones. ASM se clasifica por ISA, ABI, contrato del compilador o concurrencia; un page fault normal no es UB. Los enlaces al borrador vivo de C++ y al manual actual de GCC pueden cambiar: un banco ejecutable debera registrar compilador, flags, revision de referencia, arquitectura y binario.

El objetivo es ampliar cobertura de forma verificable. No se promete agotar todas las combinaciones ni eliminar UB globalmente.

## C

### C-001: Liberar un puntero interior

**UB / ESTUDIADA**. Objetivo: C17; malloc/free de ISO C.

free exige NULL o el puntero devuelto por una reserva compatible todavia no liberada; p+1 no cumple ese contrato.

**Control valido:** Conservar el propietario original y liberar una sola vez ese puntero, no las vistas interiores.

**Proxima prueba, pendiente:** Fixture aislado de free(p+1), con reserva comprobada; control free(p).

**Limite:** ASan puede detectar este caso instrumentado; no convierte todos los alias en propietarios seguros.

Referencia: [7.22.3.3; numeracion del borrador C11, regla contrastada para C17](https://www.open-std.org/jtc1/sc22/wg14/www/docs/n1570.pdf).

### C-002: Arreglo variable con longitud no positiva

**UB / ESTUDIADA**. Objetivo: C17; VLA de longitud evaluada en ejecucion.

El valor evaluado de una longitud VLA debe ser mayor que cero. Separar este caso de un arreglo constante cero rechazado por la traduccion ISO.

**Control valido:** Rechazar n <= 0 antes de declarar el VLA; imponer tambien un limite de recursos o preferir reserva comprobada.

**Proxima prueba, pendiente:** n recibido en ejecucion: cero y negativo frente a uno y al limite permitido; probar O0 y O2 con vla-bound.

**Limite:** Cumplir la regla de longitud no garantiza que exista pila suficiente.

Referencia: [6.7.6.2 p5](https://www.open-std.org/jtc1/sc22/wg14/www/docs/n1570.pdf).

### C-003: Llamar mediante un tipo de funcion incompatible

**UB / ESTUDIADA**. Objetivo: C17; llamada indirecta.

Convertir un puntero de funcion no hace compatible su firma. Invocar una funcion mediante un tipo incompatible produce UB.

**Control valido:** Usar un typedef de firma compartido y un adaptador con conversiones explicitas de valores.

**Proxima prueba, pendiente:** Dos unidades con firma correcta y firma incompatible, con y sin LTO; distinguir diagnostico de rechazo en ejecucion.

**Limite:** Que los registros parezcan coincidir en x86-64 no satisface la regla de tipos de C.

Referencia: [6.5.2.2 p9 y 6.3.2.3 p8](https://www.open-std.org/jtc1/sc22/wg14/www/docs/n1570.pdf).

## C++

### CXX-001: string_view conserva la vista, no el string

**UB / ESTUDIADA**. Objetivo: C++20/23; acceso a caracteres despues de destruir el propietario.

Una vista sobre un string temporal no prolonga su vida. Acceder a sus caracteres despues de destruirlo usa un objeto cuya vida termino.

**Control valido:** Conservar un std::string propietario durante todos los usos de la vista, o devolver el propietario.

**Proxima prueba, pendiente:** Lectura posterior a la expresion completa, con variantes de cadena corta y larga; comparar con propietario vivo.

**Limite:** Construir o guardar una vista colgante no equivale por si solo al acceso invalido; ASan no acredita todos los casos.

Referencia: [[basic.life]; enlace al borrador vivo, no a una edicion publicada fija](https://eel.is/c++draft/basic.life).

### CXX-002: Referencia invalidada por crecimiento de vector

**UB / ESTUDIADA**. Objetivo: C++20/23; vector con realocacion garantizada.

Una realocacion invalida referencias, punteros e iteradores anteriores. Usarlos para acceder al elemento despues de realocar es invalido.

**Control valido:** Guardar un indice y recuperar la referencia despues del cambio; validar que el indice siga designando el elemento requerido.

**Proxima prueba, pendiente:** Llenar hasta capacity y agregar otro elemento para forzar realocacion; control que recupera la referencia nueva.

**Limite:** reserve aislado no promete estabilidad ilimitada ni evita otras invalidaciones por erase o insert.

Referencia: [[vector.modifiers]](https://eel.is/c++draft/vector.modifiers).

### CXX-003: new[] y delete no son una pareja

**UB / ESTUDIADA**. Objetivo: C++20/23; asignacion ordinaria de un arreglo.

Un arreglo obtenido por new[] no se libera mediante delete escalar.

**Control valido:** Emparejar new[] con delete[] o expresar propiedad con std::unique_ptr<T[]> / std::vector<T>.

**Proxima prueba, pendiente:** Arreglo de objetos con destructor: delete escalar frente al liberador correcto; control de numero de destrucciones.

**Limite:** RAII requiere escoger el tipo y liberador correctos; el nombre unique_ptr no valida cualquier construccion.

Referencia: [[expr.delete]](https://eel.is/c++draft/expr.delete).

### CXX-004: Una carrera de datos no se arregla con volatile

**UB / ESTUDIADA**. Objetivo: C++20/23; accesos conflictivos no atomicos sin happens-before.

Accesos conflictivos en hilos distintos, al menos uno no atomico, sin el orden requerido pueden constituir una carrera de datos y UB.

**Control valido:** Proteger todo acceso al estado compartido mediante el mismo mutex, o demostrar un protocolo atomico adecuado.

**Proxima prueba, pendiente:** Dos hilos sobre un contador compartido frente al contador protegido; TSan separado de ASan y controles del resultado.

**Limite:** Una ejecucion sin aviso TSan no demuestra ausencia de carreras; un algoritmo con atomicos puede seguir siendo incorrecto.

Referencia: [[intro.races]](https://eel.is/c++draft/intro.races).

### CXX-005: f(i++, i++): fijar el dialecto cambia la clasificacion

**NO_ESPECIFICADO / ESTUDIADA**. Objetivo: C++17/20/23; i inicia en cero; f no agrega efectos incompatibles.

Desde C++17 las inicializaciones de parametros estan indeterminadamente secuenciadas entre si; no se fija cual incremento ocurre primero. En este contexto no es UB por esos dos incrementos.

**Control valido:** Separar ambos incrementos en sentencias si importa que f reciba primero cero y despues uno.

**Proxima prueba, pendiente:** Contrastar C17 con C++17/20/23 y no convertir un warning heredado en prueba normativa de UB.

**Limite:** No trasladar esta conclusion a i++ + i++, a C17 ni a casos con desbordamiento.

Referencia: [[expr.call]; contraste de secuenciacion por edicion](https://eel.is/c++draft/expr.call).

## ASM

### ASM-001: Alineacion de pila en las llamadas

**ABI / ESTUDIADA**. Objetivo: x86-64 Linux; System V AMD64, argumentos ordinarios.

El llamador debe satisfacer la alineacion exigida por la ABI en el punto de llamada; los argumentos vectoriales pueden exigir condiciones adicionales.

**Control valido:** Contabilizar pushes y reservas, respetar la alineacion de la llamada y restaurar rsp al volver.

**Proxima prueba, pendiente:** Puente C/ASM que comprueba alineacion en la entrada del callee y verifica la pila al retornar, en una VM.

**Limite:** No generalizar la regla a Windows x64, interrupciones o todas las firmas vectoriales. Es contrato ABI, no una categoria ISO de UB de ASM.

Referencia: [Function Calling Sequence; The Stack Frame](https://gitlab.com/x86-psABIs/x86-64-ABI).

### ASM-002: Registros que debe conservar el callee

**ABI / ESTUDIADA**. Objetivo: x86-64 Linux; System V AMD64.

Una funcion que altera registros no volatiles debe restaurarlos segun la ABI; el llamador puede depender de esos valores.

**Control valido:** Guardar y restaurar los registros no volatiles utilizados; mantener la informacion de desenrollado correspondiente.

**Proxima prueba, pendiente:** Harness con valores centinela en rbx y r12-r15 antes y despues de una llamada ASM.

**Limite:** El harness tambien debe respetar la ABI; los registros y reglas no se copian entre arquitecturas.

Referencia: [Registers and the Stack Frame](https://gitlab.com/x86-psABIs/x86-64-ABI).

### ASM-003: Entradas, salidas y clobbers incompletos

**CONTRATO_GCC / ESTUDIADA**. Objetivo: GNU extended asm en C/C++; GCC.

El compilador solo conoce los efectos declarados. Un operando modificado necesita una salida o un operando read/write apropiado; los registros y flags alterados requieren declaracion correcta.

**Control valido:** Describir operands, matching constraints, early-clobber cuando corresponda y cc para flags; preferir intrinsecos cuando expresen el contrato.

**Proxima prueba, pendiente:** Comparar el resultado esperado a O0/O2 y con LTO, inspeccionando tambien el ensamblado generado.

**Limite:** asm volatile no corrige automaticamente constraints equivocadas; una coincidencia en O0 no valida el contrato.

Referencia: [Input/Output Operands; Clobbers and Scratch Registers](https://gcc.gnu.org/onlinedocs/gcc/Extended-Asm.html).

### ASM-004: Clobber memory no equivale a fence de CPU

**CONCURRENCIA / ESTUDIADA**. Objetivo: GCC inline asm y modelo de memoria C/C++.

El clobber memory comunica efectos de memoria al compilador, pero no emite por si solo una barrera hardware ni establece el protocolo atomico del lenguaje.

**Control valido:** Usar atomicos del lenguaje con el orden requerido; si se necesita ASM, documentar por separado compilador, CPU y modelo de memoria.

**Proxima prueba, pendiente:** Controles con atomicos y un litmus justificado para la arquitectura; no interpretar una prueba de estres como demostracion universal.

**Limite:** Ni volatile ni un fence aislado eliminan una carrera de datos sobre accesos no atomicos.

Referencia: [memory clobber](https://gcc.gnu.org/onlinedocs/gcc/Extended-Asm.html).

### ASM-005: Excepcion de division con divisor no nulo

**ISA / ESTUDIADA**. Objetivo: x86-64; instrucciones DIV/IDIV.

DIV/IDIV pueden producir #DE si el cociente no cabe en el destino, ademas del caso divisor cero. La division entera de la ISA no se clasifica como el operador de C.

**Control valido:** Preparar correctamente el dividendo ancho y comprobar rango y divisor antes de la instruccion.

**Proxima prueba, pendiente:** Un caso de cociente fuera de rango y controles con limites validos, capturados solo en laboratorio aislado.

**Limite:** La senal observada depende del SO y de su manejo de excepciones; no es prueba de cobertura de UBSan dentro de ASM.

Referencia: [SDM volumen 2: DIV/IDIV; excepcion #DE](https://www.intel.com/content/www/us/en/developer/articles/technical/intel-sdm.html).

### ASM-006: Un page fault atendido no demuestra un bug

**CONTRASTE / ESTUDIADA**. Objetivo: x86-64 Linux; memoria virtual.

Un #PF puede formar parte de la paginacion normal y ser atendido por el kernel; accesos invalidos tienen otro resultado segun sus condiciones.

**Control valido:** Medir fallos menores/mayores, E/S y tiempos de fotograma por separado; conservar los detalles de una senal si existe un acceso invalido.

**Proxima prueba, pendiente:** Primer acceso valido a paginas frente a repeticion; clasificar los contadores sin exigir cero fallos de pagina.

**Limite:** No obtener un SIGSEGV no demuestra ausencia de UB. Estos contadores no se recogieron en la sesion de Warframe analizada.

Referencia: [ru_minflt y ru_majflt](https://www.gnu.org/software/libc/manual/html_node/Resource-Usage.html).

## Como pasa una ficha al banco

1. Fijar una regla, dialecto y precondiciones; escribir tambien un control valido.
2. Crear un fixture minimo en el laboratorio correspondiente, con resultado esperado y limites de tiempo/recursos.
3. Ejecutar los casos invalidos solo en un banco desechable sin discos personales. Registrar tambien controles aceptados.
4. Conservar versiones, flags, hashes y evidencia de cada mecanismo. Un diagnostico no equivale a aislamiento; un silencio no prueba ausencia de UB.
5. Llevar a produccion un cambio concreto revisado, nunca el conjunto de minas o banderas globales por defecto.
