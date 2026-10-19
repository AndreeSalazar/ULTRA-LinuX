# .gpus: dos modos, prototipo GLES

Idea del propietario: guardar trabajo GPU preparado y ofrecer un modo estatico
y otro dinamico que se adapte. Implementacion inicial de laboratorio,
2026-10-19. No es un formato que el driver reconozca directamente: el cargador
lee nuestro contenedor y usa glProgramBinary para entregar el binario al driver.

## Modos implementados

| Situacion | Estatico | Dinamico |
|---|---|---|
| Contenedor compatible y programa aceptado | Importa, sin compilar GLSL en nuestro codigo | Reutiliza el mismo binario |
| Ausente, truncado, corrupto o identidad distinta | Devuelve fallo sin compilar | Compila fuentes de confianza y regenera contenedor en memoria |
| Formato no admitido o programa rechazado por driver | Devuelve fallo | Intenta reconstruir desde fuentes de confianza |
| La compilacion falla | No se intenta | Falla; este prototipo no tiene programa anterior de reserva |

El dinamico funciona durante la carga, sin actualizacion asincrona. No corrige
automaticamente un shader mal escrito ni interpreta intenciones. Las fuentes
de confianza estan integradas en el ejecutable, nunca proceden del .gpus.
Una importacion exitosa evita nuestras llamadas glCompileShader/glLinkProgram;
no demuestra que el driver carezca de trabajo interno o compilacion diferida.

## Contenedor v1 experimental

Cabecera little endian de 24 bytes, serializada por campos; no se vuelca una
struct C++ con padding. Campos uint32:

| Offset | Campo |
|---:|---|
| 0 | Magic GPUS |
| 4 | Version, 1 |
| 8 | Formato binario devuelto por GLES |
| 12 | Longitud de identidad |
| 16 | Longitud del binario |
| 20 | FNV-1a32 del contenedor excluyendo estos cuatro bytes |

Despues van identidad y binario. Limites: identidad de hasta 64 KiB, binario
no vacio de hasta 1 MiB; longitud exacta sin cola extra. La identidad incluye
API, opciones del programa, GL_VENDOR, GL_RENDERER, GL_VERSION, version GLSL
y fuentes completas. Se compara exactamente. Son indicadores de compatibilidad,
no una identidad autenticada de GPU/driver; el driver decide si admite el programa.
El checksum detecta danos accidentales, no autentica contenido ni evita forjado.

[gpus.hpp](produccion/gpus.hpp) contiene serializador y parser acotado sin
dependencias graficas. Estar en produccion/ indica su separacion del banco;
esta API experimental no esta integrada ni desplegada en el escritorio.
La vista del parser presta memoria: el contenedor debe seguir vivo y sin
modificarse mientras se usa. Los helpers read32/write32 requieren rango valido;
no son lectores generales para datos sin validar.

## Evidencia

[Informe](../evidencias/cxx/grafica-63ceaccea02d4371b986358a7212b326/informe.txt):
13 controles .gpus por perfil (endurecido y ASan/UBSan), salida 0 y sin
diagnosticos de sanitizadores. Los cinco controles EGL anteriores tambien
siguen pasando por perfil. Mesa/llvmpipe software, sin acceso a GPU real.

La prueba enlaza shaders de vertice/fragmento, dibuja un triangulo que cubre
un pbuffer de 8x8 y verifica un pixel verde. Guarda 6738 bytes en
/tmp/control.gpus dentro del aislamiento, lee el archivo, destruye el contexto
y carga un programa nuevo en otro contexto del mismo proceso. Comprueba que
la ruta de importacion tiene contador de compilacion GLSL igual a cero.
No prueba ejecuciones independientes, persistencia entre reinicios ni uniforms.

Diez casos: vacio, cabecera truncada, magic incorrecto, version desconocida,
longitud excesiva, checksum incorrecto, identidad distinta simulada, formato
no admitido con checksum valido, bytes sobrantes y binario vacio. Cada uno
exige rechazo estatico y reconstruccion dinamica con pixel correcto.
No se simulo un fallo interno del driver al importar un binario de formato
valido; esa ruta tiene manejo de error pero queda pendiente de fixture.
El intento inicial de build rechazo una conversion de signo por -Werror;
se corrigio con una lista tipada de GLenum, sin relajar el perfil.

## Siguiente: actualizacion mientras se dibuja

Antes de conectar con Hyprland: fuentes/variantes identificadas, compilacion
aparte con contexto y sincronizacion adecuados, validacion del resultado,
reconfiguracion de uniforms y recursos, y cambio de programa en un limite
de frame. Conservar el programa anterior si falla el reemplazo. Limitar colas
y variantes y usar guardado atomico en cache privada con limites de archivo.
El lector de archivo actual solo comprueba el archivo generado por la prueba;
no es un importador de binarios externos. No cargar caches de terceros sin
un modelo de confianza e aislamiento definido.

## Rendimiento y alcance

La cache busca reducir preparacion del programa, no acelerar cada llamada
de API ni eliminar ramas de los shaders. Un if/else del host C++ se ejecuta
en CPU; dentro del shader el coste depende de divergencia, trabajo y hardware.
Eliminar ramas a ciegas puede aumentar trabajo, memoria o registros. Medir
carga fria/caliente por separado del tiempo de frame y la latencia.
No hay porcentaje de aceleracion medido aqui.

No hay backend Vulkan ni integracion Hyprland. No se modificaron Mesa,
driver NVIDIA, kernel ni archivos instalados en A:.
Fuentes: [program binaries](https://wikis.khronos.org/opengl/GlProgramBinary),
[cache Vulkan](https://docs.vulkan.org/guide/latest/pipeline_cache.html),
[divergencia en NVIDIA](https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html#branching-and-divergence).
La ultima referencia explica el hardware con CUDA; no es una medicion de
nuestros shaders GLES.
