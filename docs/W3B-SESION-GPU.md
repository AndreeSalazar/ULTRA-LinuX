# W3b: sesion sin root y modo NVIDIA GLES

W3c amplia el menu con [Hyprland y aplicaciones](HYPRLAND-W3C.md) y corrige
la preparacion de nodos NVIDIA. Este documento conserva el alcance de W3b.

La primera parte de W3b conserva Cage/Foot. Hyprland y los guardianes C++
del laboratorio todavia no estan integrados en estos programas de Arch.

Ambos perfiles se reconstruyeron e instalaron en A: con hashes verificados
y datos conservados. [Evidencia final](../evidencias/w3b/README.md): generico
BMO 66/66, disco 60/60, PC BMO 70/70; seleccion automatica y recuperacion
aprobadas. La prueba GLES NVIDIA en el PC sigue pendiente.

Actualizacion del menu: ambos perfiles se reconstruyeron e instalaron de
nuevo; generico BMO 70/70 y PC BMO 74/74. Ver
[evidencia del menu](../evidencias/menu-escritorio/README.md).

## Comportamiento

En la consola escribe solamente `e`. Abre el menu con flechas arriba/abajo y
Enter. Las teclas 1-6 seleccionan una opcion; Enter la ejecuta. Esc o Q vuelve
a la consola. Opciones: automatico, NVIDIA, recuperacion software, ultimo
diagnostico, estado NVIDIA y salir. Tras cerrar Foot con `exit`, una tecla
devuelve al menu. Los fallos tambien regresan al menu para elegir recuperacion.
El diagnostico muestra las ultimas 100 lineas de cada registro temporal;
puede pertenecer a una sesion anterior si el nuevo intento no llego al cliente.

`ultra-escritorio` sin argumentos abre ese mismo menu si entrada y salida
son terminales. Para scripts y pruebas usar `--auto` de forma explicita;
sin terminal conserva la seleccion automatica. `--menu` requiere consola.
La interfaz llama al mismo preparador; no modifica permisos ni el renderer.

El modo automatico elige NVIDIA si existe un unico dispositivo
DRM con driver nvidia. No supone que su nombre sea card0: recorre sysfs y
selecciona el nodo por su driver. Con mas de una GPU NVIDIA requiere ampliar
la seleccion explicita; no escoge una arbitrariamente.

```sh
e
ultra-escritorio --auto
ultra-escritorio --nvidia
ultra-escritorio --software
ultra-escritorio --software -- foot
```

El modo NVIDIA exige proveedor EGL NVIDIA, prepara sus nodos con
nvidia-modprobe como root y pide el renderer gles2 de wlroots en esa tarjeta.
El nombre gles2 es el selector de wlroots, no una afirmacion sobre la version
maxima de GLES de la RTX. El entorno del cliente comienza limpio y no concede
WLR_RENDERER_ALLOW_SOFTWARE. Un fallo se informa y devuelve la consola:
no cambia silenciosamente a pixman dentro del modo NVIDIA.

Software pide pixman, prefiere simple-framebuffer y queda como recuperacion.
Si el modo software tambien falla en el hardware concreto, conservar el
perfil generico de F11. No se cambian variables de sincronizacion, cursor,
modificadores ni mitigaciones sin un fallo observado que lo justifique.

## Privilegios

ultra-sesion es el preparador root: inicia udevd, prepara el runtime y un
seatd separado. El socket pertenece a ultra. Luego setpriv lanza el cliente
con UID/GID 1000, grupos inicializados (render), capacidades Inh/Prm/Eff/Bnd/Amb
cero y NoNewPrivs=1. ultra-cliente comprueba las credenciales antes de ejecutar
Cage. Foot y su shell heredan esa identidad. La cuenta esta bloqueada para
autenticacion por password; no tiene grupos input/video ni sudo.

seatd y udevd siguen siendo componentes privilegiados de confianza. El grupo
render permite acceder al driver; esto no protege de un fallo del kernel.
Compositor y aplicaciones comparten usuario: no hay aislamiento por aplicacion.
No se afirma que caja/Landlock/seccomp ya confinen a estos binarios dinamicos.
/home/ultra es parte del overlay temporal: esta fase no cambia propiedad de
/datos ni concede a las apps acceso general a los datos del propietario.

Un flock evita dos sesiones ULTRA simultaneas. El preparador cierra el
descriptor del candado antes de iniciar udevd, seatd y el cliente. En el
primer intento faltaba cerrarlo al iniciar udevd: rechazar --nvidia retenia
el bloqueo e impedia la recuperacion software. El diagnostico y la regresion
se conservan en evidencias/w3b; la correccion se reconstruye antes de instalar.

## Prueba en el PC

Arrancar ULTRA LinuX PC desde F11 y ejecutar:

```sh
e
```

Elegir "Escritorio NVIDIA (GPU)" con flecha abajo y Enter. Dentro de Foot,
comprobar identidad y driver:

```sh
id
grep -E 'Uid:|Gid:|CapEff:|CapBnd:|NoNewPrivs:' /proc/self/status
nvidia-smi
```

Cerrar con exit, pulsar una tecla y elegir "Ver ultimo diagnostico" en el
menu. "Ver estado NVIDIA" evita escribir nvidia-smi. La consulta directa
desde la consola sigue disponible:

```sh
ultra-entrar /usr/bin/grep -E 'EGL vendor|GL vendor|GL renderer|CREDENCIALES' /run/ultra-escritorio/ultima.log
```

Debe verse el renderer NVIDIA real, no solo renderer-solicitado=gles2.
Traer ese registro y confirmar teclado, imagen y retorno a consola. Si falla:

```sh
ultra-entrar /usr/bin/cat /run/ultra-escritorio/ultima.log
ultra-entrar /usr/bin/cat /run/ultra-escritorio/seatd.log
ultra-escritorio --software
```

Los registros pertenecen al ultimo intento que llego a lanzar el cliente y
viven en /arch/run, temporal. El kernel y nvidia-smi por si solos no acreditan
que la sesion acelere con GLES. Tampoco se anuncia un porcentaje de mejora.

Fuentes de interfaces: [seatd](https://man.archlinux.org/man/seatd.1.en),
[wlroots, entorno](https://raw.githubusercontent.com/swaywm/wlroots/master/docs/env_vars.md),
[Cage, opciones](https://raw.githubusercontent.com/cage-kiosk/cage/master/cage.1.scd).
Las ramas upstream son referencias mutables; las versiones instaladas y las
pruebas de esta build son la evidencia aplicable al proyecto.
