# Guardian: no continuar con una instalacion incompleta

2026-10-22. Corregido en fuentes y comprobado en QEMU del taller. Los binarios
actualmente instalados en A: no fueron reconstruidos en esta ronda.

`ultra_guard_install` antes descartaba los resultados de sigaltstack y de los
seis sigaction. El llamador podia continuar aunque faltara la pila alternativa
o alguno de los manejadores. Se comprueban ahora esas operaciones y sigemptyset.
Un fallo imprime paso y errno y termina el proceso con **exit 78**, mediante
_exit, sin depender del manejador cuya instalacion acaba de fallar.

No se intenta restaurar handlers parciales y seguir: la politica es terminar
ese proceso. En un PID 1 real esta politica puede detener el sistema; exige
validar el arranque antes de desplegar. Los casos de fallo se ejecutaron solo
en namespaces desechables dentro de QEMU, no contra el init del taller o del PC.

## Pruebas y limites

GCC 16.2.1 20260810; Linux 7.2.11-arch1-1 del taller, x86-64. Cuatro perfiles:
produccion static-pie O2/O3 y ASan/UBSan O1/O2. Diez escenarios por perfil:

1. Instalacion normal: se consultan la pila y los seis handlers al kernel,
   comprobando los flags requeridos y el numero de instalaciones.
2. Fallo de sigaltstack con EPERM.
3. Fallo de sigemptyset con EINVAL.
4. Fallo EPERM en cada una de las seis posiciones de sigaction: seis escenarios.
5. Instalacion normal y trampa intencional: bandera y salida 132 esperadas.

Los ocho fallos inyectados deben salir con 78, identificar operacion/errno y no
alcanzar el marcador posterior a la instalacion. La inyeccion usa wrappers de
linker solo en el ejecutable de pruebas; no hay ganchos de inyeccion en produccion.
Simula devoluciones de error: no demuestra un rechazo real de seccomp a esas
syscalls. El control normal si usa las operaciones y consultas reales del kernel.

La trampa prueba el camino del manejador; no es una demostracion de UB en el
programa. Las etiquetas historicas de SIGILL no identifican por si solas su causa
exacta. No se acredita manejo de todas las senales, recuperacion de pila agotada
o instalacion de una pila alternativa para cada hilo de cualquier aplicacion.

Tras este cambio se ejecuto la auditoria completa: 36 rechazos de compilacion,
62 casos de ejecucion con 53 VERDE/1 NEUTRA/8 AMARILLO/0 ROJO, cruces sin ROJO,
dominios ctype, buffers y nuevas cadenas. La carrera de hilos conserva su
limitacion AMARILLO; no se convierte en deteccion acreditada por pasar el script.

Evidencia y controles individuales: [ronda 10](../evidencias/laboratorio-c/ronda10-2a55e6d3d52a4a81a3bb8e9e2c493fc7/README.md).
El siguiente requisito de despliegue es reconstruir y probar init/caja y su
arranque real en una imagen desechable antes de reemplazar la version de A:.
