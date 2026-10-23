# De contadores a intervalos: CPU, RAM y almacenamiento

2026-10-23, America/Lima. Continuacion del
[lector C con Custodia](RECURSOS-C-PRODUCCION.md).

El resumen existente `scripts/resumir_medicion.py` incorpora ahora el campo
`recursos`, sin sustituir RAM/PSS/CPU/GPU anteriores. Acepta un archivo o una
carpeta de informes; solo lee sus entradas y entrega JSON. No cambia sysctls,
frecuencias, scheduler de disco, cache ni archivos de juegos.

## Que faltaba realmente

Los kernels generico #51 y PC #37 tenian `CONFIG_PSI` desactivado. Sus informes
mostraban NA porque no existia `/proc/pressure`, no porque la presion fuera
cero. Se cambia la construccion para exigir `CONFIG_PSI=y`, desactivar
`PSI_DEFAULT_DISABLED` y comprobar CPU/memory/io durante el arranque.
PSI aporta observabilidad y tiene coste de instrumentacion; esta ronda no
lo presenta como una aceleracion ni cuantifica su impacto en FPS nativos.

Tambien se corrigio un error reproducido del lector C: ante `/dev/full`, la
salida pendiente podia fallar al terminar y aun devolver exito. Ahora vacia
stdout explicitamente y devuelve 1 si falla. La prueba usa el dispositivo
sintetico del taller, sin escribir discos personales. Se mantienen los
cuatro perfiles, limites de lectura y pruebas de asignaciones fallidas.

La inspeccion HEX de main se integra en el archivo de auditoria de cada
construccion. La validacion anterior a copiar A: exige que ese informe
corresponda al SHA256 del lector y tenga los bytes contrastados con ELF.
La inspeccion de bytes no equivale a una prueba de toda ejecucion posible.

## Como interpreta el informe

- Calcula el tiempo entre capturas desde uptime, no desde la hora de pared
  ni suponiendo que una pausa solicitada de cinco segundos dura exactamente
  cinco. Mantiene separados reinicios de reloj y sesiones de medicion.
- VM: diferencias de contadores como pgmajfault, reclaim, refaults y
  compactacion. Una pagina que requiere E/S puede producir un major fault
  normal; no significa UB ni corrupcion de punteros.
- PSI: incremento de `total` dividido por microsegundos transcurridos.
  CPU full del sistema se deja sin valor porque su semantica es indefinida;
  no lo interpreta como ausencia de esperas de CPU.
- Disco: sectores de 512 bytes por tiempo, lecturas/escrituras completadas,
  espera media de esas operaciones y actividad aproximada. No confunde el
  tamano logico del bloque con las unidades del contador de sectores.
- `in_flight` es un valor instantaneo y puede bajar. Los demas campos se
  tratan como acumulados; un descenso invalida el intervalo de ese dispositivo.
  Acepta formatos conocidos de 11, 15 y 17 campos, sin mezclarlos.
- Cada dispositivo mantiene su resultado: loop, device-mapper y NVMe no se
  suman como si fuesen discos independientes. Un mismo nombre no garantiza
  identidad estable tras hotplug; esa limitacion queda en el estado calculado.

