# Sonido, suspension, semaforo y disco Personal (2026-11-02)

## ULTRA Sonido (Super+A o Super+Ctrl+A)
Panel `eww` con el estilo BMO-X. Tiene volumen, encendido/apagado de los efectos, 8 perfiles y 5 bandas:
graves 90 Hz, cuerpo 250 Hz, medios 1 kHz, presencia 3,5 kHz y agudos 9 kHz. El sonido 3D tiene dos controles:
- **Amplitud**: matriz medio/lado que ensancha el escenario.
- **Auriculares 3D**: cruce de 0,3 ms con paso bajo a 700 Hz, para que el sonido quede delante y no dentro de la cabeza.

**Fuerte** añade 9 dB de ganancia antes de un limitador estéreo de SWH.
El control **Intensidad** ajusta esa ganancia de 0 a 12 dB. El límite de
−1 dBFS se aplica a la salida del filtro, antes del volumen del dispositivo.

- **Ubicación:** `ultra-sonido preparar` lo escribe con los valores guardados antes de que arranque PipeWire. Así empieza con tus ajustes.
- **Cambios en vivo:** se aplican con `pw-cli set-param ... Props` y se releen de PipeWire. El panel distingue ajustes pendientes de audio que ya pasa por el filtro.
- **Filtro inteligente de WirePlumber 0.5** (`filter.smart`): se coloca delante del dispositivo por defecto, sea USB, HDMI o auriculares, sin cambiar cuál es ese dispositivo.
- **Apagado:** la matriz queda en (1, 0, 0), las bandas y la intensidad en 0 dB.
  El limitador, si está disponible, conserva su retardo de unos 5 ms.

## Suspensión con NVIDIA

`ultra-sesion-control` elige `deep` (S3) cuando el firmware lo ofrece y
solicita `mem` al kernel. NVIDIA Open 615 conserva la VRAM mediante sus
notificadores de suspensión y usa memoria temporal del kernel. En el modo
manual, el supervisor usa `/proc/driver/nvidia/suspend` y solicita la
reanudación aun si falla la entrada en suspensión.

**Opciones de módulo.** El init cargaba los módulos con `finit_module` y parámetros vacíos, así que `nvidia-drm.modeset=1` de `CONFIG_CMDLINE` nunca llegaba al driver. `modopciones.h` reúne las opciones como lo hace `modprobe`:
- líneas `options` de `/sistema/usr/lib/modprobe.d` y luego de `/sistema/etc/modprobe.d`;
- los `modulo.param=` de `/proc/cmdline`.

El lector admite directivas `options`. Sus pruebas (`scripts/probar_modopciones.c`)
usan ASan/UBSan y los flags de `guardianes.sh`. `etc/modprobe.d/ultra-nvidia.conf`
activa la conservación de VRAM y los notificadores del kernel.

En OBS Studio, añade **Captura de pantalla (PipeWire)** y selecciona qué
pantalla compartir en el portal de Hyprland. La receta de la imagen incluye
OBS y ese portal.

## Semaforo del informe
`ultra-salud` lee el informe terminado y pone arriba `[SALUD]`: rojo y amarillo primero, cada uno con qué hacer. Revisa:
- sesión y GPU;
- volumen alto y audio en tiempo real;
- velocidad de red;
- juegos y Heronite;
- memoria y temperatura;
- superbloques ext4 y antigüedad del respaldo.

Con el informe del 2026-11-01 marca justo los 4 avisos reales: volumen 300 %, audio sin tiempo real, red a 100 Mb/s y el aviso de Cyberpunk, que ya está corregido.

## Salud de los discos
`ultra-comprobar-datos` lee solo el superbloque (`dumpe2fs -h`) de `/datos`, `/juegos` y `/personal` al entrar al escritorio. Detecta un cierre brusco o errores registrados por el kernel. Un ext4 montado no se repara en caliente: la reparación (`e2fsck`) queda para un paso sin montar.

## Disco Personal (D:)
D: (SSD Kingston de 120 GB) estaba vacio y ULTRA ya no lo necesita, porque Cyberpunk tiene su copia Linux verificada.

**Formateo:** con permiso del usuario, se formateo desde Windows (`Format-Volume`) como **exFAT `PERSONAL`**. Antes se comprobo que solo tenia un `desktop.ini` de 129 bytes.
- **Por que exFAT:** Windows y ULTRA leen y escriben lo mismo, y no hay limite de 4 GB por archivo.
- **Por que no FAT32:** Windows no formatea FAT32 por encima de 32 GB.
- **Por que no se llama BMO:** el init solo admite una FAT32 `BMO`, y una segunda impediria arrancar.

**Montaje:** `ultra-personal montar` (desde `ultra-sesion`) busca un exFAT interno, no USB, con etiqueta `PERSONAL`. Lo monta en `/personal` con `uid=1000`, `dmask=0077`, `fmask=0177` y `noexec`. Al cerrar la sesion lo desmonta.
- **Respaldos:** la copia diaria va a `/personal/ultra/respaldos`, en otro disco
  físico. Se conserva una copia. Las anteriores se retiran tras crear la nueva.
- **Bibliotecas:** `ultra-bibliotecas` excluye la etiqueta, para no ponerlo en solo lectura.

**Kernel:** no traia exFAT. Se anadieron `EXFAT_FS` (obligatorio) y `USB_UAS` a los perfiles generico y PC.

Pruebas: `probar_sonido` (7), `probar_salud_personal` (9), `probar_sesion_control` (+4 de suspensión) y `probar_modopciones.c`. Falta validarlo en QEMU y en el PC, empezando por compilar el init con GCC 16 y `-fanalyzer`.
