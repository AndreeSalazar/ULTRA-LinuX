# Sonido, suspension, semaforo y disco Personal (2026-11-02)

## ULTRA Sonido (Super+A)
Panel `eww` con el estilo BMO-X. Tiene volumen, encendido/apagado de las mejoras, 7 perfiles y 5 bandas:
graves 90 Hz, cuerpo 250 Hz, medios 1 kHz, presencia 3,5 kHz y agudos 9 kHz. El sonido 3D tiene dos controles:
- **Amplitud**: matriz medio/lado que ensancha el escenario.
- **Auriculares 3D**: cruce de 0,3 ms con paso bajo a 700 Hz, para que el sonido quede delante y no dentro de la cabeza.

**Motor.** Es un `filter-chain` de PipeWire 1.6, solo con filtros internos (biquads, mezclador y retardo), sin plugins externos.
- **Ubicación:** `ultra-sonido preparar` lo escribe con los valores guardados antes de que arranque PipeWire. Así empieza con tus ajustes.
- **Cambios en vivo:** se aplican con `pw-cli set-param ... Props`.
- **Filtro inteligente de WirePlumber 0.5** (`filter.smart`): se coloca delante del dispositivo por defecto, sea USB, HDMI o auriculares, sin cambiar cuál es ese dispositivo.
- **Apagado:** con las mejoras apagadas, la matriz queda en (1, 0, 0) y las bandas en 0 dB. Es el sonido original.
- **Sin saturación:** la ganancia se compensa según el mayor realce, la amplitud y el cruce, y la señal mono nunca supera la unidad.

No se reemplazó PipeWire: es el motor que ya maneja ALSA, Bluetooth y las apps. Todo lo que oyes y controlas pasa ahora por ULTRA Sonido.

## Suspension con NVIDIA
Sin systemd, nadie avisaba al driver antes de dormir. Ahora `ultra-sesion-control`:
1. elige `deep` (S3) si el firmware lo ofrece;
2. escribe `suspend` en `/proc/driver/nvidia/suspend`;
3. duerme;
4. escribe `resume` siempre, aunque falle la suspensión.

Si el driver no conserva la VRAM, se niega a suspender en vez de volver con la pantalla negra.

**Opciones de módulo.** El init cargaba los módulos con `finit_module` y parámetros vacíos, así que `nvidia-drm.modeset=1` de `CONFIG_CMDLINE` nunca llegaba al driver. `modopciones.h` reúne las opciones como lo hace `modprobe`:
- líneas `options` de `/sistema/usr/lib/modprobe.d` y luego de `/sistema/etc/modprobe.d`;
- los `modulo.param=` de `/proc/cmdline`.

Solo usa `options`: nunca `install`. Sus pruebas (`scripts/probar_modopciones.c`) pasan con ASan/UBSan y los flags de `guardianes.sh`. `etc/modprobe.d/ultra-nvidia.conf` activa `NVreg_PreserveVideoMemoryAllocations=1` y guarda la VRAM en `/juegos/.ultra-nvidia`.

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
- **Respaldos:** los diarios van a `/personal/ultra/respaldos` (ultimos 10), en otro disco fisico.
- **Bibliotecas:** `ultra-bibliotecas` excluye la etiqueta, para no ponerlo en solo lectura.

**Kernel:** no traia exFAT. Se anadieron `EXFAT_FS` (obligatorio) y `USB_UAS` a los perfiles generico y PC.

Pruebas: `probar_sonido` (7), `probar_salud_personal` (9), `probar_sesion_control` (+4 de suspensión) y `probar_modopciones.c`. Falta validarlo en QEMU y en el PC, empezando por compilar el init con GCC 16 y `-fanalyzer`.
