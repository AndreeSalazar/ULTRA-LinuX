# Arranque sin GRUB

ULTRA LinuX arranca **sin cargador de arranque**. El firmware UEFI del PC lee una partición FAT32 y ejecuta directamente el kernel, que es a la vez un programa EFI gracias a `EFI_STUB`.

```
ultra-esp.img (194 MiB, GPT)
├── 1 ULTRA-ESP        EFI System, FAT32, 64 MiB
│   └── EFI/BOOT/BOOTX64.EFI      ← el bzImage del perfil generico
├── 2 ULTRA-SISTEMA-A  Linux, EROFS (solo lectura), 64 MiB → /sistema
│   ├── ULTRA-SISTEMA             nombre y versión del kernel
│   └── lib/modules/<versión>/    los módulos de ese kernel exacto
└── 3 ULTRA-DATOS      Linux, ext4, 64 MiB → /datos
    ├── LEEME.txt
    ├── PC.configuración          lo escribe el init en el primer arranque
    └── arranques.log             una línea por arranque, con tiempos
```

`EFI/BOOT/BOOTX64.EFI` es la ruta que cualquier firmware UEFI x86-64 busca por defecto en un disco extraíble, así que no hace falta configurar nada.

## Particiones por nombre

El kernel sigue arrancando con la raíz en memoria (el initramfs integrado). El init busca después las particiones por su **nombre GPT**, que el kernel publica como `PARTNAME=` en `/sys/class/block/*/uevent`. No usa el orden de los discos: el mismo USB es `vda` en QEMU y `sdb` en el PC.

- `ULTRA-SISTEMA-A` se monta en `/sistema` como EROFS, `ro,nodev,nosuid`. EROFS no tiene escritura en absoluto, así que el sistema no puede cambiar ni por error. `/lib/modules` apunta ahí, y `lsmod` y `modprobe` de BusyBox funcionan.
- `ULTRA-DATOS` se monta en `/datos` como ext4, `nodev,nosuid`. Al apagar o reiniciar, el init la desmonta (o la deja en solo lectura si algo la tiene abierta) antes de `reboot`.
- Si una partición no aparece, el init espera hasta 10 s (los USB tardan) y sigue con `AMARILLO`. Si **dos** particiones tienen el mismo nombre, no monta ninguna (`ROJO`): adivinar podría montar la equivocada.
- Ningún otro nombre se monta nunca. El disco de Windows queda visible para el kernel, pero intacto.

La letra `A` deja sitio para un `ULTRA-SISTEMA-B` y actualizaciones A/B al estilo SteamOS (fase 5).

## Detección de hardware con memoria

En el **primer arranque** en un PC, el init lee el `modalias` de cada dispositivo de `/sys/bus/pci/devices` y `/sys/bus/usb/devices`, lo compara (`fnmatch`) con `modules.alias` de `/sistema`, carga los módulos que coinciden con sus dependencias de `modules.dep` (con `finit_module`, sin `modprobe`) y escribe `/datos/PC.configuración`:

```
kernel=7.3.0-rc7-g32c52ec87d62
huella_pci=…                ← FNV-1a de los modalias PCI
deteccion_us=677
carga_us=10427

[componentes]
pci 0000:00:03.0 | Intel 8086:2668 sonido HD | driver=snd_hda_intel | modulos=snd_hda_intel | pci:v00008086d…
…
[modulos]
snd_hda_intel
```

En los **arranques siguientes**, si el kernel es el mismo y la huella PCI coincide, carga solo los módulos de la lista y mide el tiempo frente al primer arranque. Si la huella cambia (el USB se movió a otro PC), detecta de nuevo. Borrar el archivo también fuerza otra detección.

El archivo vive en una partición escribible, así que se trata como dato: los nombres se validan (`[a-z0-9_]`) y solo pueden elegir módulos que ya están en `ULTRA-SISTEMA-A`, que es de solo lectura. Un archivo manipulado no puede aportar código.

**Medida en QEMU** (`evidencias/arranque-disco-persistente.log`):

| Arranque | Lectura o detección | Carga (6 módulos) | Total |
|---|---|---|---|
| 1, detección completa | 677 µs | 10 427 µs | 11 104 µs |
| 2, guiado | 557 µs | 10 633 µs | 11 190 µs |

**Ganancia: ninguna (−86 µs).** Con 239 alias y 7 dispositivos PCI, comparar cuesta menos de 1 ms. El modo guiado también lee los modalias PCI (para la huella) y el archivo, y la carga de los módulos domina las dos columnas. La ganancia solo puede aparecer con muchos más módulos (una distribución trae miles y unos 30 000 alias) o con más dispositivos. Por eso el init la mide en cada arranque en vez de suponerla.

## Cómo se construye

Lo hace `scripts/build-baseline.sh generico` al final de la compilación, sin montar nada y sin permisos de root:

