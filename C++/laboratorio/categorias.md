# Catalogo inicial C++ y graficos

Todo pendiente salvo la sonda CPU que tenga evidencia de ejecucion.
Cada caso futuro necesita control valido, resultado esperado, timeout,
limites de recursos, versiones y log. Un error GL no se etiqueta como UB C++.

| Directorio logico | Alcance | Primer caso propuesto |
|---|---|---|
| CPU/memoria | Vida, limites, alineacion, aliasing y conversiones | Tamano de buffer valido y producto rechazado antes de reservar |
| CPU/objetos | RAII, movimientos, iteradores, referencias y excepciones | Cancelacion sin fuga ni doble liberacion |
| CPU/concurrencia | Carreras y propiedad entre hilos | Cola con control sincronizado; TSan en build independiente |
| API/egl | Configuracion, contexto y errores | Crear contexto headless y limpiar cada recurso tras fallo |
| API/gles | Estado, limites y perdida de contexto | Operacion invalida con error esperado y contexto aun utilizable |
| GPU/shaders | GLSL, compilador y ejecucion | Shader valido frente a error de sintaxis; carga pequena y acotada |
| IPC/buffers | Wayland, SHM y DMA-BUF | Desconexion de cliente con cliente testigo vivo |
| kernel/driver | DRM/KMS, fences y recuperacion | Solo GPU de pruebas y acceso de recuperacion independiente |

No ejecutar pruebas de GPU en el escritorio diario. El aislamiento CPU
con bwrap no contiene un fallo del driver del kernel si se concede una GPU.
ASan/UBSan CPU no instrumentan automaticamente GLSL ni el driver propietario.
