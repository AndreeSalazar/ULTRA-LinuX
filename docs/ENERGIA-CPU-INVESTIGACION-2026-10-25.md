# CPU, vatios y REDlauncher: investigación del 25 de octubre

## Resultado del informe físico

Se analizó `gpu-7.3.0-rc7-pc-g32c52ec87d62-nvidia-hyprland.txt` de A: con
`scripts/analizar_energia_informe.py`. El resultado público conserva el hash del
informe original, sin publicar sus UUID de GPU ni las rutas personales.

| Medida | Resultado | Interpretación |
|---|---|---|
| Muestras | 94 | Sesión mixta; no es una prueba aislada de reposo o de un juego |
| Duración por uptime | 608,49 s | Aproximadamente diez minutos |
| CPU ocupada, período completo | 58,131 % | Agregado de los 12 hilos; 100 % sería todo el procesador |
| CPU por intervalo | 2,324–81,766 % | No identifica qué proceso originó la carga |
| Potencia GPU | 14,49–169,52 W | NVIDIA; no son vatios CPU ni del enchufe |
| Temperatura GPU | 49–77 °C | No es temperatura CPU |
| VRAM usada | 1–5.154 MiB | Son muestras puntuales, no una reserva máxima garantizada |
| Potencia CPU | No capturada | No se deduce desde %, GHz, TDP o potencia GPU |

La media CPU del período se calcula con las diferencias de contadores acumulados,
excluyendo idle e iowait y sin sumar guest dos veces. No se promedian los
porcentajes de intervalos como si todos duraran lo mismo.

El reloj civil salta aproximadamente **18.000,74 s**, mientras uptime avanza
normalmente. Esto es compatible con una corrección del reloj durante la sesión;
no demuestra su causa exacta. No interpretar esas marcas UTC como cinco horas
de medición ni usarlas para calcular vatios. Para eso hace falta un reloj
monotónico.

El registro identifica Ryzen 5 5600X, `amd-pstate-epp`, gobernador `powersave` y
EPP `balance_performance`. El dmesg anuncia RAPL para dominios package y core.
Eso acredita detección del contador en el kernel; todavía hay que comprobar
lectura y permisos durante el arranque Linux.

## Cómo obtener los vatios CPU

