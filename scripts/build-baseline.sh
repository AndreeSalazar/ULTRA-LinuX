#!/bin/bash
# SPDX-License-Identifier: GPL-2.0-only
set -euo pipefail
# Profiles: produccion is what gets installed; vigia adds the kernel's own
# detectors (KASAN, UBSAN, KFENCE, lockdep) so the kernel reports its mines.
perfil=${1:-produccion}
# start: the upstream config each profile grows from. modules: whether it ships loadable modules.
start=tinyconfig modules=no modules_list=() cmdline=
case $perfil in
    produccion) dir=tiny64-baseline; extra=(KFENCE); required=(KFENCE) ;;
    generico) dir=x86_64-generico start=defconfig modules=yes
        # Boots any modern x86-64 PC: storage, USB, EFI, a firmware framebuffer and
        # the AMD pieces of the target board. Module-capable for nvidia-open later.
        extra=(KFENCE MODULES MODULE_UNLOAD BLK_DEV_NVME ATA SATA_AHCI VIRTIO_PCI VIRTIO_BLK VIRTIO_NET
               USB_XHCI_HCD USB_STORAGE USB_HID HID_GENERIC EXT4_FS VFAT_FS EFI EFI_STUB DRM DRM_SIMPLEDRM
               SYSFB_SIMPLEFB CPU_SUP_AMD MICROCODE_AMD X86_AMD_PSTATE AMD_IOMMU EROFS_FS EFI_PARTITION
               BLK_DEV_LOOP NLS_CODEPAGE_437 NLS_ISO8859_1)
        modules_list=(R8169 SND_HDA_INTEL SENSORS_K10TEMP)
        # No boot loader: the firmware runs the kernel directly, so its arguments live inside it.
        # The last console becomes /dev/console; init starts a shell on every active console.
        cmdline="console=tty0 console=ttyS0,115200 rdinit=/init panic=-1 quiet GLIBC_TUNABLES=glibc.cpu.hwcaps=SHSTK:glibc.cpu.x86_shstk=on"
        required=(KFENCE MODULES BLK_DEV_NVME SATA_AHCI EXT4_FS EROFS_FS EFI_PARTITION VFAT_FS BLK_DEV_LOOP NLS_CODEPAGE_437 NLS_ISO8859_1 EFI_STUB USB_XHCI_HCD DRM_SIMPLEDRM VIRTIO_BLK) ;;
    vigia) dir=tiny64-vigia
        extra=(KFENCE DEBUG_KERNEL KASAN KASAN_GENERIC KASAN_INLINE KASAN_STACK UBSAN UBSAN_BOUNDS UBSAN_SHIFT
               UBSAN_BOOL PROVE_LOCKING DETECT_HUNG_TASK SOFTLOCKUP_DETECTOR DEBUG_ATOMIC_SLEEP)
        required=(KFENCE KASAN KASAN_GENERIC UBSAN UBSAN_BOUNDS PROVE_LOCKING DETECT_HUNG_TASK SOFTLOCKUP_DETECTOR) ;;
    *) echo "ROJO: perfil desconocido $perfil"; exit 1 ;;
esac
base="$HOME/linux-reconstruido"
src="$base/kernel"
out="$base/build/$dir"
art="$base/artifacts/$perfil"
# Userspace lives outside O=: Kbuild needs $out/init as a directory for init/main.c.
user="$base/build/init-c"
lab=$(cd -- "$(dirname -- "$0")" && pwd)
mkdir -p "$out" "$user" "$art"
[[ -d "$out/init" || ! -e "$out/init" ]] || rm -f "$out/init"
rm -f "$art/build-complete"
exec > >(tee "$out/build.log") 2>&1
trap 'echo "ROJO: build fallo en linea $LINENO"' ERR
expected=32c52ec87d6245ee2d12ed5dd509e40062939b50
[[ $(git -C "$src" rev-parse HEAD) = "$expected" ]] || { echo 'ROJO: commit inesperado'; exit 1; }
[[ -z $(git -C "$src" ls-files --deleted) ]] || { echo 'ROJO: archivos fuente ausentes'; exit 1; }
gcc --version | head -n 1
echo "perfil: $perfil"
# Guardians: review gate first (analyzer at -O0, warnings at -O2), then the
# production build. static-pie keeps ASLR, BIND_NOW and CET in static binaries.
source "$lab/guardianes.sh"
init_src=("$lab/init.c" "$lab/particiones.c" "$lab/hardware.c")
for file in "${init_src[@]}" "$lab/caja/caja.c" "$lab/guardian/ultra_guard.c"; do
    ultra_revisar "$file" || { echo "ROJO: $file no pasa la revision de guardianes"; exit 1; }