1. `truncate` crea el archivo de imagen.
2. `sfdisk` escribe la tabla GPT con las tres particiones y sus nombres.
3. `mkfs.fat -F 32 --offset=2048` crea el FAT32 dentro de la ESP; `mmd` y `mcopy` (de mtools) crean `EFI/BOOT` y copian el kernel.
4. `mkfs.erofs --all-root -T 0` crea el sistema A a partir de los módulos instalados.
5. `mkfs.ext4 -d` crea `ULTRA-DATOS` con su contenido inicial, con dueño root.
6. `dd conv=notrunc` coloca cada sistema de archivos en el sector inicial de su partición.

Guardianes de la imagen:
- La tabla debe contener una partición de tipo EFI System y las tres particiones por nombre.
- `BOOTX64.EFI` dentro de la imagen debe tener el mismo SHA-256 que el kernel recién compilado.
- El EROFS debe caber en su partición, y `cmp` comprueba que lo que hay en cada partición es, byte a byte, el sistema de archivos construido.
- `ULTRA-SISTEMA-A` debe traer `modules.alias` del kernel exacto. `EROFS_FS` y `EFI_PARTITION` son opciones obligatorias del perfil.
- La imagen entra en `SHA256SUMS`, y la descarga, `VerDisco` y la prueba comprueban ese hash antes de usarla.

## Parámetros del kernel sin cargador

Nadie le pasa argumentos al kernel, así que van integrados (`CONFIG_CMDLINE`):

```
console=tty0 console=ttyS0,115200 rdinit=/init panic=-1 quiet GLIBC_TUNABLES=glibc.cpu.hwcaps=SHSTK:glibc.cpu.x86_shstk=on
```

La última consola de la lista se convierte en `/dev/console`. El init lee `/sys/class/tty/console/active` y lanza **una shell supervisada en cada consola**: `tty1` para la pantalla y `ttyS0` para la consola serie. La misma imagen sirve para el monitor de un PC y para las pruebas automáticas.

## Probarlo

```powershell
.\construir-kernel.ps1 -Accion Compilar -Perfil generico
.\construir-kernel.ps1 -Accion Descargar -Perfil generico
.\construir-kernel.ps1 -Accion VerDisco -Perfil generico
```

`VerDisco` arranca QEMU con su firmware UEFI (`edk2-x86_64-code.fd`, OVMF) y la imagen como disco virtio. No usa `-kernel` ni GRUB, y la imagen se abre en modo `snapshot`, así que no se modifica.

La prueba automática equivalente es:

```powershell
& "$env:USERPROFILE\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe" scripts\probar_arranque.py generico --disco
```

Además de la prueba completa, comprueba que se arrancó por UEFI (`/sys/firmware/efi/fw_platform_size` = 64), que llegó la línea de comandos integrada (`/proc/cmdline`) y que la consola serie está activa. Después prueba el disco en dos arranques:

1. **Arranque 1** sobre una copia nueva de la imagen (`ultra-esp-prueba.img`; la verificada no se escribe nunca), con un controlador Intel HD Audio añadido en QEMU para que la detección tenga algo que encontrar. Comprueba: `/sistema` es EROFS `ro` y rechaza escrituras, `/datos` es ext4 `rw`, el init hizo la detección completa, `PC.configuración` lista `snd_hda_intel` y `lsmod` lo muestra cargado. Luego escribe un testigo aleatorio en `/datos`.
2. **`reboot` dentro del sistema**: el init desmonta `/datos` limpio y reinicia.
3. **Arranque 2** sobre el mismo disco: el init arranca guiado por `PC.configuración`, el testigo sigue ahí, `snd_hda_intel` está cargado y `arranques.log` tiene las dos líneas.

Con WHPX, cuando el invitado se reinicia, la vCPU se queda atascada en el firmware (`failed to get xsave state`). Por eso la prueba usa `-no-reboot`: el `reboot` del invitado cierra QEMU y un segundo QEMU arranca el mismo disco. El camino del init es el mismo que en un PC (desmontar y `RB_AUTOBOOT`), y lo que se lee en el arranque 2 solo puede venir del disco. Resultado: todo VERDE (`evidencias/arranque-disco-persistente.log`).

`VerDisco` sigue abriendo la imagen en `snapshot`: lo que se escriba en `/datos` desde la ventana se pierde al cerrarla.

## Hacia el PC real (todavía no probado)

La guía paso a paso para la MSI A320M, el Ryzen 5600X y la RTX 3060, sin tocar Windows, está en [PRUEBA-PC-REAL.md](PRUEBA-PC-REAL.md).

## Más adelante

- **Sistema completo en EROFS:** BusyBox, glibc y los programas pasan del initramfs a `ULTRA-SISTEMA-A`, y el initramfs se queda solo con el init.
- **Unified Kernel Image (UKI):** kernel, línea de comandos e initramfs en un solo `.efi` firmable. Es la dirección de Arch y SteamOS modernos.
- **Menú opcional:** si un día hace falta elegir entre varios kernels, `systemd-boot` o rEFInd son mucho más simples que GRUB.