Las definiciones de [estadisticas de bloque](https://docs.kernel.org/block/stat.html)
y [PSI](https://docs.kernel.org/accounting/psi.html) se contrastaron con la
documentacion primaria y Kconfig del kernel fijado.

Una media de esperas de bloque no es p99 ni latencia exclusiva del SSD.
Actividad de disco tampoco prueba saturacion o causa de un tiron del juego.
Un cero derivado de tiempos redondeados en milisegundos no demuestra latencia
fisica cero; el resumen conserva las unidades del contador.
El contador ponderado permite expresar solicitudes pendientes promedio en
el tiempo, no la capacidad maxima de una cola. No se crean rankings de
cuellos de botella a partir de esos valores aislados.
Los porcentajes PSI de distintos recursos pueden solaparse; tampoco se
suman some y full como si fuesen tiempos independientes.

## Ausencia y limites

Sin datos, formato cambiado, contador reiniciado o campo duplicado, las
tasas quedan sin valor o con estado explicito. No se convierten a cero.
Capturas incompletas, multiples bloques de recursos para una muestra,
relojes ambiguos e intervalos entre sesiones no se usan para calcular tasas.
Una lectura no es atomica entre CPU, RAM y todos los dispositivos.
Una captura completa significa que se recogio el bloque del protocolo hasta
su cierre; no demuestra que incluya todos los dispositivos o recursos del PC.

Limites: 32 MiB por informe, 64 informes por consulta, 512 muestras por
informe, 32 dispositivos por muestra, claves/contadores acotados y lineas
mayores que 4096 caracteres invalidan la captura. Conserva la compatibilidad
con informes Windows CRLF y los informes antiguos sin estos contadores.

## Pruebas y siguiente decision

Fixtures con cambios conocidos verifican conversiones, tiempos, cero real,
campos ausentes, duplicados, reset, cambios de formato, cola decreciente,
capturas incompletas, sesiones separadas y limites. Pasa la regresion del
resumen anterior. No se necesitan credenciales ni discos de juegos.

En las capturas QEMU anteriores a PSI se interpretan 25 intervalos completos
de cada perfil; la presion sigue sin valor. El informe fisico previo tiene
39 muestras pero ninguna captura completa de recursos: sus tasas no se
inventan. Los hashes de entrada permanecen iguales despues del analisis.

Falta una sesion fisica con la imagen PSI y, para atribuir tirones, tiempos
por fotograma de la misma carga. Solo entonces corresponde elegir un ajuste
de CPU, memoria o E/S, comparar referencia y candidato y conservar vuelta
atras. Un contador global positivo no autoriza a culpar a Warframe o Brave.

Uso desde el proyecto Windows:

```powershell
python scripts/resumir_medicion.py A:/ULTRA/informe
```

El resultado distingue `null` de cero. No requiere reconstruir la imagen
para volver a interpretar informes guardados; habilitar PSI en el kernel si
requiere reconstruccion y un nuevo arranque.

La medicion del escritorio empieza al abrir la sesion y esta limitada a
120 muestras, con pausas minimas de cinco segundos mas el tiempo de consulta.
Para la siguiente prueba nativa conviene abrir una sesion nueva antes de
iniciar la carga. Al cerrar el escritorio, el supervisor espera al medidor
y exporta el informe a A:/ULTRA/informe. No se presupone que una partida
iniciada mucho despues de esa ventana quedara registrada.

[Capturas anteriores a PSI](../evidencias/recursos/intervalos-2026-10-23/)
y [ronda PSI y errores de salida](../evidencias/recursos/psi-recursos-0763a3a5b52844b985d44dd75d47414e/)
conservan resultados separados; sus condiciones QEMU no representan el
Ryzen completo con 16 GiB ni el almacenamiento fisico.

## Resultado de despliegue

Generico #52 y PC #38 estan instalados en A:, con fuentes y hashes de
kernel/EROFS verificados. El genérico pasa 156 comprobaciones y el PC 157,
cero fallos, incluidas las interfaces PSI, el incremento real del contador
bajo carga CPU acotada y el escritorio/Brave con persistencia.

El analizador interpreta 27 intervalos nuevos del generico y 25 del PC con
PSI de CPU/memoria/E/S medido; CPU full permanece sin valor por su semantica.
Son capturas de un invitado de un vCPU y 2 GiB, con renderizado software y
cargas de prueba. No se usan sus tasas para ajustar el SSD fisico ni culpar
a una aplicacion particular. Cada imagen pasa nueve controles negativos
de la validacion ELF/HEX.

El lector corregido pasa los cuatro perfiles y la salida diferida fallida
devuelve 1; la referencia 9dd71f3 devolvia 0 ante el mismo /dev/full.
La imagen dual descartable anterior, generico #51/PC #37, se conserva bajo
`build/respaldo-antes-psi-9dd71f3/` con hash comprobado, fuera de Git.
Los informes se respaldan en Windows y datos.ext4 conserva su hash anterior.
No se modifican el archivo de juegos de C:, las sesiones privadas ni el EFI
de Windows. La sesion fisica representativa y los tiempos por fotograma
siguen pendientes; esta ronda completa la instrumentacion y su interpretacion.
