# Arranque sin GRUB

ULTRA LinuX arranca **sin cargador de arranque**. El firmware UEFI del PC lee una partición FAT32 y ejecuta directamente el kernel, que es a la vez un programa EFI gracias a `EFI_STUB`.

```
ultra-esp.img (64 MiB, GPT)
└── partición 1: EFI System, FAT32 "ULTRA-ESP"
    └── EFI/BOOT/BOOTX64.EFI   ← el bzImage del perfil generico
```

`EFI/BOOT/BOOTX64.EFI` es la ruta que cualquier firmware UEFI x86-64 busca por defecto en un disco extraíble, así que no hace falta configurar nada.

## Cómo se construye

Lo hace `scripts/build-baseline.sh generico` al final de la compilación, sin montar nada y sin permisos de root:

1. `truncate` crea el archivo de imagen.
2. `sfdisk` escribe una tabla GPT con una partición de tipo EFI System (`C12A7328-F81F-11D2-BA4B-00A0C93EC93B`).
3. `mkfs.fat -F 32 --offset=2048` crea el FAT32 dentro de esa partición.
4. `mmd` y `mcopy` (de mtools) crean `EFI/BOOT` y copian el kernel.

Guardianes de la imagen:
- La tabla debe contener una partición de tipo EFI System.
- `BOOTX64.EFI` dentro de la imagen debe tener el mismo SHA-256 que el kernel recién compilado.
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

Además de la prueba completa, comprueba que se arrancó por UEFI (`/sys/firmware/efi/fw_platform_size` = 64), que llegó la línea de comandos integrada (`/proc/cmdline`) y que la consola serie está activa.

## Hacia el PC real (todavía no probado)

El kernel genérico aún no ha arrancado en el metal. Cuando se haga:

1. **Usar un USB, no el disco de Windows.** Se escribe `ultra-esp.img` en un USB que se pueda borrar entero, con una herramienta de imagen en modo DD (por ejemplo Rufus en modo "DD"). Así se reemplaza todo el contenido del USB y **no se toca** el disco ni la ESP de Windows.
2. **Secure Boot desactivado** en la BIOS de la MSI, porque el kernel no está firmado. La alternativa a futuro es firmarlo con claves propias.
3. **Menú de arranque de la placa** (en MSI suele ser F11): elegir la entrada UEFI del USB.
4. **Qué esperar:**
   - la shell `ultra:~#` en la pantalla, si el firmware ofrece framebuffer (`simpledrm`);
   - sin red ni disco persistente todavía: la raíz vive en memoria;
   - la RTX 3060 se usa solo como pantalla básica del firmware; `nvidia-open` llega más adelante.
5. Si algo falla, quitar el USB devuelve el PC a Windows sin cambios.

## Más adelante

- **Raíz persistente:** una segunda partición ext4 para el sistema, porque FAT32 no tiene permisos, propietarios ni enlaces simbólicos. La ESP sigue en FAT32 y sigue sin GRUB.
- **Unified Kernel Image (UKI):** kernel, línea de comandos e initramfs en un solo `.efi` firmable. Es la dirección de Arch y SteamOS modernos.
- **Menú opcional:** si un día hace falta elegir entre varios kernels, `systemd-boot` o rEFInd son mucho más simples que GRUB.
