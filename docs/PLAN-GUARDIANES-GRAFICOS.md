# Aplicacion de guardianes CPU, GLES y escritorio

2026-10-19. Plan aprobado por el propietario y primera ronda aplicada en el
laboratorio C++. El objetivo es un escritorio usable, recuperable y medido.
Cada etapa requiere evidencia antes de anunciar integracion en produccion.

| Etapa | Resultado exigido | Estado actual |
|---|---|---|
| 1. Base RTX | Arranque fisico, nvidia-smi, DRM, monitor y retorno a consola | Kernel/driver/RTX y nodos DRM confirmados en PC; render y sesion pendientes |
| 2. Sesion sin root | Preparacion privilegiada separada; compositor/cliente con UID, capacidades y fd comprobados | Cage/Foot migrados a UID 1000, capacidades cero y NNP; validar tambien sesion en RTX y ampliar aislamiento |
| 3. Guardianes CPU/C++ | Dimensiones, capacidad, overflow, presupuesto y propiedad antes de operar | Contratos RGBA8/rangos y parser probados; concurrencia y adopcion global pendientes |
| 4. Guardianes GL/EGL | Estado de API, vida de recursos y cargas acotadas | Texturas RGBA8, PBO/pixel-store, contexto y RAII probados en software |
| 5. .gpus dinamico | No perder programa anterior cuando falla el candidato | Reemplazo transaccional sincrono probado; compilacion asincrona pendiente |
| 6. Hyprland propio | Revision fija, build reproducible, cambios pequenos y regresiones | Pendiente de toolchain/dependencias completas e integracion |
| 7. Rendimiento | Comparacion reproducible y mejora sin regresiones | Sin porcentaje de aceleracion medido |

Actualizacion W3c: [Hyprland y aplicaciones](HYPRLAND-W3C.md) incorpora el
paquete oficial fijado y una configuracion propia. Las fotos de Cage/Foot
aportadas por el propietario muestran proveedor/driver EGL NVIDIA y salida
limpia; la primera preparacion de nodos fallo y se corrige en W3c. La etapa
6 avanza con una referencia upstream instalada, no con un fork del renderer
ni con los contratos C++ integrados. La validacion fisica de Hyprland sigue
pendiente y las etapas de aislamiento por aplicacion y rendimiento conservan
sus requisitos.

Etapas 3 a 5 avanzan en paralelo logico al arranque fisico: no requieren
exponer la RTX. Esto no implica que todos sus casos esten terminados.

[Evidencia real de NVIDIA](../evidencias/pc-real/nvidia-2026-10-19/README.md):
615.78.08 reconoce RTX 3060 y nvidia-drm se inicializa. En aquella imagen el
escritorio forzaba pixman; [W3b](W3B-SESION-GPU.md) agrega seleccion GLES.
El registro tambien exige revisar microcodigo/SRSO, audio
y aviso HDMI; no se certifica mitigacion completa ni aceleracion por la foto.

## Ronda aplicada

[Informe final](../evidencias/cxx/grafica-f277ba08607045a9b886f351111b0688/informe.txt).
G++ 16.2.1/C++26, perfil endurecido y perfil ASan/UBSan; ambos terminaron
con codigo 0. Sin diagnosticos de sanitizadores en esta ejecucion.

Por perfil:

- 3479 combinaciones de layout/rangos frente a oraculo independiente de
  128 bits, mas dos casos de limite de dimensiones. Los extremos numericos
  no reservan la memoria que describen.
- Parser .gpus: prefijos truncados, mutaciones de cada byte, maximos validos,
  entradas excesivas/vacias, 30 cabeceras adversas con checksum correcto y
  4096 entradas pseudoaleatorias de hasta 255 bytes. No es fuzzing exhaustivo.
- Nueve controles GLES: incluyen PBO y pixel-store previos adversos,
  restauracion de estado, lectura del pixel, rechazo sin contexto y RAII.
- Quince controles .gpus/programas: importacion, reconstruccion y candidato
  fallido que conserva el dibujo anterior; candidato valido cambia de verde
  a azul entre dibujos. glReadPixels sincroniza estos ensayos; no es una
  medicion de frames ni una actualizacion asincrona del escritorio.

El banco inicia un servicio transitorio con UID/GID del usuario del taller;
sudo solo solicita el cgroup al gestor del taller. Antes de bwrap se verifican
UID no cero, memory.max=536870912, memory.swap.max=0 y pids.max=64.
Dentro de bwrap se vuelve a comprobar UID no cero y ausencia de /dev/dri;
el informe registra UID 1000 antes y despues de entrar en los namespaces.
Tambien hay limite de tiempo de servicio (35 s), timeout del ensayo (30 s),
CPU (20 s), core desactivado y tmpfs de 64 MiB. Los binarios no reciben GPU,
red, home ni sockets de sesion. Esto comparte el kernel de la VM y /usr en
solo lectura: no acredita aislamiento frente a cualquier fallo de kernel.

## Contratos que debe respetar quien integre

[limites.hpp](../C++/produccion/limites.hpp) valida RGBA8 empaquetado, no
formatos arbitrarios ni multiplano. Un span presupone memoria valida durante
la llamada: la aritmetica no descubre un puntero falso ni una carrera externa.

[guard_gles.hpp](../C++/produccion/guard_gles.hpp) es una API optativa propia,
no un interceptor global. Exige contexto GLES3 actual y error GL previo limpio;
observa/consume errores de GL en su frontera. Valida antes de asignar textura
y neutraliza/restaura PBO y pixel-store de carga. Solo acepta RGBA8 empaquetado.
El contexto propietario debe sobrevivir al objeto. close rechaza otro contexto;
destruir una textura viva fuera del contexto propietario termina el proceso,
para evitar borrar un nombre de textura ajeno. El integrador debe cerrar
recursos antes de destruir contextos; la comparacion de handles no sustituye
ese contrato ni trata la reutilizacion de handles tras destruir un contexto.
No se ha probado aun toda transferencia entre hilos ni contextos compartidos.

La comprobacion de checksum .gpus no autentica contenido: el banco demuestra
que alguien que cambia contenido y recalcula checksum produce un contenedor
estructuralmente valido. No importar caches de terceros como si fueran fiables.

## Siguiente trabajo concreto

1. Sesion: medir credenciales de udev/seatd/compositor/cliente; separar usuario,
   runtime, capacidades y dispositivos. Mantener consola de recuperacion.
2. Build Hyprland: fijar commit y versiones; construir upstream sin parches,
   guardar manifiesto y ejecutar sesion de referencia antes de adaptar APIs.
3. Ampliar contratos: buffers GL, texturas con formatos adicionales, fd/SHM,
   DMA-BUF y sincronizacion. Cada ampliacion incluye control valido y fallo.
4. Reemplazo: contexto/hilo de preparacion, cola limitada, identificacion de
   variantes, uniforms y recursos, validacion y cambio en limite de frame.
   Conservar el anterior ante fallo y escribir cache privada atomicamente.
5. Medir referencia en RTX: resolucion/Hz/escala/VRR y versiones fijas;
   carga fria/caliente separada de tiempos de frame p50/p95/p99, tirones,
   uso CPU/GPU y memoria. Cambiar una variable por comparacion.

No se desactivan mitigaciones ni validaciones para mejorar una cifra. El
nombre guardian identifica contratos implementados y comprobados, no una
garantia de ausencia de UB, de fallos de driver o de todo tipo de ataque.
No se modificaron el escritorio instalado, el kernel ni A: en esta ronda.
