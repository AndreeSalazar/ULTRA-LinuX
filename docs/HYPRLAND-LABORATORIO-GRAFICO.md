# Hyprland: laboratorio grafico y escritorio de uso diario

2026-10-19. Peticion del propietario: investigar fallos por categorias,
aislar aplicaciones y mejorar fluidez sin perjudicar el escritorio ni juegos.
Este documento organiza trabajo pendiente: no instala Hyprland ni acredita
aislamiento de GPU. W3a esta instalado; la RTX aun requiere validacion real.

## Que usa cada capa

Wayland es el protocolo entre clientes y compositor. Hyprland tiene renderer
OpenGL ES; desde 0.50 exige al menos GLES 3.0. EGL gestiona contextos y
superficies. Aquamarine es su backend y se declara independiente de la API
de render. No confundir esto con que todo cliente tenga que usar OpenGL:
las aplicaciones pueden producir sus buffers mediante otras APIs.

Fuentes oficiales consultadas:
[Hyprland 0.50](https://hypr.land/news/update50/),
[renderer](https://github.com/hyprwm/Hyprland/blob/main/src/render/OpenGL.cpp),
[Aquamarine](https://github.com/hyprwm/aquamarine),
[DRM UAPI](https://docs.kernel.org/gpu/drm-uapi.html).
Fijar versiones y commits antes de implementar: main es una referencia mutable.

## Categorias y evidencia exigida

| Categoria | Casos del banco | Evidencia para aceptar |
|---|---|---|
| C/C++ en CPU | Desbordes de dimensiones, vida de buffers, conversiones, referencias y carreras | Caso minimo, resultado esperado, diagnostico y control valido |
| API GL/EGL | Contexto incompatible, uso fuera de estado y error de creacion | Error observado y recuperacion; un error de API no equivale a UB de C |
| Shaders | Compilacion fallida, limites y carga acotada | Log de compilacion, salida y tiempo limite; sanitizadores CPU no prueban el shader |
| Wayland/SHM | Desconexion, objeto destruido y almacenamiento truncado | Solo el cliente de prueba pierde servicio; cliente testigo sigue respondiendo |
| DMA-BUF | Formato, planos, stride, modificadores y sincronizacion | Importacion valida o rechazo, vida del buffer y ausencia de reutilizacion prematura |
| DRM/KMS | Cambio de modo, desconexion y retorno a consola | Recuperacion observada en hardware y registro del driver |
| Permisos | UID, capacidades, fd, sockets y dispositivos | Cada proceso conserva solo recursos declarados; casos permitidos y denegados |
| Recursos | Limites de RAM, fd, procesos y tiempo | Limite efectivo y terminacion del ensayo sin perder el entorno de control |

Extiende las categorias W-01 a W-12 de
[Wayland e investigacion C](WAYLAND-INVESTIGACION-C.md).
No instalar el manejador SIGBUS propio en libwayland sin resolver el
conflicto ya documentado. Hyprland es C++: requiere revisar su build y
contratos de C++, no aplicar automaticamente el perfil C17.

## Separacion propuesta

1. Produccion: compositor y aplicaciones sin root; preparacion privilegiada
   de sesion separada. Comprobar credenciales reales antes de declararlo hecho.
2. Cada cliente: proceso y politica propios, archivos y sockets concedidos,
   sin acceso general a entrada, KMS ni datos de otros clientes. Adaptar la
   caja para bibliotecas dinamicas, IPC y driver; medir antes de filtrar.
3. Laboratorio CPU/protocolo: entorno desechable sin datos personales,
   limites de recursos y compositor anidado de prueba. No usar el socket del
   escritorio diario. Empezar con render por software para reducir alcance.
4. Laboratorio GPU: maquina dedicada o entorno con GPU dedicada y recuperacion
   independiente. Una VM sin acceso a la RTX no prueba su driver; un compositor
   anidado que comparte GPU tampoco la aisla del kernel anfitrion.

GL/EGL y otras bibliotecas dentro de Hyprland comparten memoria y privilegios
con el compositor. Dividir el catalogo no crea fronteras de proceso. Separar
un componente a un servicio requiere disenar IPC, propiedad y validacion de
buffers. Un render node limita operaciones respecto a KMS, pero sigue dando
acceso al driver del kernel; no promete contencion completa de fallos GPU.

## Optimizar con comparaciones reproducibles

Primero registrar GPU/driver, kernel, compositor, resolucion, Hz, escala,
VRR, energia y carga. Verificar que el renderer real es la RTX y no software.
Guardar una referencia por escenario: escritorio quieto, mover ventanas,
video y juego. Comparar con la misma configuracion y duracion.

Medir tiempos de frame p50/p95/p99, frames perdidos, CPU/GPU, RAM/VRAM,
potencia y fallos. FPS no mide por si solo latencia de entrada: esta exige
medicion adicional. Repetir muestras y conservar dispersion.

Cambiar una variable por ensayo: desenfoque, sombras, animaciones, escala,
copias de buffers, sincronizacion o politica energetica. Registrar diferencia
absoluta y porcentual y revertir regresiones. No desactivar mitigaciones,
validaciones ni aislamiento para presentar una mejora de rendimiento.
Usar builds de diagnostico para detectar fallos y builds equivalentes de
produccion para rendimiento; no comparar una build instrumentada con otra
sin instrumentar como si fuera una optimizacion.

## Orden inmediato

Validar RTX en W3a; retirar root de la sesion; fijar Hyprland y dependencias;
arrancar y medir una referencia; crear el primer ensayo aislado con control
valido y cliente testigo. Despues ampliar categorias y convertir cada fallo
reproducido en regresion. No hay mejoras de velocidad medidas en esta fase.
