# Probar ULTRA LinuX en el PC real

PC: MSI A320M-A PRO MAX, Ryzen 5 5600X y RTX 3060. **Windows no se toca**: no se escribe ninguna tabla de particiones ni la partición EFI de Windows.

Estado: los dos caminos están probados en QEMU con firmware UEFI (`evidencias/arranque-bmo-archivos.log` y `evidencias/arranque-disco-persistente.log`).

**Primer arranque en el metal (18-10-2026), desde `A:\` con F11:** el sistema funcionó. Detectó 50 componentes, cargó `r8169` (red Realtek 10ec:8168), `k10temp` (sensor del Ryzen) y `snd_hda_intel` (sonido de la placa 1022:1487 y HDMI de la RTX 3060 10de:228e), y guardó todo en `A:\ULTRA\`. La detección tardó **184 ms** y la carga 21,6 ms (`evidencias/pc-real/`). En QEMU la detección era de 0,7 ms: en el hardware real sí hay tiempo que ganar con el arranque guiado.

**Pero la pantalla se quedó negra.** El kernel tenía `simpledrm` sin consola de framebuffer (`DRM_FBDEV_EMULATION` y `FRAMEBUFFER_CONSOLE` apagadas), así que nadie dibujaba texto; y los mensajes del init solo iban al puerto serie. Las pruebas no lo veían porque leían solo el puerto serie. Corregido:
- las dos opciones son ahora obligatorias en el perfil `generico`;
- el init copia sus mensajes a `tty1`, la pantalla;
- la prueba captura el monitor emulado, escribe `echo 4242` con el teclado emulado y exige que la pantalla cambie. Con el kernel anterior da ROJO (0 píxeles cambiados), igual que en el PC; con el corregido, VERDE (`evidencias/pantalla-bmo-tty1.png`).

## Dónde vive: archivos dentro de `A:\` (BMO)

No hay USB libre, así que ULTRA vive como **archivos** en la partición FAT32 `A:\` (etiqueta `BMO`) del SSD Kingston SATA de 480 GB:

```
A:\EFI\ULTRA\ULTRA.EFI                 kernel generico  -> F11 "ULTRA LinuX"
A:\EFI\ULTRA\ULTRA-PC.EFI              kernel Zen 3     -> F11 "ULTRA LinuX PC (Zen 3)"
A:\ULTRA\sistema-<versión>.erofs       el sistema de cada kernel, de solo lectura ("-pc-" = Zen 3)
A:\ULTRA\datos.ext4                    /datos, compartido: PC.configuración y arranques.log
A:\ULTRA\LEEME.txt                     qué es cada cosa, para leerlo desde Windows
A:\ULTRA\informe\                      lo que deja ULTRA en cada arranque para Windows:
    PC-configuracion.txt                 componentes y módulos de este PC
    arranques.txt                        una línea por arranque: kernel y tiempos
    dmesg-<versión>.txt                  registro del kernel, con marcas de tiempo
