# OpenGL ES y Vulkan: vida util comprobada

Fecha: 2026-10-22, America/Lima. Objetivo: mejorar el banco de correccion antes
de modificar el escritorio diario. Esta entrega cambia sondas y su ejecutor;
no cambia el renderer de Hyprland ni las imagenes instaladas en A:.

Entrega posterior: [actualizar texturas y reutilizar envios Vulkan](GRAFICA-REUTILIZACION.md).
Los resultados y pendientes de este documento describen la primera entrega;
la ampliacion ya incluye trabajo valido en cola y fences en software.

## Cambios y resultado

El ejecutor `probar-grafica-cxx.ps1` ahora construye y ejecuta tambien Vulkan.
Antes solo incluia contratos CPU, EGL/GLES y la cache experimental .gpus.

En GLES se agregan dos controles:

- G-10: cambiar a otro contexto EGL, rechazar `close()` y comprobar despues
  que la textura sigue perteneciendo al contexto original. El contexto original
  se restaura tambien al salir por una excepcion del control.
- G-11: 256 ciclos crear/mover/destruir. El objeto movido queda sin nombre,
  cerrarlo no libera la textura del nuevo propietario y, al terminar el ambito,
  `glIsTexture` deja de reconocer el nombre. Un pixel verde posterior comprueba
  que el contexto sigue funcionando.

Vulkan ahora recorre 64 ciclos de crear buffer, obtener requisitos, elegir
memoria HOST_VISIBLE/HOST_COHERENT compatible, asignar, vincular en offset cero,
mapear y comparar un patron de 4096 bytes. RAII desmonta el mapeo, destruye el
buffer y libera la memoria, tambien si falla alguna operacion intermedia.
La asignacion individual se rechaza si requiere mas de 1 MiB.
Este patron se escribe y lee desde CPU: no es una transferencia ni un dibujo GPU.

Se corrige una decision prematura: antes se fijaba el resultado antes de
destruir el dispositivo y la instancia. Ahora se cuentan los errores hasta el
final y solo se permite exactamente un error previsto, el VUID
`VUID-VkBufferCreateInfo-size-00912`, dentro de la llamada negativa concreta.
Cualquier otro error, incluido ese mismo VUID fuera de esa llamada, hace fallar
la sonda. La capa aborta la llamada invalida antes del driver.

El modo exclusivo del laboratorio `--control-error-final` genera otro rechazo
despues del veredicto preliminar: el ejecutor exige salida 1 y los contadores
`errores=2 esperados=1 inesperados=1`. Ese ROJO dentro del informe es deliberado;
si no aparece el rechazo exacto, el banco completo falla.

## Evidencia

[Informe completo](../evidencias/cxx/grafica-a96800f81d6348c19f74a88a417efef2/informe.txt),
con hashes de fuentes, paquetes y opciones de compilacion. Ejecutor final: salida 0.
Ambos perfiles, O2 y ASan/UBSan O1, completaron los controles GLES y Vulkan;
el control negativo adicional se ejecuto en O2. Los controles previos de .gpus
tambien pasaron. No aparecieron diagnosticos de ASan, UBSan o LSan en esta ronda.

Backend: Mesa 26.2.5, llvmpipe LLVM 23.1.3, EGL 1.5, OpenGL ES 3.2.
El taller QEMU tiene su propio kernel Arch; no es un arranque de la imagen ULTRA.
Cada proceso trabaja sin dispositivos GPU, red ni sockets del escritorio,
con UID sin root, memoria limitada a 512 MiB y tiempo acotado.

Se conserva el [aviso historico de 512 bytes](../evidencias/grafica-nvme-2026-10-21/laboratorio/vulkan-diagnostico-inicial.txt).
El aislamiento ya mantenia las bibliotecas Vulkan cargadas para atribucion LSan,
con `detect_leaks=1`; esta entrega no cambia esa opcion ni agrega supresiones.
Que esta configuracion termine sin avisos no demuestra que se haya corregido
aquella asignacion interna ni que otras configuraciones carezcan de fugas.
Los nombres GL liberados tampoco miden cuando el driver devuelve VRAM fisica.

## Lo siguiente, por prioridad

1. Vulkan: enviar trabajo valido en software, esperar una fence con plazo y
   verificar el contenido resultante. Despues comprobar reutilizacion y
   dependencias de recursos. Las pruebas actuales no ejercitan colas ni fences.
2. GLES: comprobar destruccion/recreacion completa de contextos y rutas de
   error del programa de shaders. La perdida real de contexto sigue pendiente.
3. Medir CPU, RAM, VRAM y tiempos de fotograma en la RTX con una carga repetible,
   comparando la misma escena antes/despues. Primero sin tocar banderas globales.
4. Integrar solo mejoras justificadas en un consumidor real. La clase Texture
   y el formato .gpus siguen siendo candidatos del banco, sin integracion en
   Hyprland, NVIDIA, Proton ni los juegos.

Referencias oficiales: [contexto OpenGL](https://www.khronos.org/opengl/wiki/Get_Context_Info),
[referencia OpenGL ES](https://registry.khronos.org/OpenGL-Refpages/),
[Vulkan: destruccion del dispositivo y sus hijos](https://docs.vulkan.org/spec/latest/chapters/devsandqueues.html#VUID-vkDestroyDevice-device-05137).
