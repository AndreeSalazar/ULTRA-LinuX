# Primera evidencia NVIDIA real de W3a

2026-10-19, America/Lima. Cinco fotos adjuntas del propietario, conservadas
sin editar, y registros leidos de A:/ULTRA/informe despues del arranque.
La captura nvidia-smi muestra 10:27:52; los archivos de informe tienen otra
hora de escritura y no se presentan como captura completa de esa consola.
fuentes.sha256 identifica las fotos y los registros conservados. En dmesg-pc.txt
se quitaron espacios finales para Git; el original permanece en A:.

## Confirmado

- Kernel 7.3.0-rc7-pc-g32c52ec87d62 en la foto.
- Modulos nvidia, nvidia_modeset y nvidia_drm cargados.
- nvidia-smi/KMD 615.78.08 reconoce GeForce RTX 3060, 12288 MiB.
- En esa muestra: P8, 53 C, 19 W, 1 MiB de VRAM y 0 % de uso; sin procesos
  GPU listados. Es una muestra en reposo, no un benchmark ni prueba de juego.
- card0 corresponde a simple-framebuffer; card1 y renderD128 aparecen bajo
  PCI 0000:29:00.0. card1 tiene conectores DP y HDMI-A.
- dmesg confirma inicializacion de nvidia-drm en minor 1.

## No demostrado por estas fotos

El propietario informo despues que ejecuto ultra-escritorio y olvido la foto.
Se conserva como testimonio de ejecucion, no como captura del renderer ni
validacion automatica de la sesion. La conclusion sobre pixman procede del
script instalado, no de interpretar la ausencia de foto.

No aparece una prueba de cage/Hyprland renderizando con la RTX, identificacion
GL_RENDERER NVIDIA, Vulkan, estabilidad bajo carga ni salida DRM activa del
monitor. Disp.A Off y ausencia de procesos describen ese instante; no bastan
para diagnosticar una averia. El script instalado fuerza WLR_RENDERER=pixman,
por lo que abrir el escritorio actual no prueba aceleracion GLES NVIDIA.

La version CUDA UMD anunciada por nvidia-smi no acredita toolkit instalado
ni una carga CUDA ejecutada. Los cuatro indicadores del arranque tampoco
demuestran que toda aplicacion dinamica use la caja ni guardianes C++ nuevos.

## Hallazgos del registro

- SRSO: "IBPB-extending microcode not applied!" y "Vulnerable: Safe RET,
  no microcode". Safe RET esta seleccionado; falta microcodigo que extienda
  IBPB segun el kernel. No confundirlo con los avisos objtool de NVIDIA ni
  afirmar cobertura total. Verificar microcodigo temprano adecuado y estado
  sysfs tras la siguiente build. No se cambio BIOS ni mitigaciones.
  [Interpretacion oficial](https://docs.kernel.org/admin-guide/hw-vuln/srso.html).
- snd_hda_intel 0000:29:00.1: "Cannot probe codecs, giving up". Revisar esa
  funcion de audio y su ruta HDMI/DP; no implica perdida de todo audio.
- nvidia-modeset avisa de una restriccion de pixel clock HDMI anunciada por
  RLD. Hace falta comprobar EDID, modo y frecuencia reales antes de ajustar.
- Cargar un modulo externo marca taint; esa linea no demuestra un ataque.

## Siguiente paso

Mantener la consola y el perfil generico de recuperacion. Preparar un modo
NVIDIA explicito, separado del modo software: seleccion de dispositivo por
identidad, EGL/GLES real, registro del renderer y prueba de retorno a consola.
Separar privilegios de sesion antes de adoptar ese modo como escritorio.
Los guardianes C++/.gpus siguen en laboratorio y necesitan integracion propia.