```

**Por qué no se escribe el disco entero:** ese mismo SSD tiene, antes de `A:`, la partición EFI de 600 MB desde la que **arranca Windows** (Windows Boot Manager); el NVMe de `C:` no tiene una propia. Una imagen escrita en modo DD sobre ese disco la borraría. Con archivos dentro de `A:` no se toca ni esa partición ni `F:`.

El init encuentra `A:` leyendo la etiqueta `BMO` del directorio raíz del FAT32, sin montar nada más. Windows deja `NO NAME` en el sector de arranque, y por eso se lee del directorio raíz. Después monta `A:` en `/bmo` (sin permiso de ejecución) y los dos archivos como discos *loop*. Al apagar, desmonta `/datos`, `/sistema` y `/bmo` en ese orden, para que Windows encuentre el FAT32 limpio.

Ya comprobado en este PC (18-10-2026): **Secure Boot desactivado** y **BitLocker sin protección**. No hay riesgo de que Windows pida la clave de recuperación.

## 1. Instalar o actualizar los archivos

Tras cada `Compilar` y `Descargar` del perfil `generico`:

```powershell
.\instalar-en-A.ps1 -Accion Copiar
```

El script se niega si `A:` no es FAT32 con la etiqueta `BMO`, o si es una partición de sistema o de arranque. Comprueba el SHA-256 de cada archivo antes y después de copiarlo. `datos.ext4` se copia solo la primera vez, para no borrar lo guardado; `-NuevosDatos` empieza de cero.

## 2. La entrada del menú F11 (una sola vez)

```powershell
.\instalar-en-A.ps1 -Accion CrearEntrada
```

Crea una entrada de arranque del firmware llamada **ULTRA LinuX** que apunta a `A:\EFI\ULTRA\ULTRA.EFI`. La pone **al final** del orden, así que Windows sigue arrancando por defecto. Se quita con `-Accion QuitarEntrada`, y `-Accion Estado` muestra qué hay instalado.

## 3. Arrancar

Reinicia y pulsa **F11** hasta que salga el menú de arranque. Elige **ULTRA LinuX**.

Si la entrada no aparece: en la BIOS (Supr), `Settings > Advanced > Windows OS Configuration`, `Windows 10/11 WHQL Support` debe estar en **Enabled** (UEFI sin CSM). Así la RTX 3060 también ofrece su pantalla UEFI, que es la que usa ULTRA LinuX.

Qué deberías ver:

```
ULTRA LinuX: INIT C READY
VERDE [KERNEL]: Linux 7.3.0-rc7-... x86_64
AMARILLO [PARTICION]: discos buscados durante 800 ms        ← normal: SATA tarda en aparecer
VERDE [PARTICION]: BMO (/dev/sda2, vfat) en /bmo
VERDE [PARTICION]: ULTRA-SISTEMA-A <- /bmo/ULTRA/sistema-a.erofs (/dev/loop0, erofs, solo lectura) en /sistema
VERDE [PARTICION]: ULTRA-DATOS <- /bmo/ULTRA/datos.ext4 (/dev/loop1, ext4) en /datos
VERDE [HARDWARE]: primer arranque en este PC (...): deteccion completa
VERDE [HARDWARE]: N componentes, M modulos necesarios (...); deteccion ... us + carga ... us
VERDE [HARDWARE]: guardado en /datos/PC.configuración
ultra:~#
```

La letra (`sda`, `sdb`…) depende del orden de los discos; da igual, porque se busca por etiqueta.

Antes de la shell aparece la **tarjeta de arranque** (`src/tarjeta.c`, captura en `evidencias/pantalla-tarjeta-tty1.png`). Todo se lee en ese momento, nada se supone:
- **Equipo:** el modelo de CPU, su temperatura (del sensor `k10temp` que cargó ese mismo arranque) y la RAM.
- **Sistema:** el kernel y cuántos módulos necesitó este PC.
- **Tubería:** el tiempo hasta el fin de la etapa de hardware como una barra (cuanto más corta, más ancha la tubería), con `▼`/`▲` frente al arranque anterior del mismo kernel.
- **Guardianes:** CET, Landlock y seccomp, preguntados al kernel; FORTIFY, de la compilación. Verde si está activo, rojo si no.

En el puerto serie va lo mismo como una sola línea `[TARJETA]`, y la prueba exige ver los cuatro guardianes activos. Los módulos de la lista se cargan en paralelo, uno por proceso.

## 4. Qué comprobar

```sh
cat /datos/PC.config*                  # componentes y modulos detectados
lsmod                                  # modulos cargados
grep "model name" /proc/cpuinfo | head -1
cat /sys/class/hwmon/hwmon*/name       # k10temp = sensor del Ryzen
cat /sys/class/hwmon/hwmon*/temp1_input
ip link                                # eth0 si cargo r8169 (la red de la placa)
minas --cet                            # pila sombra CET: el 5600X (Zen 3) la tiene
grep x86_Thread /proc/1/status
```

Lo esperable en `PC.configuración`:
- `r8169` para la red Realtek de la placa;
- `k10temp` para el sensor de temperatura del Ryzen;
- `snd_hda_intel` para el sonido de la placa y para el audio HDMI de la RTX 3060.

La RTX 3060 aparecerá como `NVIDIA 10de:2504 pantalla` (o similar) con `driver=-` y `modulos=-`: aún no hay driver NVIDIA, la pantalla es la del firmware (`simpledrm`).

## 5. La memoria entre arranques

```sh
reboot
```

F11 otra vez y **ULTRA LinuX**. Ahora debe salir:

```
VERDE [HARDWARE]: arranque guiado por PC.configuracion: M modulos (...); lectura ... us + carga ... us = ... us; primer arranque ... us; ganado ... us
```

Haz varios reinicios para tener varias medidas. Para forzar otra detección: `rm /datos/PC.config*` y reiniciar.

## 5b. Comparar `generico` y `pc` (Zen 3)

Los dos kernels conviven en `A:\`: `ULTRA.EFI` y `ULTRA-PC.EFI`, cada uno con su propio `sistema-<versión>.erofs` y el mismo `datos.ext4`. Se instalan con `.\instalar-en-A.ps1 -Accion Copiar -Perfil pc` (y `-Perfil generico`), y `-Accion CrearEntrada -Perfil pc` añade al menú F11 la entrada **ULTRA LinuX PC (Zen 3)**.

Arranca cada uno 4 o 5 veces, alternando, y apaga siempre con `poweroff`. Cada línea de `A:\ULTRA\informe\arranques.txt` dice qué kernel arrancó (`kernel=…-pc-…` o sin `-pc`) y cuánto tardó:
- `uptime_ms`: desde que arranca el kernel hasta que termina la etapa de hardware;
- `total_us`: la etapa de hardware del init.

**Resultado en el metal (18-10-2026, `evidencias/pc-real/arranques-generico-vs-pc.txt`):**

| | `generico` (4 arranques) | `pc` Zen 3 (5 arranques) |
|---|---|---|
| Carga de los 3 módulos (mediana) | 19,3 ms | **17,4 ms** (−10 %, sin solaparse) |
| Etapa de hardware guiada | 20,2 ms | **18,4 ms** |
| Primer arranque (detección completa) | 205,7 ms | — |
| Del kernel al fin de esa etapa (`uptime`) | 2000–2020 ms | 2000–2020 ms |

- El arranque guiado gana **unos 187 ms** frente a la detección completa: en el hardware real, `PC.configuración` sí vale la pena.
- `pc` carga los módulos un 10 % más rápido (trabajo de CPU dentro del kernel), pero el arranque total empata.
- **El cuello de botella son los 2 segundos del kernel**, no la CPU.

### Dónde se iban los 2 segundos (`evidencias/pc-real/dmesg-*.txt`)

El init no empezaba hasta el segundo 2,27 (`Run /init`), porque el kernel espera a que terminen todos los discos SATA, y la placa activa el arranque escalonado (`ahci: SSS flag set, parallel bus scan disabled`). Así revisaba los puertos **uno por uno**:

| Tiempo | Puerto | Coste |
|---|---|---|
| 0,92 s | `ata1`, vacío | ~0,3 s |
| 1,24 s | `ata2`, vacío | ~0,3 s |
| 1,70 s | `ata5`, SSD de 480 GB (`A:`) | ~0,46 s |
| 2,20 s | `ata6`, SSD de 120 GB | ~0,5 s |

Unos 1,6 s de 2,0 s. El arranque escalonado protege a los discos mecánicos de un pico de corriente; este PC solo tiene SSD. Arreglo: `libahci.ignore_sss=1` en la línea integrada del kernel, para revisar los puertos en paralelo.

El primer intento usó `ahci.ignore_sss=1` y en el PC **no hizo nada**. El parámetro está en `libahci.c`, así que su nombre lleva el prefijo `libahci.`. El kernel no avisa de un parámetro con punto que no reconoce, y la prueba solo comprobaba el texto de `/proc/cmdline`. Ahora la prueba lee el valor que el kernel aplicó (`/sys/module/libahci/parameters/ignore_sss`). Con el nombre equivocado ese valor da 0 y la prueba falla.

El mismo `dmesg` mostró un bug: la entrada F11 (copia de Windows Boot Manager) pasaba sus datos opcionales al kernel, que los añadía a su línea como `䥗䑎坏S` ("WINDOWS" leído como UTF-16). Arreglo: `CONFIG_CMDLINE_OVERRIDE`, para que solo valga la línea integrada. La prueba comprueba que no llega nada detrás.

Queda otro salto de 0,3 s al iniciar el IOMMU de AMD (`AMD-Vi`). No se toca: protege la memoria frente a los dispositivos.

## 6. Ver los resultados en Windows

Apaga con `poweroff` y arranca Windows. En `A:\ULTRA\informe\` están `PC-configuracion.txt`, `arranques.txt` y un `dmesg-<versión>.txt` por kernel, actualizados en cada arranque de ULTRA. No hace falta escribir nada a mano.

## Si algo falla

| Síntoma | Qué significa | Qué probar |
|---|---|---|
| "ULTRA LinuX" no sale en F11 | El firmware no tomó la entrada, o CSM activo | `.\instalar-en-A.ps1 -Accion Estado`; paso 3 (WHQL en Enabled) |
| "Secure Boot violation" | Secure Boot se volvió a activar | Desactivarlo en `Windows OS Configuration > Secure Boot` |
| Pantalla negra tras elegirla | El firmware no dio framebuffer | Otra salida de la RTX (DisplayPort/HDMI); comprobar CSM apagado |
| El PC se reinicia solo en bucle | Pánico del kernel (`panic=-1` reinicia al instante) | Elegir Windows en F11; anotar lo último visible |
| `ni particiones ... ni FAT32 BMO` | El SSD tardó más de 10 s o `A:` perdió la etiqueta | Comprobar en Windows que `A:` sigue llamándose `BMO` |
| `sistema-…erofs: No such file` | Faltan los archivos de ese kernel | `.\instalar-en-A.ps1 -Accion Copiar -Perfil generico` (o `-Perfil pc`) |
| Windows pide revisar `A:` | ULTRA se apagó sin desmontar (corte de luz, botón) | Dejar que lo revise; `datos.ext4` puede necesitar empezar de cero con `-NuevosDatos` |

Para volver a Windows basta con no elegir ULTRA en F11: Windows sigue siendo el arranque por defecto.

## Límites conocidos

- **Peligro dentro de la shell**: eres root y el kernel ve el disco de Windows (`/dev/nvme0n1`), la partición EFI de Windows y `F:`. El init no monta nada de eso, pero un `dd` o `mkfs` escrito a mano sí podría dañarlo. No ejecutes nada sobre `/dev/nvme*` ni `/dev/sd*`.
- **`/datos` sobre FAT32**: el ext4 tiene diario, pero el FAT32 que lo contiene no. Un corte de luz con ULTRA encendido puede dejar `A:` para revisar.
- **El arranque guiado no gana tiempo en QEMU**: con `A:` la lectura pasa por FAT32 y loop, y salió 1,9 ms más lento que la detección completa. En el PC real la detección costó 184 ms, así que ahí sí se espera ganancia; se mide en cada arranque (`arranques.txt`).
- **Sin sonido todavía**: se carga el controlador `snd_hda_intel`, pero el códec Realtek de la placa no está compilado y el bus `hdaudio` aún no se recorre.
- **Sin driver NVIDIA**: solo la pantalla básica del firmware; `nvidia-open` llega en una fase posterior.
- **Red sin configurar**: `r8169` crea `eth0`, pero no hay DHCP.

## Alternativa: un USB completo

Si un día hay un USB que se pueda borrar, `build\generico\ultra-esp.img` se escribe entero con Rufus en modo DD. Lleva su propia partición EFI, `ULTRA-SISTEMA-A` y `ULTRA-DATOS`, y se elige con F11 como `UEFI: <USB>`. **Nunca en el disco de `A:`**: ese disco tiene la partición EFI de Windows.