La ruta preferida para este AMD es la PMU RAPL de perf, comprobando primero
`/sys/bus/event_source/devices/power/events/energy-pkg`, su escala/unidad y cpumask.
El [controlador del kernel](https://github.com/torvalds/linux/blob/master/arch/x86/events/rapl.c)
registra la PMU `power` y expone energía acumulada del paquete AMD. El antiguo
[`amd_energy` de hwmon fue retirado](https://github.com/torvalds/linux/commit/9049572fb145746725b198a19e27fa2671b80448)
en Linux 5.13; no basar el diseño en su antigua
ruta `energy*_input`.

Si `perf` está instalado y la PMU está disponible, una captura de diagnóstico
con permisos apropiados sería:

```sh
perf stat -a --per-socket -I 1000 -e power/energy-pkg/ -- sleep 60
```

Este comando es una propuesta para ejecutar dentro de ULTRA; no se ejecutó en
Windows. El manifiesto de la imagen actual no enumera el paquete `perf`: falta
incorporar esa herramienta o el lector propio, y comprobarlo en el arranque. El lector final
puede usar `perf_event_open` con eventos y duración fijos, sin ejecutar comandos
arbitrarios. El escritorio debe recibir resultados agregados, sin privilegios
para modificar MSR ni desactivar globalmente las restricciones de perf.

Potencia media: `W = diferencia_de_julios / diferencia_de_segundos_monotonicos`.
Un ejemplo de conversión, no una medición del PC: 60 J en 2 s son 30 W.
Registrar fuente, dominio, intervalo, errores, eventos habilitados/ejecutados y
multiplexación. Si no puede leerse, mostrar **NA**, nunca cero. No sumar package
y core: los núcleos ya forman parte del paquete. Medir el equipo completo requiere
un medidor del enchufe. Un contador energético disponible tampoco implica que
el firmware permita fijar un límite de potencia.

Referencias: [perf y permisos](https://www.kernel.org/doc/html/latest/admin-guide/perf-security.html),
[API perf_event_open](https://man7.org/linux/man-pages/man2/perf_event_open.2.html),
[powercap y sus dominios](https://www.kernel.org/doc/html/latest/power/powercap/powercap.html).

## Orden para optimizar sin perder fluidez

1. Capturar 60 s de escritorio quieto y 120 s de la misma escena del juego,
   manteniendo FPS, resolución y ajustes. Guardar energía CPU, GPU W, CPU por
   proceso, tiempos de cuadro p95/p99 y duración monotónica. Repetir A/B/A;
   separar compilación de shaders, actualizaciones y verificaciones del lanzador.
2. Comparar efectos vivos y reducidos. `borderangle` gira en bucle en
   `hyprland.lua`; produce redibujados cuando el borde está visible. El modo
   juego ya reduce efectos y pausa el fondo vivo, pero hay que verificar que
   realmente se activa para el proceso observado. Es una hipótesis medible,
   no una cantidad de vatios ahorrados demostrada.
3. Comparar el límite de FPS deseado con renderizado sin límite. Con la GPU cerca
   del 99 % y de 170 W en parte de esta sesión, es un candidato prioritario para
   consumo total. No afirmar que resuelve por sí solo la potencia de CPU.
4. Reducir trabajo de monitores ocultos: eww escucha datos cada 2 s y mantiene
   un `nvidia-smi -l 2`; durante juego sus datos pasan a 10 s, pero el sondeo de
   NVIDIA continúa cada 2 s. El reloj eww se actualiza cada segundo. Comprobar
   si siguen activos con las tarjetas ocultas antes de modificar su ciclo de vida.
5. Probar EPP `balance_power` solamente si aparece en
   `energy_performance_available_preferences`, conservando y restaurando los
   valores anteriores. En modo activo `powersave` no significa frecuencia mínima
   fija: la preferencia EPP guía las decisiones del hardware. Comparar a iguales
   FPS y frametimes antes de conservar el cambio. [Documentación AMD pstate](https://www.kernel.org/doc/html/latest/admin-guide/pm/amd-pstate.html).
6. Investigar límites de potencia soportados por firmware/BIOS si aún hacen falta.
   No aplicar un undervolt, un límite de TDP o escrituras SMU/MSR genéricas basándose
   solo en el nombre Ryzen; este informe no acredita soporte de esos controles.

## Qué aporta el Super Grafo

Ayuda a encontrar dependencias, rutas costosas y trabajo repetido en nuestro
código. F12 presenta tiempos de carriles, no energía del paquete. El grafo no
observa automáticamente el código interno de Cyberpunk ni transforma coincidencias
de UB en estimaciones de vatios.

Ya hay decisiones favorables: geometría retenida, redibujado por eventos y un
muestreador de procesos que duerme cuando no sigue ningún proceso. También hay
candidatos concretos para medir: `disco.rs` despierta cada 250 ms y revisa marcas
de tiempo aproximadamente cada segundo; F12 solicita un refresco cada 500 ms.
El modo diagnóstico tiene un coste y conviene cerrarlo al tomar una referencia
de reposo. En las muestras de procesos de este informe no aparece Carpetas por
su nombre; no puede atribuírsele la carga CPU observada.

## REDlauncher: cursor visible, clic sin respuesta

El usuario confirmó este síntoma después de jugar. La corrección previa de cursor
compuesto se dirige a su visibilidad, y no demuestra resolver foco o recepción de
clic. La regla actual solo fuerza posición flotante y centrada para dos clases.

Para la próxima captura: registrar clase, XWayland/Wayland, foco, escala y
geometría con REDlauncher abierto, además de comprobar si responde a Tab/Enter.
No exportar títulos o comandos de otras ventanas. El informe contiene procesos
`REDupdater.exe` y `REDEngineErrorR`; su mera presencia no prueba la causa del clic
ni identifica el error del juego.

Una corrección candidata, **todavía no aplicada**, es limitar
`allows_input = true` y `focus_on_activate = true` a la clase real de REDlauncher
si se confirma que es XWayland y falla el foco/aceptación de entrada. La
[documentación de reglas](https://wiki.hypr.land/configuring/core/rules/window-rules/)
distingue estas opciones del renderizado del cursor. No forzar foco o capturar
el ratón globalmente: afectaría a Heroic y al propio juego. Si es un bloqueo del
cliente, esas reglas no bastarán y habrá que revisar el registro Wine del lanzador.

## Entrega de esta investigación

Se guardan analizador, pruebas matemáticas y resumen sanitizado. No se cambió la
política energética, el juego, el lanzador ni la imagen instalada de A:. Esta
ronda investiga; la captura física de energía CPU y la corrección de entrada
siguen pendientes de validación en Linux.

[Resumen y pruebas](../evidencias/energia-cpu/2026-10-25/) ·
[Analizador reproducible](../scripts/analizar_energia_informe.py).
Los informes originales actuales se respaldaron y verificaron dentro del
proyecto Windows; no se publican como parte del resumen.
