# Produccion: C17 con GCC

`ultra_guard.c/h` contienen la API real. `perfil-gcc.sh` es la fuente unica
de banderas para codigo de usuario: avisos como errores, revision con
analizador en O0 y avisos en O2, endurecimiento y trampas de UB en produccion.
Al cargar el perfil se descarta el estado ASan/TSan/UBSan y los arrays de
diagnostico/crudo de una sesion anterior.

[ultra_buffer.h](ultra_buffer.h) ofrece limites comprobados de buffers por filas
y pixeles de cuatro bytes, con estado de error obligatorio. La
[investigacion grafica](../../../docs/WAYLAND-INVESTIGACION-C.md) explica
el contrato, las pruebas y lo que falta para integrarlo con Wayland.

La entrada estable [scripts/guardianes.sh](../../../scripts/guardianes.sh)
la siguen usando init y caja. Los perfiles no se inyectan en Kbuild, GCC
instalado, PATH ni configuracion global de la maquina.

Para reutilizar el perfil en otro programa C de usuario dentro del taller Linux:

```bash
bash /ruta/al/proyecto/src/guardian/produccion/compilar.sh /ruta/de/salida/app /ruta/fuente/main.c
```

El compilador revisa los fuentes, construye static-pie y publica la salida
solo si GCC termina correctamente. No ejecuta el programa, no sobrescribe
una salida existente y no incorpora automaticamente la API de guardianes.
Un programa que quiera ULTRA_ADD u otras APIs debe incluir el header y enlazar
ultra_guard.c. Los proyectos con bibliotecas, opciones de enlace o build propio
deben integrar el perfil y validar sus controles; esta entrada minima no
pretende sustituir todos los sistemas de compilacion.

El perfil actual corresponde al taller GNU/Linux x86-64 con GCC 16 probado.
No garantiza portabilidad a cualquier GCC/CPU/libc ni cubre todo UB de C.
`-fhardened` y los sanitizers tienen alcances especificos:
[documentacion GCC](https://gcc.gnu.org/onlinedocs/gcc/Instrumentation-Options.html).
El kernel tiene reglas distintas de compilacion:
[Kbuild](https://docs.kernel.org/kbuild/makefiles.html).

Promocion desde laboratorio: regla concreta, fixture invalido, control valido,
pruebas en los perfiles relevantes y evidencia revisada. Las banderas o
concesiones de syscalls no se aprenden ni cambian automaticamente.
