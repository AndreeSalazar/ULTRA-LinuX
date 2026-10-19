# W3e: escritorio real y consumo por perfil

La foto del propietario muestra Hyprland con dos ventanas y el tema ULTRA.
El informe fisico registra modo=nvidia, compositor=hyprland, salida=0,
RTX 3060 y los tres nodos NVIDIA correctos. Se conserva el original en
[evidencia](../evidencias/medicion-w3e/pc-real/).
El extracto no contiene el renderer GL ni contadores de memoria: no permite
deducir VRAM, RAM ni aceleracion porcentual a partir de la fotografia.

| Perfil fisico recibido | Sesion | RAM usada | VRAM usada durante Hyprland |
|---|---|---|---|
| generico | sin informe GPU | sin medir | sin medir |
| pc | NVIDIA/Hyprland, salida 0 | sin medir | sin medir |

El antiguo nvidia-smi fotografiado con 1 MiB, 19 W y 53 C corresponde a otra
instantanea sin procesos graficos listados; no describe este escritorio.
Los tiempos de arranques.txt miden deteccion/carga, no uso de memoria.

## Medicion nueva

ultra-medir-sesion se inicia tras preparar dispositivos y antes del cliente.
No inicia si falla la preparacion. El preparador detiene y espera al
observador antes de generar/exportar el informe. Maximo 120 muestras, una
pausa de cinco segundos entre ellas, hasta 128 procesos UID 1000 por muestra.
El limite temporal incluye ademas el coste de lectura y consultas acotadas;
no es una grabacion indefinida. Los ficheros quedan en un directorio root 0700.

- RAM del sistema: MemTotal, MemAvailable, diferencia estimada y swap.
- Procesos UID 1000: PID/nombre, RSS/PSS de smaps_rollup. No se suman RSS
  como memoria exclusiva; las sumas parciales indican cuantos se leyeron.
  Incluye todas las aplicaciones de ese usuario, no solo Hyprland.
- CPU: contadores agregados brutos; el resumen calcula ocupacion del sistema
  entre muestras, excluyendo idle/iowait y sin sumar guest dos veces.
- NVIDIA: UUID/PCI, VRAM total/usada en MiB, ocupacion, temperatura y W.
  Consulta sin root/capacidades/privilegios nuevos, timeout de dos segundos
  y kill adicional de un segundo. Solo si los tres nodos ya existen.
  NA/fallo de consulta no se convierte en cero ni en exito del renderer.
- Una consulta acotada de Hyprland tras unas tres pausas recoge systeminfo
  y monitores como UID 1000. Puede faltar si la sesion termina antes. En QEMU
  informa GL 3.2 y backend DRM, pero no una cadena GL_RENDERER: esa identidad
  exacta sigue requiriendo un registro que la publique.

La lectura PSS y nvidia-smi tienen coste y pueden alterar el reposo; las
comparaciones deben usar este mismo observador. No es un benchmark de FPS.
No recopila VRAM por aplicacion ni cubre picos entre muestras.

La definicion de MemAvailable y PSS procede de la
[documentacion procfs del kernel](https://www.kernel.org/doc/html/latest/filesystems/proc.html).
Las unidades y campos GPU proceden de
[NVIDIA SMI](https://docs.nvidia.com/deploy/nvidia-smi/index.html).

## Prueba sin escribir comandos largos

Arrancar cada perfil por separado. Pulsar `e`, seleccionar Hyprland, dejar
30 segundos el escritorio quieto, abrir las mismas aplicaciones y usarlas
otros 30 segundos. Cerrar la sesion con Super+Shift+M y salir del menu con Q.
En pc elegir NVIDIA; el generico conserva su recuperacion sin driver NVIDIA.
Mantener monitor/resolucion/Hz iguales y distinguir pruebas frias/calientes.

Se conserva gpu-<kernel>.txt para compatibilidad y ademas
gpu-<kernel>-<modo>-<compositor>.txt por combinacion. Guarda el ultimo intento
de cada combinacion, no un historial completo. Fallos siguen separados.
La copia exterior ocurre al salir del comando/menu: si pruebas varios modos
en un mismo menu, solo se exporta el ultimo. Salir con Q entre modos para
conservar cada comparacion en A:.
Desde Windows, scripts/resumir_medicion.py resume una carpeta de informes:
minimo/maximo muestreado de RAM, PSS completo de UID1000, VRAM, uso, C y W.
No mezclar datos QEMU/software con consumos de la RTX fisica.

## Resultado de esta ronda

Ambas imagenes compilaron y pasaron el arranque BMO/QEMU. El perfil PC pasa
91 comprobaciones. El generico pasa sus comprobaciones, incluidas las tres
nuevas de RAM/PSS/exportacion, con salida 0. La extraccion automatica del
informe completo se agrego despues de aquella prueba generica y se valido
en PC. Los cuatro archivos instalados en A: coinciden con sus SHA256 locales;
datos.ext4 conserva el hash previo y hay copia verificada de ambas imagenes
anteriores en build/antes-medicion-<id>.

En QEMU PC/software, cuatro muestras durante inicio y apertura de aplicaciones:
RAM estimada 84.96 a 283.83 MiB; PSS UID1000 0.60 a 377.43 MiB; tres deltas
CPU entre 74.25% y 99.80%. No es reposo estable ni consumo de la RTX.
PSS incluye paginas mapeadas de archivos que MemAvailable puede considerar
recuperables, y las lecturas de procesos no son simultaneas con meminfo:
no deben coincidir ni restarse como si fueran categorias excluyentes.
VRAM queda NA por no haber GPU NVIDIA en esta prueba. El informe confirma
fin_medicion muestras=4 y monitor virtual 1280x800 a 74.994 Hz.

## Fronteras C / C++ / GL que se comparan

| Frontera | Compatible bajo contrato | Conflicto que debe rechazarse o aislarse | Evidencia actual |
|---|---|---|---|
| C: tamanos y copia | buffer con longitud/capacidad validas | overflow, solape prohibido, vida acabada | guardian/laboratorio y minas; regresion de arranque |
| C++: RGBA8/rangos | dimensiones y presupuesto dentro del limite | negativos, desborde, exceso de capacidad | probar-contratos con oraculo de 128 bits |
| GL: carga CPU | span vivo, contexto correcto, estado restaurado | PBO/pixel-store heredado que cambia el significado del puntero | sonda-egl: controles validos y adversos |
| GL: propiedad | cerrar recurso antes del contexto propietario | borrar en contexto ajeno, destruir fuera del propietario | RAII optativo; hilos/handles reutilizados pendientes |
| .gpus | cache propia con identidad/longitud validas | truncado, formato ajeno, candidato invalido | sonda-gpus conserva el programa anterior |
| GCC/G++ | perfil propio de produccion y otro ASan/UBSan | aplicar banderas globales sin probar ABI/dependencias | perfiles separados existentes; no -Ofast global |
| GPU/compositor | dispositivos preparados, apps UID1000/caps0 | falta de control NVIDIA, permiso o renderer software inadvertido | nodos reales y pruebas QEMU; GL/monitores capturados, cadena de renderer pendiente |

Los contratos C++ siguen siendo APIs propias del laboratorio: no interceptan
todas las llamadas de Hyprland ni eliminan todo UB. Una rama if/else no
demuestra por si misma un cuello de botella GPU. Antes de tocar el renderer:
capturar referencia, identificar espera CPU/GPU/sincronizacion, cambiar una
sola frontera y comparar memoria y tiempos de frame. Ninguna aceleracion
global o garantia de ausencia de fallos se deduce de estas pruebas.