done
# Static userspace built separately from the unmodified kernel sources.
gcc "${ULTRA_WARN[@]}" "${ULTRA_PROD[@]}" -static-pie "${init_src[@]}" "$lab/guardian/ultra_guard.c" -o "$user/init"
gcc "${ULTRA_WARN[@]}" "${ULTRA_PROD[@]}" -static-pie "$lab/caja/caja.c" "$lab/guardian/ultra_guard.c" -o "$user/caja"
# Production build of the minefield, so the guardians can be watched on the ULTRA kernel itself.
gcc "${ULTRA_PROD[@]}" -w -static-pie "$lab/guardian/minas.c" "$lab/guardian/ultra_guard.c" -o "$user/minas"
for bin in "$user/init" "$user/caja"; do
    readelf -n "$bin" | grep -q 'x86 feature: IBT, SHSTK' || { echo "ROJO: $bin sin marca CET"; exit 1; }
done
printf '%s
' "$(gcc --version | head -n 1)" "revision: ${ULTRA_WARN[*]} -O0 -fanalyzer / -O2"     "produccion: ${ULTRA_WARN[*]} ${ULTRA_PROD[*]} -static-pie" "$GLIBC_TUNABLES" > "$user/guardianes.txt"
busybox=/usr/bin/busybox
[[ -x $busybox ]] || { echo 'ROJO: falta busybox; instalar con pacman -S busybox'; exit 1; }
# A dynamic BusyBox would need a libc that the initramfs does not carry.
! readelf -l "$busybox" | grep -q INTERP || { echo 'ROJO: busybox no es estatico'; exit 1; }
{
    for d in /dev /proc /sys /tmp /run /root /etc /bin /sbin /lib /sistema /datos; do echo "dir $d 0755 0 0"; done
    # Modules live on the read-only system partition; busybox modprobe and lsmod look here.
    echo 'slink /lib/modules /sistema/lib/modules 0777 0 0'
    echo 'nod /dev/console 0600 0 0 c 5 1'
    echo 'nod /dev/ttyS0 0600 0 0 c 4 64'
    echo "file /init $user/init 0755 0 0"
    echo "file /bin/busybox $busybox 0755 0 0"
    echo "file /bin/caja $user/caja 0755 0 0"
    echo "file /bin/minas $user/minas 0755 0 0"
    echo "file /bin/probar-caja $lab/rootfs/bin/probar-caja 0755 0 0"
    for applet in $("$busybox" --list); do
        case $applet in busybox|init|linuxrc) continue;; esac
        echo "slink /bin/$applet busybox 0777 0 0"
    done
    for f in "$lab"/rootfs/etc/*; do echo "file /etc/${f##*/} $f 0644 0 0"; done
} > "$out/initramfs.list"
if [[ ! -f "$out/.config" ]]; then
    make -C "$src" O="$out" ARCH=x86_64 CC=gcc HOSTCC=gcc "$start"
fi
for option in 64BIT PRINTK TTY VT VT_CONSOLE VGA_CONSOLE FONT_8x16 INPUT INPUT_KEYBOARD KEYBOARD_ATKBD SERIO SERIO_I8042 SERIAL_8250 SERIAL_8250_CONSOLE UNIX98_PTYS POSIX_TIMERS EPOLL SIGNALFD TIMERFD EVENTFD FILE_LOCKING PROC_SYSCTL PCI ACPI NET SMP RELOCATABLE RANDOMIZE_KSTACK_OFFSET NAMESPACES UTS_NS PID_NS NET_NS SYSVIPC IPC_NS SECCOMP SECCOMP_FILTER SECURITY SECURITY_LANDLOCK X86_USER_SHADOW_STACK HARDENED_USERCOPY INIT_ON_ALLOC_DEFAULT_ON INIT_STACK_ALL_ZERO STACKPROTECTOR STACKPROTECTOR_STRONG RANDOMIZE_BASE RANDOMIZE_KSTACK_OFFSET_DEFAULT SLAB_FREELIST_RANDOM SLAB_FREELIST_HARDENED LIST_HARDENED BUG_ON_DATA_CORRUPTION SCHED_STACK_END_CHECK FORTIFY_SOURCE BINFMT_ELF BLK_DEV_INITRD DEVTMPFS DEVTMPFS_MOUNT PROC_FS SYSFS SHMEM TMPFS FUTEX MULTIUSER "${extra[@]}"; do
    "$src/scripts/config" --file "$out/.config" --enable "$option"
