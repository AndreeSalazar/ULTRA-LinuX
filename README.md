# Linux Reconstruido

Laboratorio personal x86-64 para entender y modificar Linux, inspirado en la sencillez y el control de Arch. Primer perfil: Ryzen 5 5600X y MSI A320M-A PRO MAX. Objetivo: acceso amplio al ecosistema Linux x86-64, incluyendo aplicaciones gráficas, herramientas de desarrollo, servicios, redes y juegos compatibles. Chromium será una prueba entre varias; no define todo el alcance.

## Qué hay aquí

- `upstream/linux.git`: repositorio oficial de Linus obtenido de kernel.org, como espejo bare superficial del estado inicial. No incluye todo el historial.
- `upstream/origen.json`: URL y commit exacto descargado, una vez completada la descarga.
- `docs/PLAN.md`: etapas, contratos y criterios de avance.
- `scripts/preparar-linux.sh`: despliega una copia de trabajo en un sistema de archivos Linux, conservando el origen.
- `evidencias/`: resultados medidos de futuras compilaciones y arranques.

El kernel no constituye una distribución completa. Todavía no se ha compilado ni arrancado este proyecto. La instalación del entorno WSL se registra en `evidencias/wsl-instalacion.json`; no se considera operativo hasta verificar Debian y GCC. No se han cambiado discos, entradas UEFI ni el proyecto TALK-CPU.

## Punto de partida fijado

Linux **7.3-rc7**, commit `32c52ec87d6245ee2d12ed5dd509e40062939b50` del repositorio oficial. Es una versión candidata de mainline, no una publicación estable. La conectividad de los objetos se comprobó con Git; no se verificó una firma de publicación. El registro rastreado está en `evidencias/origen-inicial.json`.

## Entorno de trabajo

Compilar con GCC en un entorno Linux. Conservar la copia de trabajo en su sistema de archivos Linux, por ejemplo en `~/linux-reconstruido/kernel`, para respetar nombres sensibles a mayúsculas y semántica de archivos. El repositorio bare puede permanecer en esta carpeta de Windows.

Tras disponer de un entorno Linux con Git, desde esta carpeta accesible en él:

```sh
bash scripts/preparar-linux.sh "$HOME/linux-reconstruido/kernel"
```

El script rechaza destinos existentes. Después deben instalarse las dependencias indicadas por `Documentation/process/changes.rst` de la versión descargada. La primera compilación será una base sin modificaciones, seguida de una prueba en QEMU. Este README no afirma que Chromium funcione todavía.

## Cambios y guardianes

Primero establecer una referencia reproducible; después modificar una cosa por vez. Evitar eliminar interfaces o controladores por antigüedad: comprobar primero su uso, dependencias y efecto medido. Compatibilidad objetivo inicial: programas x86-64 de Linux; el alcance POSIX se documentará por interfaces y pruebas, sin anunciar conformidad completa.

ROJO: detener el avance si falla compilación, arranque de referencia o una prueba obligatoria. AMARILLO: comportamiento no medido, degradación o dependencia pendiente. VERDE: comprobación concreta aprobada; no significa seguridad universal. Cada resultado debe incluir motivo, evidencia y siguiente acción.

## Procedencia y licencias

El código del kernel conserva `COPYING`, `LICENSES/`, avisos y licencias por archivo. Su distribución global es GPL versión 2 solamente, con la nota de llamadas al sistema descrita por upstream. Los archivos originales de este laboratorio se ofrecen bajo GPL-2.0-only; no se relicencian los componentes de terceros. Chromium y el resto del entorno conservarán sus propias licencias.

Fuentes: https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git y https://www.kernel.org/doc/html/latest/process/license-rules.html.
## Preparar las herramientas

Desde PowerShell, después de cualquier reinicio requerido por la instalación de WSL:

```powershell
.\preparar-entorno.ps1
```

Debian WSL sirve como taller de compilación; no decide la distribución final. El preparador instala GCC, binutils, make, dependencias del kernel, depurador, QEMU y utilidades para initramfs. Conserva fuentes y compilación en `/root/linux-reconstruido` dentro de Linux; registros y configuración quedan en esta carpeta. No habilita acceso remoto ni crea contraseñas.

El objetivo es compatibilidad amplia con el ecosistema, no prometer todos los programas de todas las distribuciones. Se comprobarán por separado ABI Linux x86-64, bibliotecas, carga dinámica, procesos, memoria, red, gráficos, sonido y aislamiento. Los paquetes de distintas distribuciones no son intercambiables automáticamente. No se eliminarán mecanismos de compatibilidad sin identificar qué aplicaciones dependen de ellos.
## Estado real de instalación

WSL 3.0.3.0 instalado desde el MSI oficial, con SHA-256 y firma Authenticode de Microsoft verificados. Windows solicita reiniciar para activar VirtualMachinePlatform. El intento de instalar Debian terminó indicando ese reinicio; no se afirma que Debian esté registrado o que GCC esté instalado aún. No se reinició automáticamente.

Después del reinicio manual, ejecutar `preparar-entorno.ps1` para instalar/completar Debian y las herramientas. El kernel de ejecución que incorpora WSL pertenece al taller: es distinto del kernel oficial fijado en `upstream/linux.git` que vamos a compilar y probar por separado.

Referencia de instalación: https://learn.microsoft.com/windows/wsl/install.