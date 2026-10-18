# Linux Reconstruido

Laboratorio personal x86-64 para modificar el Linux oficial con C y GCC. Inspirado en el control de Arch. Perfil físico inicial: Ryzen 5 5600X y MSI A320M-A PRO MAX.

## Ruta elegida: QEMU y después metal

El taller es una máquina Arch Linux dentro de QEMU. Las fuentes, GCC y la compilación viven en su disco Linux. Windows solo ejecuta QEMU y los lanzadores; WSL no forma parte de esta ruta. WSL se instaló durante la preparación anterior, pero aquí no se utiliza y no se desinstala automáticamente.

La imagen Arch aporta el taller de trabajo; el kernel que reconstruimos es el original de Linus, conservado por separado. No confundir el kernel de la imagen Arch con el kernel experimental. El sistema final y sus decisiones de paquetes todavía están pendientes.

## Componentes

- `upstream/linux.git`: repositorio oficial de Linus, bare y superficial; no incluye todo el historial.
- `evidencias/origen-inicial.json`: commit original fijado, `32c52ec87d6245ee2d12ed5dd509e40062939b50`, Linux 7.3-rc7. Es una candidata, no una publicación estable.
- `evidencias/imagen-arch.json`: origen y SHA-256 de la imagen oficial del taller; no se afirma verificación de firma.
- `vm/`: imagen base, disco de trabajo de 64 GiB virtuales, configuración inicial y clave local. Todo excluido de Git.
- `scripts/taller_qemu.py`: prepara el disco y cloud-init, inicia QEMU y comprueba GCC/fuentes por SSH.
- `docs/PLAN.md`: etapas y criterios de avance.

La VM reserva 4 GiB de RAM y 4 CPU virtuales usando TCG con reloj por instrucciones (`icount`) y un hilo de emulación. Los primeros perfiles Q35/PC sin ese reloj fallaron al comprobar el temporizador; sus registros están en evidencias. No necesita WSL ni reiniciar Windows para usar ese emulador. Compilar un kernel completo con emulación puede ser lento. No se conectan discos físicos a la VM.

## Uso

```powershell
.\preparar-entorno.ps1 -Accion Preparar
.\preparar-entorno.ps1 -Accion Iniciar
.\preparar-entorno.ps1 -Accion Estado
.\conectar-taller.ps1
```

Preparar requiere la imagen y las herramientas locales ya descargadas. Iniciar rechaza puertos ocupados y discos ausentes. El primer arranque instala paquetes oficiales Arch y copia el repositorio original al disco Linux. Estado solo declara listo tras comprobar la marca de finalización, GCC, herramientas, cloud-init y commit del kernel. Revisar `vm/serial.log`, `vm/qemu.log` y `vm/http.log` ante fallos.

SSH se publica únicamente en `127.0.0.1:12222`, autenticado por la clave generada en `vm/`. El servidor local `127.0.0.1:18765` entrega solo la copia de las fuentes. QMP usa `127.0.0.1:14444`. El usuario `builder` tiene sudo en esta VM de laboratorio y no usa contraseña SSH. No reutilizar esta imagen/configuración como servidor público.

Dentro de la VM, las fuentes están en `/home/builder/linux-reconstruido/kernel`. Para apagarla limpiamente: `sudo poweroff`. El servidor local de fuentes puede seguir en marcha hasta cerrarlo mediante el PID registrado en `vm/session.json`; no se deben terminar procesos por nombre global.

## Ecosistema y guardianes

Objetivo: acceso amplio al ecosistema Linux, incluyendo aplicaciones gráficas, compiladores, herramientas, servicios, redes, sonido y juegos compatibles. Chromium será una prueba entre varias. No existe un paquete que incluya todo el ecosistema ni una promesa automática de compatibilidad universal.

Primero se preservan la ABI Linux x86-64 y las dependencias necesarias; después se miden cambios. POSIX se documentará por interfaces y pruebas. No se eliminarán componentes por antigüedad sin identificar sus usos.

ROJO: falla una condición obligatoria y se detiene ese avance. AMARILLO: comportamiento no medido o dependencia pendiente. VERDE: comprobación concreta aprobada, sin extrapolarla al sistema completo. Cada guardián debe explicar motivo, evidencia, acción y coste.

## Próximos hitos

1. Herramientas y fuentes verificadas dentro de QEMU.
2. Kernel oficial sin cambios compilado con GCC, configuración e informe reproducibles.
3. Arranque de ese kernel con initramfs y consola en otra prueba QEMU.
4. Entorno de usuario y pruebas de compatibilidad, gráficos, USB, red y sonido.
5. Arranque en el PC real conservando recuperación y las entradas actuales.

Todavía no se afirma que el kernel experimental haya compilado o arrancado. Los registros distinguen instalación de herramientas, compilación y ejecución.

## Licencias y procedencia

El kernel conserva `COPYING`, `LICENSES/` y los avisos por archivo. Los archivos originales del laboratorio se ofrecen bajo GPL-2.0-only; los componentes de terceros mantienen sus licencias. La licencia no se determina por usar C o GCC.

Fuentes oficiales:
- https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git
- https://www.kernel.org/doc/html/latest/process/license-rules.html
- https://wiki.archlinux.org/title/Arch_Linux_on_a_VPS
- https://github.com/archlinux/arch-boxes