done
"$src/scripts/config" --file "$out/.config" --set-str INITRAMFS_SOURCE "$out/initramfs.list" --set-str LSM landlock --enable INITRAMFS_COMPRESSION_NONE --disable SLUB_TINY --disable DEBUG_INFO --disable DEBUG_INFO_BTF --disable WERROR
[[ $modules = yes ]] || "$src/scripts/config" --file "$out/.config" --disable MODULES
[[ -z $cmdline ]] || "$src/scripts/config" --file "$out/.config" --enable CMDLINE_BOOL --set-str CMDLINE "$cmdline"
for option in "${modules_list[@]}"; do "$src/scripts/config" --file "$out/.config" --module "$option"; done
make -C "$src" O="$out" ARCH=x86_64 CC=gcc HOSTCC=gcc olddefconfig
for option in 64BIT SERIAL_8250_CONSOLE BINFMT_ELF BLK_DEV_INITRD VT_CONSOLE POSIX_TIMERS EPOLL ACPI PID_NS NET_NS SYSVIPC IPC_NS SECCOMP_FILTER SECURITY_LANDLOCK X86_USER_SHADOW_STACK HARDENED_USERCOPY INIT_ON_ALLOC_DEFAULT_ON RANDOMIZE_BASE RANDOMIZE_KSTACK_OFFSET_DEFAULT STACKPROTECTOR_STRONG SLAB_FREELIST_RANDOM SLAB_FREELIST_HARDENED LIST_HARDENED FORTIFY_SOURCE "${required[@]}"; do
    grep -qx "CONFIG_${option}=y" "$out/.config" || { echo "ROJO: falta CONFIG_$option"; exit 1; }
done
grep -qx 'CONFIG_LSM="landlock"' "$out/.config" || { echo 'ROJO: Landlock no esta en la lista de LSM activos'; exit 1; }
targets=(bzImage); [[ $modules = yes ]] && targets+=(modules)
make -C "$src" O="$out" ARCH=x86_64 CC=gcc HOSTCC=gcc -j"$(nproc)" "${targets[@]}"
if [[ $modules = yes ]]; then
    rm -rf "$art/modulos"
    make -s -C "$src" O="$out" ARCH=x86_64 INSTALL_MOD_PATH="$art/modulos" INSTALL_MOD_STRIP=1 modules_install
    tar -C "$art/modulos" -czf "$art/modulos.tar.gz" lib
    for m in "${modules_list[@]}"; do
        grep -qx "CONFIG_$m=m" "$out/.config" || { echo "ROJO: CONFIG_$m no quedo como modulo"; exit 1; }
    done
