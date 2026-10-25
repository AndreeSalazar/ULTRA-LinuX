# Puntero de REDlauncher y siguiente medición de energía

El usuario confirmó que consiguió jugar Cyberpunk tras la recuperación,
pero el puntero no aparecía o no permitía pulsar cómodamente en REDlauncher.
El informe de la sesión mostraba `hardwareCursorsInUse: true`, NVIDIA y
XWayland. Esa coincidencia orienta una prueba de compatibilidad; no demuestra
por sí sola la causa del fallo de entrada.

## Configuración persistente

`/etc/ultra/hyprland.lua` selecciona `cursor.no_hardware_cursors = 1` cuando
`ultra-sesion` identifica el modo de render NVIDIA. El cursor pasa por la
composición del escritorio. Los demás perfiles conservan el modo automático
(`2`). No se fuerza el render por CPU del juego ni se cambia su API gráfica.
Se desactiva el ocultamiento por teclado, tacto e inactividad del compositor;
un juego aún puede solicitar ocultar su cursor durante el control de cámara.

REDlauncher y REDprelauncher reciben una regla por clase de ventana para abrir
flotantes y centrados. No coincide con `Cyberpunk2077.exe`: el juego conserva
la gestión normal de ventanas y pantalla completa. La regla solo se aplica si
Wine publica esas clases; debe contrastarse con la ventana real si falla.

Las dos decisiones están en el sistema EROFS, se empaquetan con huella de
fuentes y se vuelven a cargar al abrir Hyprland. No dependen de ejecutar un
comando manual en cada sesión. No se han sustituido partidas, prefijos,
instalaciones de Proton ni configuraciones personales de Heroic.

Referencias oficiales consultadas:
[opciones de cursor](https://wiki.hypr.land/configuring/core/config-options/#cursor)
y [reglas de ventana](https://wiki.hypr.land/configuring/core/rules/window-rules/).
La compatibilidad de sintaxis se comprueba con el Hyprland empaquetado, no solo
con la wiki de la versión de desarrollo.

## Validación y entrega

El banco `scripts/banco_puntero.py` ejecuta la política de ambos perfiles en
el intérprete Lua real, comprueba que acepta el modo compuesto y crea ventanas
XWayland ficticias para distinguir lanzador flotante de juego normal. Envía
un clic con el ratón virtual de QEMU y espera el evento ButtonPress en la
ventana del lanzador ficticio.
No ejecuta REDlauncher ni el juego y no demuestra visibilidad o clic correcto
en la RTX. La confirmación visual del usuario sigue siendo necesaria.

Pasaron 152 comprobaciones generales de la imagen final y las seis
comprobaciones específicas del puntero en una ejecución dirigida posterior.
La prueba específica se repitió tras corregir la instrumentación para usar
la API Lua de Hyprland 0.56.2. Se verificaron las nueve fuentes empaquetadas,
las huellas de sistema/BMO y que el kernel sigue siendo idéntico.
El empaquetador ahora resuelve su propia ruta antes de cambiar de directorio
y renueva el manifiesto sin acumular huellas antiguas duplicadas.

La entrega PC se copió a A: y se verificó su hash. La protección del instalador
rechazó primero un hash antiguo de `datos.ext4` sin escribir. Después se leyó
dos veces el estado actual, se respaldó y se conservó idéntico durante la copia.
El sistema instalado empieza por `7576002de6ffac76`; no se modificó la
partición de juegos ni se reconstruyó el kernel.

[Evidencias sanitizadas](../evidencias/puntero-launcher/2026-10-24).

## Warframe: verificación y tirón al abrir

La inspección posterior de solo lectura confirmó que el prefijo activo conserva
`EE.cfg` con la huella de la sesión recuperada. Proton Experimental, Sniper y
Steam Linux Runtime 4 figuran completos (`StateFlags=4`); esto no demuestra que
el contenido del juego esté actualizado ni que nunca necesiten actualizarse.

El `Launcher.log` reciente contiene operaciones `Hashing` sobre herramientas,
CEF y el ejecutable. El preprocesador recorre `Cache.Windows` y encuentra
40 bloques. Es comprobación de contenido del propio lanzador, distinta de la
descarga de Proton. Aparecen opciones WinHTTP no implementadas, pero esas líneas
no prueban por sí solas un fallo de Internet ni explican la pausa del escritorio.

Las 120 muestras del último informe GPU no identifican procesos Warframe/Steam;
no sirven para atribuirles el pico de CPU de esa sesión. El máximo de CPU
calculado entre muestras fue 88,3 %, y el mínimo de memoria disponible 7.765 MiB:
son datos del informe de otra carga, no una medición del tirón comunicado.

Para localizarlo se necesita una captura durante el arranque: pulsar Super+F11
justo antes de abrir Warframe y Super+Shift+F11 al notar la pausa. El recolector
existente guarda dos minutos de CPU/hilos, E/S, presión y GPU con límites de
duración y archivos. No elimina la comprobación del juego ni borra sus cachés.
La causa del tirón y su corrección quedan pendientes de esa evidencia.

El soporte oficial describe la
[verificación y optimización de caché](https://support.warframe.com/hc/en-us/articles/212890418-What-should-I-do-if-my-game-keeps-crashing)
como operaciones del propio lanzador. No se ejecutaron reparaciones ni descargas
autenticadas durante esta inspección.

## Próxima fase: CPU y vatios

El informe actual recoge `scaling_governor=powersave` y
`energy_performance_preference=balance_performance`. En este contexto,
`powersave` no significa necesariamente frecuencia mínima constante. No se
han cambiado estos parámetros en la corrección del puntero.

Antes de optimizar:

1. Medir el escritorio en reposo y la misma escena de juego con FPS limitados
   de forma idéntica: CPU por proceso, tiempos de cuadro p95/p99, temperaturas,
   frecuencias, GPU y energía si el equipo expone un contador válido.
2. Distinguir vatios del paquete CPU, vatios de GPU y consumo del PC en el
   enchufe. Un porcentaje de CPU no se convierte directamente en vatios.
3. Comparar efectos del escritorio y perfil energético cambiando una sola
   variable. El cursor compuesto puede añadir composición al mover el ratón;
   su coste debe medirse, no suponerse nulo.
4. Conservar solo cambios que reduzcan consumo sin empeorar la fluidez medida.
   Si no hay sensor de potencia CPU utilizable, marcar el dato como no
   disponible y recurrir a un medidor externo para el consumo total.

No se prometen menos vatios ni más FPS sin esa comparación en el PC físico.
