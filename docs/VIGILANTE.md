# VIGILANTE: observar antes de ajustar

El VIGILANTE informa. No es un guardian que deniega accesos, un detector de UB
ni una politica automatica del kernel. Aprovecha el observador de juegos del
escritorio, sin un segundo proceso de sondeo ni indexacion de fuentes al jugar.

## Que registra

- CPU por proceso de juego e I/O por intervalo del observador existente.
- Incrementos de fallos menores/mayores y cambios de contexto, identificando
  procesos por PID y tiempo de inicio. La primera muestra no inventa deltas.
- Eventos nuevos de `memory.high`, `max` y OOM, con identidad del grupo para
  evitar restar contadores de grupos distintos. No son FPS ni causa demostrada.
- Potencia del paquete CPU y temperatura desde el controlador de energia,
  solo si la muestra tiene como maximo cinco segundos de antiguedad.
- Presencia de MangoHud en `maps` de hasta 16 procesos de juegos identificados,
  y tres preferencias acotadas de su entorno. No exporta el entorno completo,
  tokens, argumentos, pilas ni archivos de cuentas.

`mapped=true` confirma una biblioteca observada, no que el HUD sea visible.
`false` significa no observado en ese proceso con lectura completa; puede
tratarse de un lanzador, no del proceso que presenta cuadros. `null` indica
lectura inaccesible o limitada. El VIGILANTE no deduce FPS de `/proc`.
Los fallos de pagina son contadores normales de memoria virtual, no pruebas
de comportamiento indefinido en C ni de una vulnerabilidad.

## Uso y retencion

En Super+Espacio: Juegos > FPS y HUD > **VIGILANTE: juegos, CPU y estado del HUD**.
Tambien `ultra-vigilante panel`. El panel comparte los datos del monitor vivo.
Se inicia automaticamente con el escritorio cuando dispone del home persistente.

El monitor toma muestras aproximadamente cada cinco segundos y guarda una
ventana de hasta 120 muestras en `~/.cache/ultra/vigilante.json`, como usuario,
con permisos 0600. Escribe cada treinta segundos, con limite de 1 MiB; no es
un archivo historico que crece indefinidamente. Pueden perderse los ultimos
treinta segundos al cerrar. La cadencia depende del coste de las consultas.

Al salir del escritorio, el informe GPU exporta esa ventana bajo `[VIGILANTE]`.
Comprueba el identificador de arranque y el inicio monotono de la sesion para
no atribuir al intento actual muestras viejas. `ultra-vigilante informe` permite
consultar la ventana manualmente. El estado en vivo sigue en `juegos-vivo.json`.

## Como sirve para optimizar

Primero se correlacionan intervalos y se formulan hipotesis: presion por limite
de memoria, lecturas durante cargas, trabajo de CPU o HUD no cargado. Despues se
elige un cambio reversible y se compara la misma escena, FPS y temperatura.
El kernel sigue aplicando sus politicas existentes: no aprende una politica
segura leyendo este JSON ni sabe semanticamente que trabajo necesita Warframe.
Para atribuir coste a funciones hacen falta perfiles de muestreo separados;
este observador no ve todas las instrucciones ni captura cada microtiron.

Las pruebas cubren reutilizacion de PID, reinicio de contadores, muestras
ausentes/caducadas, mapas inaccesibles/grandes, FIFO, privacidad de variables,
limites de historial y rechazo de otro arranque o sesion. El banco instalado
usa un proceso sintetico del usuario; no arranca juegos reales ni mide la RTX.

## Validacion de esta entrega

Diez pruebas del VIGILANTE y cinco del monitor de juegos pasaron en Linux,
ademas de la regresion del lanzador HUD. La imagen final completo 168
comprobaciones de arranque FAT32/UEFI en QEMU, incluido el VIGILANTE instalado
como UID1000, persistencia tras reiniciar y apagado limpio. Las evidencias y
los hashes de las fuentes probadas estan en `evidencias/vigilante/2026-10-26/`.
La prueba de MangoHud observa carga de bibliotecas; la visibilidad del HUD en
Warframe y sus FPS siguen pendientes de comprobar en el equipo fisico.

Instalada en A: para el perfil PC el 26 de octubre de 2026. El kernel y el
EROFS coinciden por SHA-256 con la entrega probada; `datos.ext4` conserva el
hash medido inmediatamente antes de instalar. La imagen EROFS mide
2058076160 bytes y cabe en FAT32. Se conserva el kernel previamente auditado;
esta entrega cambia el observador y sus informes, no las politicas de recursos.
