# Probar ULTRA LinuX en el PC real

PC: MSI A320M-A PRO MAX, Ryzen 5 5600X y RTX 3060. **Windows no se toca**: no se escribe ninguna tabla de particiones ni la partición EFI de Windows.

Estado: los dos caminos están probados en QEMU con firmware UEFI (`evidencias/arranque-bmo-archivos.log` y `evidencias/arranque-disco-persistente.log`). **Todavía no ha arrancado en el metal.**

## Dónde vive: archivos dentro de `A:\` (BMO)

No hay USB libre, así que ULTRA vive como **archivos** en la partición FAT32 `A:\` (etiqueta `BMO`) del SSD Kingston SATA de 480 GB:

```
A:\EFI\ULTRA\ULTRA.EFI          el kernel; lo arranca la entrada "ULTRA LinuX" del menú F11
A:\ULTRA\sistema-a.erofs        el sistema, de solo lectura (módulos del kernel)
A:\ULTRA\datos.ext4             /datos: PC.configuración y arranques.log
A:\ULTRA\PC-configuracion.txt   copias que deja ULTRA en cada arranque,
A:\ULTRA\arranques.txt          para leerlas directamente desde Windows
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

## 6. Ver los resultados en Windows

Apaga con `poweroff` y arranca Windows. En `A:\ULTRA\` están `PC-configuracion.txt` y `arranques.txt`, actualizados en cada arranque de ULTRA. Para el registro completo del kernel, antes de apagar:

```sh
dmesg > /bmo/ULTRA/dmesg.txt
```

## Si algo falla

| Síntoma | Qué significa | Qué probar |
|---|---|---|
| "ULTRA LinuX" no sale en F11 | El firmware no tomó la entrada, o CSM activo | `.\instalar-en-A.ps1 -Accion Estado`; paso 3 (WHQL en Enabled) |
| "Secure Boot violation" | Secure Boot se volvió a activar | Desactivarlo en `Windows OS Configuration > Secure Boot` |
| Pantalla negra tras elegirla | El firmware no dio framebuffer | Otra salida de la RTX (DisplayPort/HDMI); comprobar CSM apagado |
| El PC se reinicia solo en bucle | Pánico del kernel (`panic=-1` reinicia al instante) | Elegir Windows en F11; anotar lo último visible |
| `ni particiones ... ni FAT32 BMO` | El SSD tardó más de 10 s o `A:` perdió la etiqueta | Comprobar en Windows que `A:` sigue llamándose `BMO` |
| `sistema-a.erofs: No such file` | Faltan los archivos | `.\instalar-en-A.ps1 -Accion Copiar` |
| Windows pide revisar `A:` | ULTRA se apagó sin desmontar (corte de luz, botón) | Dejar que lo revise; `datos.ext4` puede necesitar empezar de cero con `-NuevosDatos` |

Para volver a Windows basta con no elegir ULTRA en F11: Windows sigue siendo el arranque por defecto.

## Límites conocidos

- **Peligro dentro de la shell**: eres root y el kernel ve el disco de Windows (`/dev/nvme0n1`), la partición EFI de Windows y `F:`. El init no monta nada de eso, pero un `dd` o `mkfs` escrito a mano sí podría dañarlo. No ejecutes nada sobre `/dev/nvme*` ni `/dev/sd*`.
- **`/datos` sobre FAT32**: el ext4 tiene diario, pero el FAT32 que lo contiene no. Un corte de luz con ULTRA encendido puede dejar `A:` para revisar.
- **El arranque guiado no gana tiempo en QEMU**: con `A:` la lectura pasa por FAT32 y loop, y salió 1,9 ms más lento que la detección completa. En el PC real se mide en cada arranque (`arranques.txt`).
- **Sin sonido todavía**: se carga el controlador `snd_hda_intel`, pero el códec Realtek de la placa no está compilado y el bus `hdaudio` aún no se recorre.
- **Sin driver NVIDIA**: solo la pantalla básica del firmware; `nvidia-open` llega en una fase posterior.
- **Red sin configurar**: `r8169` crea `eth0`, pero no hay DHCP.

## Alternativa: un USB completo

Si un día hay un USB que se pueda borrar, `build\generico\ultra-esp.img` se escribe entero con Rufus en modo DD. Lleva su propia partición EFI, `ULTRA-SISTEMA-A` y `ULTRA-DATOS`, y se elige con F11 como `UEFI: <USB>`. **Nunca en el disco de `A:`**: ese disco tiene la partición EFI de Windows.