fi
cp "$out/arch/x86/boot/bzImage" "$art/bzImage-tiny64"
cp "$out/.config" "$art/tiny64.config"
cp "$user/guardianes.txt" "$art/guardianes.txt"
make -s -C "$src" O="$out" ARCH=x86_64 kernelrelease > "$art/kernelrelease.txt"
sums=("$art/bzImage-tiny64" "$user/init" "$user/caja")
if [[ -n $cmdline ]]; then
    grep -qx 'CONFIG_CMDLINE_BOOL=y' "$out/.config" || { echo 'ROJO: falta la linea de comandos integrada'; exit 1; }
    # Boot disk without GRUB, built with sfdisk, mkfs.fat, mtools, mkfs.erofs and mkfs.ext4:
    # nothing is mounted. GPT, with the init finding the Linux partitions by name:
    #   1 ULTRA-ESP        FAT32, the kernel as the firmware's default loader
    #   2 ULTRA-SISTEMA-A  EROFS, read-only: the modules of this exact kernel
    #   3 ULTRA-DATOS      ext4: what must survive a reboot (PC.configuración)
    command -v mkfs.erofs > /dev/null || { echo 'ROJO: falta mkfs.erofs; instalar con pacman -S erofs-utils'; exit 1; }
    release=$(cat "$art/kernelrelease.txt")
    img="$art/ultra-esp.img" esp_type=C12A7328-F81F-11D2-BA4B-00A0C93EC93B linux_type=0FC63DAF-8483-4772-8E79-3D69D8477DE4
    export MTOOLS_SKIP_CHECK=1
    rm -f "$img"
    # 1 MiB for the GPT, three 64 MiB partitions, and room for the backup GPT at the end.
    truncate -s 194M "$img"
    printf 'label: gpt\nstart=2048, size=64MiB, type=%s, name="ULTRA-ESP"\nsize=64MiB, type=%s, name="ULTRA-SISTEMA-A"\nsize=64MiB, type=%s, name="ULTRA-DATOS"\n' \
        "$esp_type" "$linux_type" "$linux_type" | sfdisk -q "$img"
    # part NAME 1 -> first sector, part NAME 2 -> size in sectors.
    part() { sfdisk --dump "$img" | sed -n "s/.*start= *\([0-9]*\), size= *\([0-9]*\).*name=\"$1\".*/\\$2/p"; }
    for name in ULTRA-ESP ULTRA-SISTEMA-A ULTRA-DATOS; do
        [[ -n $(part "$name" 1) ]] || { echo "ROJO: la particion $name no aparece en la tabla GPT"; exit 1; }
    done
    offset=$(( $(part ULTRA-ESP 1) * 512 )) sectors=$(part ULTRA-ESP 2)
    mkfs.fat -F 32 -s 1 -n ULTRA-ESP --offset=$(part ULTRA-ESP 1) "$img" $((sectors / 2)) > /dev/null
    mmd -i "$img@@$offset" ::/EFI ::/EFI/BOOT
    mcopy -i "$img@@$offset" "$art/bzImage-tiny64" ::/EFI/BOOT/BOOTX64.EFI
    # System A: the modules installed above, without the link back to the build tree.
    sistema="$art/sistema-a" datos="$art/datos-semilla"
    rm -rf "$sistema" "$datos" "$art/sistema-a.erofs" "$art/datos.ext4"
    mkdir -p "$sistema/lib" "$datos"
    cp -a "$art/modulos/lib/modules" "$sistema/lib/"
    rm -f "$sistema/lib/modules/$release/build" "$sistema/lib/modules/$release/source"
    printf 'ULTRA-SISTEMA-A\nkernel=%s\n' "$release" > "$sistema/ULTRA-SISTEMA"
    mkfs.erofs --quiet --all-root -T 0 "$art/sistema-a.erofs" "$sistema"
    printf '%s\n' 'ULTRA-DATOS: lo que sobrevive a un reinicio.' \
        'PC.configuración: componentes y modulos de este PC, escrito por el init en el primer arranque.' \
        'arranques.log: una linea por arranque con los tiempos medidos.' > "$datos/LEEME.txt"
    truncate -s $(( $(part ULTRA-DATOS 2) * 512 )) "$art/datos.ext4"
    mkfs.ext4 -q -F -L ULTRA-DATOS -E root_owner=0:0 -d "$datos" "$art/datos.ext4"
    (( $(stat -c %s "$art/sistema-a.erofs") <= $(part ULTRA-SISTEMA-A 2) * 512 )) ||
        { echo 'ROJO: el sistema EROFS no cabe en ULTRA-SISTEMA-A'; exit 1; }
    for pair in "ULTRA-SISTEMA-A:$art/sistema-a.erofs" "ULTRA-DATOS:$art/datos.ext4"; do
        name=${pair%%:*} file=${pair#*:}
        dd if="$file" of="$img" bs=1M oflag=seek_bytes seek=$(( $(part "$name" 1) * 512 )) conv=notrunc,fsync status=none
        # Guardian: what the firmware and the kernel will read is exactly what was built.
        cmp -s -n "$(stat -c %s "$file")" "$file" "$img" 0 $(( $(part "$name" 1) * 512 )) ||
            { echo "ROJO: $name dentro de la imagen no coincide con su sistema de archivos"; exit 1; }
    done
    # Guardians: the firmware will only find what is really inside the image.
    sfdisk --dump "$img" | grep -qi "type=$esp_type" || { echo 'ROJO: la particion no es de tipo EFI System'; exit 1; }
    [[ $(mtype -i "$img@@$offset" ::/EFI/BOOT/BOOTX64.EFI | sha256sum | cut -d' ' -f1) = $(sha256sum < "$art/bzImage-tiny64" | cut -d' ' -f1) ]] ||
        { echo 'ROJO: BOOTX64.EFI no es el kernel recien compilado'; exit 1; }
    [[ -f "$sistema/lib/modules/$release/modules.alias" ]] || { echo "ROJO: ULTRA-SISTEMA-A sin modules.alias para $release"; exit 1; }
    echo "VERDE: disco EFI sin GRUB: $(basename "$img"): ESP FAT32 $((sectors / 2048)) MiB, ULTRA-SISTEMA-A EROFS $(( $(stat -c %s "$art/sistema-a.erofs") / 1024 )) KiB, ULTRA-DATOS ext4 $(( $(part ULTRA-DATOS 2) / 2048 )) MiB"
    # Second layout: ULTRA as files on a FAT32 partition labelled BMO (A:\ on the PC),
    # on a disk shared with the Windows EFI partition. This test disk copies that shape:
    #   1 EFI System  FAT32 "Windows": EFI/Microsoft/Boot, which the init must never mount
    #   2 Basic data  FAT32, label BMO only in the root directory, as Windows writes it
    #   3 Basic data  without a file system, like F:
    # QEMU starts the kernel from partition 1; the real PC from a firmware entry on BMO.
    bmo="$art/disco-bmo.img" basic_type=EBD0A0A2-B9E5-4433-87C0-68B6B72699C7
    rm -f "$bmo"
    truncate -s 226M "$bmo"
    printf 'label: gpt\nstart=2048, size=64MiB, type=%s, name="EFI system partition"\nsize=144MiB, type=%s, name="Basic data partition"\nsize=16MiB, type=%s, name="Basic data partition"\n' \
        "$esp_type" "$basic_type" "$basic_type" | sfdisk -q "$bmo"
    bpart() { sfdisk --dump "$bmo" | sed -n "s/^[^ ]*$1 : start= *\([0-9]*\), size= *\([0-9]*\).*/\\$2/p"; }
    win=$(( $(bpart 1 1) * 512 )) a=$(( $(bpart 2 1) * 512 ))
    mkfs.fat -F 32 -s 1 --offset=$(bpart 1 1) "$bmo" $(( $(bpart 1 2) / 2 )) > /dev/null 2>&1
    mmd -i "$bmo@@$win" ::/EFI ::/EFI/BOOT ::/EFI/Microsoft ::/EFI/Microsoft/Boot
    mcopy -i "$bmo@@$win" "$art/bzImage-tiny64" ::/EFI/BOOT/BOOTX64.EFI
    echo 'no es Windows: marca de prueba' > "$art/bootmgfw-falso.efi"
    mcopy -i "$bmo@@$win" "$art/bootmgfw-falso.efi" ::/EFI/Microsoft/Boot/bootmgfw.efi
    mkfs.fat -F 32 -s 1 -n BMO --offset=$(bpart 2 1) "$bmo" $(( $(bpart 2 2) / 2 )) > /dev/null 2>&1
    # Windows keeps the label in the root directory and leaves NO NAME in the boot sector.
    printf 'NO NAME    ' | dd of="$bmo" bs=1 seek=$(( a + 71 )) conv=notrunc status=none
    mmd -i "$bmo@@$a" ::/EFI ::/EFI/ULTRA ::/ULTRA
    mcopy -i "$bmo@@$a" "$art/bzImage-tiny64" ::/EFI/ULTRA/ULTRA.EFI
    mcopy -i "$bmo@@$a" "$art/sistema-a.erofs" ::/ULTRA/sistema-a.erofs
    mcopy -i "$bmo@@$a" "$art/datos.ext4" ::/ULTRA/datos.ext4
    [[ $(mtype -i "$bmo@@$a" ::/ULTRA/sistema-a.erofs | sha256sum | cut -d' ' -f1) = $(sha256sum < "$art/sistema-a.erofs" | cut -d' ' -f1) ]] ||
        { echo 'ROJO: sistema-a.erofs dentro de BMO no coincide'; exit 1; }
    mlabel -i "$bmo@@$a" -s :: | grep -q 'BMO' || { echo 'ROJO: la particion de prueba no se llama BMO'; exit 1; }
    echo "VERDE: disco de prueba $(basename "$bmo"): EFI de Windows simulada, FAT32 BMO con ULTRA en archivos y particion vacia"
    sums+=("$img" "$bmo" "$art/sistema-a.erofs" "$art/datos.ext4")
fi
sha256sum "${sums[@]}" > "$art/SHA256SUMS"
printf 'VERDE: kernel %s e init C compilados. Arranque QEMU todavia pendiente.\n' "$perfil"
touch "$art/build-complete"
