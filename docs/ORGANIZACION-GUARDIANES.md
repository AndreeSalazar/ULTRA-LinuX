# Produccion y banco de investigacion C

Actualizacion 2026-10-23: [cuatro responsabilidades de los guardianes](CATEGORIAS-GUARDIANES.md):
Escritura, Caza UB, Fluidez y Custodia. Se aplican sobre las carpetas existentes, con
contratos, evidencias y estados separados; no crean protecciones globales.

Ronda 8, 2026-10-19. La reorganizacion no incorpora automaticamente las
42 lineas pendientes ni promete cobertura de todo C. Crea una base reutilizable
para desarrollarlas y contrastarlas con el mismo GCC y la misma API.

## Dos categorias fisicas

| Carpeta | Contenido | Entrada |
|---|---|---|
| [Produccion](../src/guardian/produccion/README.md) | Implementacion unica ultra_guard, avisos, revision y banderas GCC de produccion | scripts/guardianes.sh; compilar.sh para programas sencillos |
| [Laboratorio](../src/guardian/laboratorio/README.md) | Minas, controles, diagnostico y comparacion cruda | scripts/guardianes-lab.sh y lanzadores de pruebas |

Las rutas antiguas ultra_guard.c/h son pequenas entradas de compatibilidad,
sin logica duplicada. La ruta antigua minas.c incluye su caso de laboratorio:
conserva el diagnostico que la imagen de ULTRA ya incorporaba. Esta ronda
no elimina ese ejecutable de la imagen ni cambia la politica de arranque.
Las auditorias de syscalls siguen bajo src/caja; usan el perfil de produccion.

## GCC explicito y alcance de uso general

El perfil conserva C17, revision con analizador en O0 y avisos en O2,
FORTIFY/endurecimiento y trampas de UB. El laboratorio agrega ASan/UBSan
o prueba TSan por separado. Volver a cargar produccion elimina variables
experimentales y opciones de runtime de sanitizers de esa shell.

Se puede reutilizar la entrada de compilacion en el taller Linux:

```bash
bash /ruta/proyecto/src/guardian/produccion/compilar.sh /ruta/salidas/programa /ruta/fuentes/programa.c
```

Revisa antes de construir; publica static-pie solo tras exito; no ejecuta ni
sobrescribe una salida. Permite varios fuentes C. La API requiere include y
enlace explicitos. Para bibliotecas o sistemas de build complejos hay que
integrar el perfil y verificar su compatibilidad.

No instala flags globales en el sistema ni modifica los flags del kernel.
El banco es general en las familias de C que puede investigar, pero su perfil
actual esta validado en GNU/Linux x86-64 con GCC 16. Una opcion adicional
necesita pruebas: mas banderas no implica mas cobertura ni mejor rendimiento.
Fuentes: [GCC](https://gcc.gnu.org/onlinedocs/gcc/Instrumentation-Options.html),
[Kbuild](https://docs.kernel.org/kbuild/makefiles.html).

## Evitar colisiones entre ejecuciones

Los lanzadores crean un UUID por llamada: transferencia, compilacion y
evidencia tienen directorios distintos. No escriben sobre build/artifacts del
kernel ni sobre informes anteriores. El header del catalogo generado por la
auditoria queda dentro de su ejecucion; ya no se copia sobre el fuente local.
La evidencia solo se recupera despues de que la suite finalice correctamente.

Esto separa archivos, no reserva recursos exclusivos. Todos siguen compartiendo
la VM; las cuotas agregadas por caja permanecen pendientes. Para ejecutar una
suite shell directamente se debe asignar un ULTRA_*_OUT propio; los lanzadores
PowerShell ya lo hacen. Los directorios remotos se conservan para inspeccion.

## Evidencia y limite de integracion

El [control de perfiles](../evidencias/laboratorio-c/ronda8-610cf0daf6a84fa8a0da028bd8ee3f97/perfiles-ronda8.txt)
verifica separacion, vuelta de laboratorio a produccion, compilacion valida,
rechazo por division constante entre cero, proteccion de salida y compilacion
de las 62 minas mediante la entrada de compatibilidad usada por el build.

La [suite C](../evidencias/laboratorio-c/ronda8-610cf0daf6a84fa8a0da028bd8ee3f97/campo-minas-ronda8.txt)
mantiene 53 VERDE, 1 NEUTRA, 8 AMARILLO y cero ROJO, con 36 rechazos de
compilacion. Pasaron tambien cruces de perfiles, hilos y dominios ctype.
La [auditoria de caja](../evidencias/syscalls/ronda8-594d6f628c0c4a09b660891fd01cf8fd/syscalls-ronda8.txt)
mantiene 20 controles del filtro y 17 de la caja; la equivalencia simbolica
sigue pasando bajo su modelo. Se guardan dos ejecuciones C independientes,
en directorios diferentes, realizadas durante esta reorganizacion.

Estas son pruebas en el kernel Arch de la VM. Compilar la entrada del
diagnostico de arranque no demuestra un nuevo arranque completo de ULTRA.
No se incluyen aqui las modificaciones simultaneas de firmware, init y logo.
