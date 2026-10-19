# Probar ULTRA LinuX en el PC real desde un USB

PC: MSI A320M-A PRO MAX, Ryzen 5 5600X y RTX 3060. Todo arranca desde un USB y **Windows no se toca**: el init solo monta particiones que se llamen `ULTRA-SISTEMA-A` y `ULTRA-DATOS`, y esas existen únicamente en el USB.

Estado: probado en QEMU con firmware UEFI (`evidencias/arranque-disco-persistente.log`). **Todavía no ha arrancado en el metal.** Esta guía es el primer intento.

## Qué hace falta

- Un USB de 1 GB o más que se pueda **borrar entero**. Todo lo que tenga se pierde.
- `build\generico\ultra-esp.img` descargado y verificado (`.\construir-kernel.ps1 -Accion Descargar -Perfil generico`).
- [Rufus](https://rufus.ie), o cualquier programa que escriba imágenes en modo DD.
- Un teclado USB (o PS/2) y el monitor conectado a la RTX 3060.

## 0. Antes de nada: BitLocker

Si Windows tiene BitLocker o "cifrado de dispositivo" activo, **cambiar Secure Boot puede hacer que Windows pida la clave de recuperación** en el siguiente arranque. No borra nada, pero sin la clave no se entra.

En PowerShell como administrador:

```powershell
manage-bde -status C:
```

Si dice `Protección activada`, anota la clave de recuperación (aparece con `manage-bde -protectors -get C:`, o en https://aka.ms/myrecoverykey) y suspende la protección hasta que termines:

```powershell
manage-bde -protectors -disable C: -RebootCount 3
```

Se reactiva sola tras 3 reinicios.

## 1. Escribir el USB

1. Comprueba que la imagen es la que se compiló:
   ```powershell
   (Get-FileHash build\generico\ultra-esp.img).Hash
   Select-String ultra-esp.img build\generico\SHA256SUMS
   ```
   Los dos valores deben coincidir.
2. Abre Rufus, elige **el USB** en "Dispositivo" (Rufus solo muestra discos extraíbles, no el disco de Windows) y selecciona `ultra-esp.img`.
3. Si Rufus pregunta el modo, elige **DD**. Pulsa Empezar y acepta que borra el USB.

El USB queda con tres particiones GPT de 64 MiB: `ULTRA-ESP` (FAT32, con el kernel), `ULTRA-SISTEMA-A` (EROFS, solo lectura) y `ULTRA-DATOS` (ext4). El resto del USB queda sin usar. Windows puede ofrecer "formatear" las particiones que no entiende: **di que no**.

## 2. Ajustes de la BIOS (MSI Click BIOS)

Entra con **Supr** al encender.

1. **Secure Boot desactivado**: `Settings > Advanced > Windows OS Configuration > Secure Boot`. El kernel no está firmado y con Secure Boot el firmware lo rechaza.
2. **Arranque UEFI**: en ese mismo menú, `Windows 10/11 WHQL Support` en **Enabled** (equivale a CSM apagado). La imagen solo arranca por UEFI y así la RTX 3060 ofrece su pantalla UEFI (GOP), que es la que usa ULTRA LinuX.
3. No cambies el orden de arranque: se usa el menú de una sola vez.

Guarda con F10.

## 3. Arrancar

Con el USB puesto, enciende y pulsa **F11** hasta que salga el menú de arranque. Elige la entrada **`UEFI: <nombre del USB>`**. Si solo aparece sin `UEFI:`, el paso 2.2 no quedó guardado.

Qué deberías ver en la pantalla:

```
ULTRA LinuX: INIT C READY
VERDE [KERNEL]: Linux 7.3.0-rc7-... x86_64
AMARILLO [PARTICION]: particiones buscadas durante 1200 ms     ← normal en USB: tarda en aparecer
VERDE [PARTICION]: ULTRA-SISTEMA-A (/dev/sdb2, erofs, solo lectura) en /sistema
VERDE [PARTICION]: ULTRA-DATOS (/dev/sdb3, ext4) en /datos
VERDE [HARDWARE]: primer arranque en este PC (...): deteccion completa
VERDE [HARDWARE]: N componentes, M modulos necesarios (...); deteccion ... us + carga ... us
VERDE [HARDWARE]: guardado en /datos/PC.configuración
ultra:~#
```

La letra del disco (`sdb`, `sdc`…) depende de cuántos discos tenga el PC; da igual, porque se busca por nombre.

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

## 5. Comprobar la memoria entre arranques

```sh
reboot
```

Pulsa F11 otra vez y elige el USB. Ahora debe salir:

```
VERDE [HARDWARE]: arranque guiado por PC.configuracion: M modulos (...); lectura ... us + carga ... us = ... us; primer arranque ... us; ganado ... us
```

y `cat /datos/arranques.log` debe tener una línea `arranque=deteccion` y otra `arranque=guiado`. Haz varios reinicios para tener varias medidas.

Para forzar otra detección: `rm /datos/PC.config*` y reiniciar. Si mueves el USB a otro PC, la huella PCI cambia y detecta de nuevo solo.

## 6. Llevarte los resultados a Windows

Windows no lee ext4, pero sí la partición FAT32 del USB. Desde la shell de ULTRA:

```sh
esp=$(grep -l PARTNAME=ULTRA-ESP /sys/class/block/*/uevent | cut -d/ -f5)
mkdir -p /esp && mount -t vfat /dev/$esp /esp
cp /datos/PC.config* /esp/PC-configuracion.txt
cp /datos/arranques.log /esp/arranques.txt
dmesg > /esp/dmesg.txt
umount /esp
poweroff
```

En Windows, el USB aparece como una unidad `ULTRA-ESP` con esos archivos. Si no le asigna letra, una foto de la pantalla también sirve.

## 7. Volver a Windows

1. Apaga, quita el USB y enciende: arranca Windows como siempre.
2. Si quieres, vuelve a activar Secure Boot en la BIOS (paso 2.1).
3. Si suspendiste BitLocker, se reactiva solo; puedes forzarlo con `manage-bde -protectors -enable C:`.

## Si algo falla

| Síntoma | Qué significa | Qué probar |
|---|---|---|
| El USB no sale en F11, o sale sin `UEFI:` | CSM activo o USB mal escrito | Paso 2.2; volver a escribir con Rufus en modo DD |
| "Secure Boot violation" | Secure Boot sigue activo | Paso 2.1 |
| Pantalla negra tras elegir el USB | El firmware no dio framebuffer | Otra salida de la RTX (DisplayPort/HDMI); comprobar que CSM está apagado |
| El PC se reinicia solo en bucle | Pánico del kernel (`panic=-1` reinicia al instante) | Quitar el USB; anotar lo último visible en pantalla |
| `AMARILLO [PARTICION]: ULTRA-DATOS no encontrada` | El USB tardó más de 10 s o no tiene esa partición | Puerto USB trasero de la placa; volver a escribir la imagen |
| `ROJO [PARTICION]: 2 particiones se llaman ...` | Hay dos USB de ULTRA puestos | Dejar solo uno: el init se niega a adivinar |
| Sin teclado | El teclado no es USB HID estándar o está en un hub | Puerto USB trasero, o un teclado PS/2 |

## Límites conocidos

- **Peligro dentro de la shell**: eres root y el kernel ve el disco de Windows (`/dev/nvme0n1` o `/dev/sda`). El init no lo monta nunca, pero un `dd` o `mkfs` escrito a mano sí podría dañarlo. No ejecutes nada sobre `/dev/nvme*` ni `/dev/sd*` salvo los comandos de esta guía.
- **Sin sonido todavía**: se carga el controlador `snd_hda_intel`, pero el códec Realtek de la placa (`SND_HDA_CODEC_REALTEK`) no está compilado, y sus dispositivos aparecen en el bus `hdaudio`, que la detección aún no recorre.
- **Sin driver NVIDIA**: solo la pantalla básica del firmware; `nvidia-open` llega en una fase posterior.
- **Red sin configurar**: `r8169` crea `eth0`, pero no hay DHCP. La RTL8111 puede avisar de que falta su firmware `rtl8168h-2.fw` y aun así funcionar.
- **Dispositivos USB nuevos** después del primer arranque no provocan otra detección: la huella solo mira PCI.
