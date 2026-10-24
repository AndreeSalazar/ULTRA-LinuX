#!/bin/bash
# SPDX-License-Identifier: GPL-2.0-only
set -euo pipefail
# Profiles: produccion is what gets installed; vigia adds the kernel's own
# detectors (KASAN, UBSAN, KFENCE, lockdep) so the kernel reports its mines.
perfil=${1:-produccion}
# start: the upstream config each profile grows from. modules: whether it ships loadable modules.
# userflags: extra GCC options for init, caja and minas. localversion: tells kernels apart in uname -r.
start=tinyconfig modules=no modules_list=() cmdline= userflags=() localversion=
case $perfil in
    produccion) dir=tiny64-baseline; extra=(KFENCE); required=(KFENCE) ;;
    generico|pc) dir=x86_64-generico start=defconfig modules=yes
        # Boots any modern x86-64 PC: storage, USB, EFI, a firmware framebuffer and
        # the AMD pieces of the target board. Module-capable for nvidia-open later.
        extra=(KFENCE PSI MODULES MODULE_UNLOAD BLK_DEV_NVME ATA SATA_AHCI VIRTIO_PCI VIRTIO_BLK VIRTIO_NET
               USB_XHCI_HCD USB_STORAGE USB_HID HID_GENERIC EXT4_FS VFAT_FS EFI EFI_STUB DRM DRM_SIMPLEDRM
               SYSFB_SIMPLEFB CPU_SUP_AMD MICROCODE_AMD X86_AMD_PSTATE AMD_IOMMU EROFS_FS EFI_PARTITION
               BLK_DEV_LOOP NLS_CODEPAGE_437 NLS_ISO8859_1 NTFS3_FS
               # simpledrm alone draws nothing: text on the firmware screen needs fbdev emulation and fbcon.
               DRM_FBDEV_EMULATION FRAMEBUFFER_CONSOLE
               # efivarfs built in: the init itself prepares the F11 entry of the tuned kernel.
               EFIVAR_FS
               # /dev/fb0, so the init can draw the ULTRA logo on the firmware screen.
               FB_DEVICE
               # Arch's system as a root: EROFS below, a writable layer in memory on top.
               OVERLAY_FS USER_NS MEMCG CFS_BANDWIDTH CGROUP_PIDS)
        modules_list=(R8169 SND_HDA_INTEL SND_HDA_GENERIC SND_HDA_CODEC_REALTEK SND_HDA_CODEC_HDMI SND_USB_AUDIO SENSORS_K10TEMP)
        # No boot loader: the firmware runs the kernel directly, so its arguments live inside it.
        # The last console becomes /dev/console; init starts a shell on every active console.
        # libahci.ignore_sss=1 (the parameter lives in libahci.c, so "ahci." is silently ignored):
        # the board sets staggered spin-up, so AHCI scanned the SATA ports one
        # by one and init waited ~1.6 s for it (evidencias/pc-real/dmesg-*). That protects spinning
        # disks from a power surge; this PC has only SSDs, so the ports are scanned in parallel.
        cmdline="console=tty0 console=ttyS0,115200 rdinit=/init panic=-1 quiet libahci.ignore_sss=1 firmware_class.path=/sistema/usr/lib/firmware GLIBC_TUNABLES=glibc.cpu.hwcaps=SHSTK:glibc.cpu.x86_shstk=on"
        required=(KFENCE PSI MODULES BLK_DEV_NVME SATA_AHCI EXT4_FS EROFS_FS EFI_PARTITION VFAT_FS BLK_DEV_LOOP NLS_CODEPAGE_437 NLS_ISO8859_1 EFI_STUB USB_XHCI_HCD DRM_SIMPLEDRM DRM_FBDEV_EMULATION FRAMEBUFFER_CONSOLE EFIVAR_FS FB_DEVICE OVERLAY_FS EROFS_FS_ZIP VIRTIO_BLK USER_NS) ;;
    vigia) dir=tiny64-vigia
        extra=(KFENCE DEBUG_KERNEL KASAN KASAN_GENERIC KASAN_INLINE KASAN_STACK UBSAN UBSAN_BOUNDS UBSAN_SHIFT
               UBSAN_BOOL PROVE_LOCKING DETECT_HUNG_TASK SOFTLOCKUP_DETECTOR DEBUG_ATOMIC_SLEEP)
        required=(KFENCE KASAN KASAN_GENERIC UBSAN UBSAN_BOUNDS PROVE_LOCKING DETECT_HUNG_TASK SOFTLOCKUP_DETECTOR) ;;
    *) echo "ROJO: perfil desconocido $perfil"; exit 1 ;;
esac
if [[ $perfil = generico || $perfil = pc ]]; then required+=(MEMCG CFS_BANDWIDTH CGROUP_PIDS); fi
if [[ $perfil = pc ]]; then
    # generico tuned for this project's Ryzen 5 5600X: Zen 3 only, by design. The workshop
    # VM runs with -cpu host, so -march=native here is the real CPU. Guardians stay intact:
    # -O3 and LTO come after ULTRA_PROD, and must pass the same review and the same boot test.
    dir=x86_64-pc localversion=-pc
    extra+=(X86_NATIVE_CPU) required+=(X86_NATIVE_CPU)
    userflags=(-O3 -march=znver3 -flto=auto)
    # W3a: the RTX 3060 drives the screen with KMS (what Wayland needs) and its own console.
    # Its firmware (GSP) lives in the system image, mounted at /sistema before the module loads.
    cmdline=${cmdline/ GLIBC_TUNABLES/ nvidia-drm.modeset=1 nvidia-drm.fbdev=1 GLIBC_TUNABLES}
fi
base="$HOME/linux-reconstruido"
src="$base/kernel"
out="$base/build/$dir"
art="$base/artifacts/$perfil"
# Userspace lives outside O=: Kbuild needs $out/init as a directory for init/main.c.
user="$base/build/init-c-$perfil"
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
init_src=("$lab/init.c" "$lab/particiones.c" "$lab/hardware.c" "$lab/tarjeta.c" "$lab/efi.c" "$lab/perfil.c" "$lab/logo.c" "$lab/sistema.c")
for file in "${init_src[@]}" "$lab/raiz.c" "$lab/red.c" "$lab/recursos.c" "$lab/guardian/produccion/ultra_custodia.c" "$lab/guardian/produccion/ultra_owner.c" "$lab/caja/caja.c" "$lab/guardian/ultra_guard.c"; do
    ultra_revisar "$file" || { echo "ROJO: $file no pasa la revision de guardianes"; exit 1; }
done
# Static userspace built separately from the unmodified kernel sources.
gcc "${ULTRA_WARN[@]}" "${ULTRA_PROD[@]}" "${userflags[@]}" -static-pie "${init_src[@]}" "$lab/guardian/ultra_guard.c" -o "$user/init"
gcc "${ULTRA_WARN[@]}" "${ULTRA_PROD[@]}" "${userflags[@]}" -static-pie "$lab/caja/caja.c" "$lab/guardian/ultra_guard.c" -o "$user/caja"
gcc "${ULTRA_WARN[@]}" "${ULTRA_PROD[@]}" "${userflags[@]}" -static-pie "$lab/raiz.c" "$lab/guardian/ultra_guard.c" -o "$user/ultra-raiz"
gcc "${ULTRA_WARN[@]}" "${ULTRA_PROD[@]}" "${userflags[@]}" -static-pie "$lab/red.c" "$lab/guardian/ultra_guard.c" -o "$user/ultra-red-contadores"
gcc "${ULTRA_WARN[@]}" "${ULTRA_PROD[@]}" "${userflags[@]}" -static-pie "$lab/recursos.c" "$lab/guardian/produccion/ultra_custodia.c" "$lab/guardian/produccion/ultra_owner.c" "$lab/guardian/ultra_guard.c" -o "$user/ultra-recursos-contadores"
# Production build of the minefield, so the guardians can be watched on the ULTRA kernel itself.
# Same options as init: a guardian that -O3 or LTO optimizes away shows up in the boot test.
gcc "${ULTRA_PROD[@]}" "${userflags[@]}" -w -static-pie "$lab/guardian/minas.c" "$lab/guardian/ultra_guard.c" -o "$user/minas"
audit=$(mktemp -d "$user/elf-produccion.XXXXXXXX")
for name in init caja ultra-raiz ultra-red-contadores ultra-recursos-contadores minas; do
    python "$lab/auditar-elf-produccion.py" "$user/$name" --out "$audit/$name"
done
python "$lab/inspeccionar-custodia-hex.py" "$user/ultra-recursos-contadores" "$audit/hex-recursos" main
sha256sum "${init_src[@]}" "$lab/raiz.c" "$lab/red.c" "$lab/recursos.c" "$lab/guardian/produccion/ultra_custodia.c" "$lab/guardian/produccion/ultra_owner.c" "$lab/caja/caja.c" \
    "$lab/guardian/produccion/ultra_guard.c" "$lab/guardian/produccion/ultra_guard.h" \
    "$lab/guardian/produccion/ultra_custodia.h" "$lab/guardian/produccion/ultra_owner.h" "$lab/guardian/produccion/ultra_string.h" "$lab/guardian/produccion/perfil-gcc.sh" \
    "$lab/auditar-elf-produccion.py" "$lab/inspeccionar-custodia-hex.py" "$lab/inspeccionar-asm-hex.py" "$lab/build-baseline.sh" > "$audit/fuentes.sha256"
tar -C "$audit" -czf "$user/elf-produccion.tar.gz" .
printf '%s
' "$(gcc --version | head -n 1)" "revision: ${ULTRA_WARN[*]} -O0 -fanalyzer / -O2"     "produccion: ${ULTRA_WARN[*]} ${ULTRA_PROD[*]} ${userflags[*]} -static-pie" "$GLIBC_TUNABLES" > "$user/guardianes.txt"
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
    echo 'file /etc/localtime /usr/share/zoneinfo/America/Lima 0644 0 0'
    echo "file /init $user/init 0755 0 0"
    echo "file /bin/busybox $busybox 0755 0 0"
    echo "file /bin/caja $user/caja 0755 0 0"
    echo "file /bin/minas $user/minas 0755 0 0"
    echo "file /bin/probar-caja $lab/rootfs/bin/probar-caja 0755 0 0"
    echo "file /bin/ultra-entrar $lab/rootfs/bin/ultra-entrar 0755 0 0"
    echo "file /bin/ultra-escritorio $lab/rootfs/bin/ultra-escritorio 0755 0 0"
    echo "file /bin/e $lab/rootfs/bin/e 0755 0 0"
    echo "file /bin/ultra-raiz $user/ultra-raiz 0755 0 0"
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
if [[ $perfil == generico || $perfil == pc ]]; then
    # The resource observer requires real system PSI, not missing files/NA.
    "$src/scripts/config" --file "$out/.config" --disable PSI_DEFAULT_DISABLED
fi
[[ $modules = yes ]] || "$src/scripts/config" --file "$out/.config" --disable MODULES
# CMDLINE_OVERRIDE: only the built-in line. Without it, the EFI stub appends the firmware's load
# options: an F11 entry copied from Windows Boot Manager passed "WINDOWS" to the kernel.
[[ -z $cmdline ]] || "$src/scripts/config" --file "$out/.config" --enable CMDLINE_BOOL --set-str CMDLINE "$cmdline" --enable CMDLINE_OVERRIDE
# Each profile has its own module tree; a different uname -r keeps them from mixing.
"$src/scripts/config" --file "$out/.config" --set-str LOCALVERSION "$localversion"
for option in "${modules_list[@]}"; do "$src/scripts/config" --file "$out/.config" --module "$option"; done
make -C "$src" O="$out" ARCH=x86_64 CC=gcc HOSTCC=gcc olddefconfig
for option in 64BIT SERIAL_8250_CONSOLE BINFMT_ELF BLK_DEV_INITRD VT_CONSOLE POSIX_TIMERS EPOLL ACPI PID_NS NET_NS SYSVIPC IPC_NS SECCOMP_FILTER SECURITY_LANDLOCK X86_USER_SHADOW_STACK HARDENED_USERCOPY INIT_ON_ALLOC_DEFAULT_ON RANDOMIZE_BASE RANDOMIZE_KSTACK_OFFSET_DEFAULT STACKPROTECTOR_STRONG SLAB_FREELIST_RANDOM SLAB_FREELIST_HARDENED LIST_HARDENED FORTIFY_SOURCE "${required[@]}"; do
    grep -qx "CONFIG_${option}=y" "$out/.config" || { echo "ROJO: falta CONFIG_$option"; exit 1; }
done
grep -qx 'CONFIG_LSM="landlock"' "$out/.config" || { echo 'ROJO: Landlock no esta en la lista de LSM activos'; exit 1; }
if [[ $perfil == generico || $perfil == pc ]]; then
    ! grep -qx 'CONFIG_PSI_DEFAULT_DISABLED=y' "$out/.config" || { echo 'ROJO: PSI desactivado al arrancar'; exit 1; }
    [[ $cmdline != *psi=0* ]] || { echo 'ROJO: linea de comandos desactiva PSI'; exit 1; }
fi
targets=(bzImage); [[ $modules = yes ]] && targets+=(modules)
make -C "$src" O="$out" ARCH=x86_64 CC=gcc HOSTCC=gcc -j"$(nproc)" "${targets[@]}"
if [[ $modules = yes ]]; then
    rm -rf "$art/modulos"
    make -s -C "$src" O="$out" ARCH=x86_64 INSTALL_MOD_PATH="$art/modulos" INSTALL_MOD_STRIP=1 modules_install
    if [[ $perfil = pc ]]; then
        # W3a, the RTX 3060: nvidia-open from Arch's own source package, built against this very
        # kernel, the same version as the nvidia-utils that goes into the system image.
        mrel=$(make -s -C "$src" O="$out" ARCH=x86_64 kernelrelease)
        # A fresh package database, so the module and the userspace come from the same release.
        sudo pacman -Sy > /dev/null
        nv_ver=$(pacman -Si nvidia-utils | sed -n 's/^Version *: *\([0-9.]*\)-.*/\1/p')
        [[ -n $nv_ver ]] || { echo 'ROJO: no se pudo saber la version de nvidia-utils'; exit 1; }
        sudo pacman -Sw --noconfirm "nvidia-open-dkms=$nv_ver" > /dev/null
        nvpkg=$(ls /var/cache/pacman/pkg/nvidia-open-dkms-"$nv_ver"-*-x86_64.pkg.tar.zst | tail -1)
        nvsrc="$base/build/nvidia-open"
        rm -rf "$nvsrc" && mkdir -p "$nvsrc" && tar -C "$nvsrc" -xf "$nvpkg" usr/src
        make -C "$nvsrc/usr/src/nvidia-$nv_ver" -j"$(nproc)" IGNORE_PREEMPT_RT_PRESENCE=1 NV_EXCLUDE_BUILD_MODULES='' \
            SYSSRC="$src" SYSOUT="$out" CC=gcc modules > "$art/nvidia-build.log" 2>&1 ||
            { tail -20 "$art/nvidia-build.log"; echo "ROJO: nvidia-open $nv_ver no compila contra $mrel"; exit 1; }
        nvdest="$art/modulos/lib/modules/$mrel/kernel/drivers/video"
        mkdir -p "$nvdest"
        for m in nvidia nvidia-modeset nvidia-drm nvidia-uvm nvidia-peermem; do
            install -m 0644 "$nvsrc/usr/src/nvidia-$nv_ver/kernel-open/$m.ko" "$nvdest/$m.ko"
            strip --strip-debug "$nvdest/$m.ko"
            # Guardian: a module for another kernel would not load, or worse, would load wrong.
            [[ $(modinfo -F vermagic "$nvdest/$m.ko") = "$mrel "* ]] || { echo "ROJO: $m.ko no es para $mrel"; exit 1; }
        done
        depmod -b "$art/modulos" "$mrel"
        grep -q 'nvidia' "$art/modulos/lib/modules/$mrel/modules.alias" || { echo 'ROJO: depmod no registro los alias de NVIDIA'; exit 1; }
        # Guardian flag: functions returning without the return thunk weaken the SRSO (Inception)
        # mitigation. Count observed warning lines, not unique functions or full protection.
        naked=$(grep -c "'naked' return found in MITIGATION_RETHUNK build" "$art/nvidia-build.log" || true)
        printf 'nvidia-open %s para %s\nAMARILLO [SRSO]: %s lineas de aviso objtool "naked return" (MITIGATION_RETHUNK); no son un recuento de funciones unicas. Cobertura efectiva pendiente de comprobar con microcodigo, sysfs y pruebas en hardware real.\n' \
            "$nv_ver" "$mrel" "$naked" > "$art/nvidia-guardian.txt"
        echo "VERDE: nvidia-open $nv_ver compilado para $mrel ($(du -sh "$nvdest" | cut -f1))"
        echo "AMARILLO [SRSO]: $naked avisos observados; cobertura real pendiente (ver nvidia-guardian.txt)"
    fi
    tar -C "$art/modulos" -czf "$art/modulos.tar.gz" lib
    for m in "${modules_list[@]}"; do
        grep -qx "CONFIG_$m=m" "$out/.config" || { echo "ROJO: CONFIG_$m no quedo como modulo"; exit 1; }
    done
fi
cp "$out/arch/x86/boot/bzImage" "$art/bzImage-tiny64"
cp "$out/.config" "$art/tiny64.config"
cp "$user/guardianes.txt" "$art/guardianes.txt"
cp "$user/elf-produccion.tar.gz" "$art/elf-produccion.tar.gz"
make -s -C "$src" O="$out" ARCH=x86_64 kernelrelease > "$art/kernelrelease.txt"
sums=("$art/bzImage-tiny64" "$user/init" "$user/caja" "$user/ultra-raiz" "$user/ultra-red-contadores" "$user/ultra-recursos-contadores" "$user/minas" "$art/elf-produccion.tar.gz")
if [[ -n $cmdline ]]; then
    grep -qx 'CONFIG_CMDLINE_BOOL=y' "$out/.config" || { echo 'ROJO: falta la linea de comandos integrada'; exit 1; }
    grep -qx 'CONFIG_CMDLINE_OVERRIDE=y' "$out/.config" || { echo 'ROJO: el firmware podria anadir argumentos al kernel'; exit 1; }
    # Boot disk without GRUB, built with sfdisk, mkfs.fat, mtools, mkfs.erofs and mkfs.ext4:
    # nothing is mounted. GPT, with the init finding the Linux partitions by name:
    #   1 ULTRA-ESP        FAT32, the kernel as the firmware's default loader
    #   2 ULTRA-SISTEMA-A  EROFS, read-only: the modules of this exact kernel
    #   3 ULTRA-DATOS      ext4: what must survive a reboot (PC.configuración)
    command -v mkfs.erofs > /dev/null || { echo 'ROJO: falta mkfs.erofs; instalar con pacman -S erofs-utils'; exit 1; }
    release=$(cat "$art/kernelrelease.txt")
    img="$art/ultra-esp.img" esp_type=C12A7328-F81F-11D2-BA4B-00A0C93EC93B linux_type=0FC63DAF-8483-4772-8E79-3D69D8477DE4
    export MTOOLS_SKIP_CHECK=1
    # System A (Wayland stage W1): Arch's base system, installed with its own pacman into a
    # separate root, plus the modules of this kernel. Compatible with Arch, not a copy: the
    # packages come unchanged from Arch's repositories and their list is kept as evidence.
    # Docs, man pages, translations and development headers stay out: a boot needs none.
    sistema="$art/sistema-a" datos="$art/datos-semilla"
    sudo rm -rf "$sistema"
    rm -rf "$datos" "$art/sistema-a.erofs" "$art/datos.ext4"
    mkdir -p "$datos"
    sudo mkdir -p "$sistema/var/lib/pacman"
    # DownloadUser out: pacman's own user cannot enter /home/builder, where the root is built.
    sed -e '/^\[options\]/a NoExtract = usr/share/man/* usr/share/doc/* usr/share/info/* usr/share/locale/* usr/share/i18n/*\nNoExtract = usr/share/gtk-doc/* usr/share/help/* usr/include/*' \
        -e '/^DownloadUser/d' -e '/^#\[multilib\]/,+1s/^#//' /etc/pacman.conf > "$art/pacman-sistema.conf"
    # W1: the base. W2: the first Wayland window: cage (compositor), foot (terminal), seatd
    # (session), systemd only for its udevd (devices for libinput), and a font.
    arch_pkgs=(filesystem glibc bash coreutils grep sed less cage foot seatd systemd ttf-dejavu ttf-jetbrains-mono util-linux shadow
        hyprland=0.56.2-4 fuzzel waybar thunar mousepad dbus tzdata chrony swaybg librsvg
        nss alsa-lib libcups libxcomposite libxdamage libxrandr ca-certificates curl iproute2 dhcpcd
        ethtool linux-firmware-realtek papirus-icon-theme otf-font-awesome mako libnotify
        pipewire pipewire-audio pipewire-alsa pipewire-pulse wireplumber alsa-utils pavucontrol kmod
        steam umu-launcher lib32-libpulse lib32-alsa-plugins lib32-libxtst lib32-libxrandr gawk procps-ng vulkan-tools e2fsprogs
        mesa-utils vulkan-validation-layers xdg-desktop-portal-gtk gnome-keyring libsecret
        mangohud=0.8.4-1 lib32-mangohud=0.8.4-1
        # Rust para el usuario: rustup instala las versiones en el home persistente; gcc enlaza.
        rustup gcc git pkgconf
        # Capturas de pantalla.
        grim slurp wl-clipboard cliphist
        # Fondo vivo: Pillow genera el bucle en el home; rsvg-convert (librsvg) pasa el SVG a imagen.
        python-pillow)
    # W3a, pc profile only: NVIDIA's userspace and the GPU firmware, the same version as the
    # nvidia-open module built above. The generic image stays without it: the safety net.
    [[ $perfil = pc ]] && arch_pkgs+=("nvidia-utils=$nv_ver")
    if [[ $perfil = pc ]]; then arch_pkgs+=("lib32-nvidia-utils=$nv_ver");
    else arch_pkgs+=(vulkan-swrast lib32-vulkan-swrast lib32-mesa); fi
    sudo pacman --config "$art/pacman-sistema.conf" --root "$sistema" --dbpath "$sistema/var/lib/pacman" \
        -Sy --noconfirm "${arch_pkgs[@]}" > "$art/pacman-sistema.log" 2>&1 ||
        { tail -20 "$art/pacman-sistema.log"; echo 'ROJO: pacman no pudo instalar el sistema base de Arch'; exit 1; }
    # Reproductor del fondo vivo: awww (antes swww). Opcional: sin el, el fondo queda estatico.
    for motor in awww swww; do
        if sudo pacman --config "$art/pacman-sistema.conf" --root "$sistema" --dbpath "$sistema/var/lib/pacman" \
            -S --needed --noconfirm "$motor" >> "$art/pacman-sistema.log" 2>&1; then echo "VERDE [FONDO VIVO]: $motor"; break; fi
        echo "AMARILLO [FONDO VIVO]: $motor no esta en los repositorios"
    done
    sudo pacman --root "$sistema" --dbpath "$sistema/var/lib/pacman" -Q > "$art/sistema-paquetes.txt"
    bash "$lab/instalar-brave.sh" "$sistema" "$art" "$base"
    bash "$lab/instalar-heroic.sh" "$sistema" "$art" "$base"
    # Fixed session identity; render nodes only. KMS/input go through root seatd.
    sudo systemd-sysusers --root="$sistema"
    sudo chroot "$sistema" /usr/bin/groupadd -f -r render
    sudo chroot "$sistema" /usr/bin/groupadd -g 1000 ultra
    sudo chroot "$sistema" /usr/bin/useradd -u 1000 -g ultra -G render,audio -m -s /usr/bin/bash ultra
    sudo chroot "$sistema" /usr/bin/usermod -L ultra
    sudo install -d -m 0755 "$sistema/usr/local/libexec"
    for helper in ultra-sesion ultra-cliente ultra-menu ultra-hyprland ultra-preparar-nvidia ultra-informe-gpu ultra-medir-sesion ultra-medir-recursos ultra-preparar-red ultra-preparar-hora ultra-informe-red ultra-desktop-session ultra-preparar-juegos ultra-preparar-windows ultra-preparar-warframe ultra-preparar-bibliotecas ultra-bibliotecas.py ultra-iniciar-escritorio ultra-eventos-kernel ultra-preparar-limites ultra-crear-fondo; do
        sudo install -m 0755 "$lab/rootfs/bin/$helper" "$sistema/usr/local/libexec/$helper"
    done
    sudo install -m 0755 "$user/ultra-red-contadores" "$sistema/usr/local/libexec/ultra-red-contadores"
    sudo install -m 0755 "$user/ultra-recursos-contadores" "$sistema/usr/local/libexec/ultra-recursos-contadores"
    # These kernels have FW_LOADER_COMPRESS disabled. Ship plain Realtek NIC
    # firmware rather than merely accepting an unreadable compressed package.
    for firmware in "$sistema"/usr/lib/firmware/rtl_nic/*.zst; do
        [[ -f $firmware ]] || continue
        sudo zstd -dq -f -o "${firmware%.zst}" -- "$firmware"
    done
    [[ -s $sistema/usr/lib/firmware/rtl_nic/rtl8168h-2.fw ]] ||
        { echo 'ROJO: falta firmware Ethernet RTL8168h'; exit 1; }
    sudo install -d -m 0755 "$sistema/etc/ultra"
    sudo install -m 0644 "$lab/rootfs/desktop/"* "$sistema/etc/ultra/"
    sudo pacman -S --noconfirm --needed librsvg xorg-xcursorgen > "$art/tema-herramientas.log" 2>&1
    sudo python "$lab/rootfs/crear-tema.py" "$sistema"
    # Tema Neon: Rajdhani y Chakra Petch (OFL) viajan con el proyecto; JetBrains Mono viene de Arch.
    sudo install -d -m 0755 "$sistema/usr/share/fonts/ultra-neon" "$sistema/usr/share/licenses/ultra-neon"
    sudo install -m 0644 "$lab/rootfs/fuentes/"*.ttf "$sistema/usr/share/fonts/ultra-neon/"
    sudo install -m 0644 "$lab/rootfs/fuentes/"OFL-*.txt "$sistema/usr/share/licenses/ultra-neon/"
    sudo chroot "$sistema" /usr/bin/fc-cache -f > /dev/null
    sudo install -m 0644 "$art/sistema-paquetes.txt" "$sistema/etc/ultra/sistema-paquetes.txt"
    sudo install -d -m 0755 "$sistema/usr/local/bin" "$sistema/usr/share/applications"
    sudo install -m 0755 "$lab/rootfs/bin/ultra-brave" "$sistema/usr/local/bin/ultra-brave"
    sudo install -m 0755 "$lab/rootfs/bin/ultra-ambiente" "$sistema/usr/local/bin/ultra-ambiente"
    for helper in ultra-audio ultra-juegos ultra-bibliotecas ultra-steam-warframe ultra-graficos ultra-windows ultra-hora ultra-pantallas ultra-discos ultra-hud ultra-juegos-vivo ultra-gog-cyberpunk ultra-diagnostico-juego ultra-equipo ultra-tema ultra-aplicaciones ultra-espacio ultra-personalizar ultra-incidente ultra-captura ultra-portapapeles ultra-modo-juego ultra-fondo-vivo; do sudo install -m 0755 "$lab/rootfs/bin/$helper" "$sistema/usr/local/bin/$helper"; done
    sudo ln -sfn /usr/share/zoneinfo/America/Lima "$sistema/etc/localtime"
    printf 'America/Lima\n' | sudo tee "$sistema/etc/timezone" > /dev/null
    sudo install -d -m 0755 "$sistema/usr/lib/locale"
    sudo localedef --prefix="$sistema" -i es_PE -f UTF-8 es_PE.UTF-8
    sudo chroot "$sistema" /usr/bin/env LC_ALL=es_PE.UTF-8 /usr/bin/locale charmap | grep -qx UTF-8
    sudo install -d -m 0755 "$sistema/etc/wireplumber/wireplumber.conf.d"
    sudo install -m 0644 "$lab/rootfs/desktop/wireplumber-usb.conf" "$sistema/etc/wireplumber/wireplumber.conf.d/51-ultra-usb.conf"
    sudo install -m 0644 "$lab/rootfs/desktop/"*.desktop "$sistema/usr/share/applications/"
    for config in foot fuzzel gtk-3.0; do sudo install -d -m 0755 "$sistema/home/ultra/.config/$config"; done
    sudo install -m 0644 "$lab/rootfs/desktop/foot.ini" "$sistema/home/ultra/.config/foot/foot.ini"
    sudo install -m 0644 "$lab/rootfs/desktop/fuzzel.ini" "$sistema/home/ultra/.config/fuzzel/fuzzel.ini"
    sudo install -m 0644 "$lab/rootfs/desktop/gtk.css" "$sistema/home/ultra/.config/gtk-3.0/gtk.css"
    sudo install -m 0644 "$lab/rootfs/desktop/gtk-settings.ini" "$sistema/home/ultra/.config/gtk-3.0/settings.ini"
    sudo chown -R 1000:1000 "$sistema/home/ultra/.config"
    sudo chroot "$sistema" /usr/bin/dbus-uuidgen --ensure=/etc/machine-id
    # pacman's hook leaves udev's hardware database almost empty outside a booted system.
    if [[ -x $sistema/usr/bin/systemd-hwdb ]]; then sudo systemd-hwdb --root="$sistema" --usr update; fi
    sudo mkdir -p "$sistema/usr/lib/modules"
    sudo cp -a "$art/modulos/lib/modules/$release" "$sistema/usr/lib/modules/"
    sudo rm -f "$sistema/usr/lib/modules/$release/build" "$sistema/usr/lib/modules/$release/source"
    printf 'ULTRA-SISTEMA-A\nkernel=%s\nbase=arch\npaquetes=%s\n' "$release" "$(wc -l < "$art/sistema-paquetes.txt")" |
        sudo tee "$sistema/ULTRA-SISTEMA" > /dev/null
    # Inside Arch, show where you are: root's login shell reads this after /etc/profile.
    printf '%s\n' '# ULTRA LinuX: Arch base system mounted at /arch by the init (Wayland stage W1).' \
        "PS1='\\[\\033[1;35m\\]arch\\[\\033[0m\\]@\\[\\033[1;36m\\]ultra\\[\\033[0m\\]:\\w# '" \
        'echo "Dentro del sistema Arch de ULTRA LinuX. exit: volver a la shell de ULTRA."' |
        sudo tee "$sistema/root/.bash_profile" > /dev/null
    # Guardian: the dynamic programs keep the CET marks the shadow stack needs.
    for f in usr/lib/libc.so.6 usr/bin/bash usr/bin/ls; do
        readelf -n "$sistema/$f" | grep -q 'x86 feature: IBT, SHSTK' || { echo "ROJO: $f sin marca CET"; exit 1; }
    done
    sudo mkfs.erofs --quiet -zlzma,level=6 -T 0 "$art/sistema-a.erofs" "$sistema"
    sudo chown "$(id -u):$(id -g)" "$art/sistema-a.erofs"
    echo "VERDE: sistema A con base Arch: ${#arch_pkgs[@]} pedidos, $(wc -l < "$art/sistema-paquetes.txt") instalados, EROFS lzma $(( $(stat -c %s "$art/sistema-a.erofs") / 1048576 )) MiB"
    rm -f "$img"
    # Sized to the system just built: its EROFS plus 64 MiB of margin. GPT, ESP 64 MiB, data 64 MiB.
    sys_mib=$(( $(stat -c %s "$art/sistema-a.erofs") / 1048576 + 64 ))
    truncate -s $(( 1 + 64 + sys_mib + 64 + 1 ))M "$img"
    printf 'label: gpt\nstart=2048, size=64MiB, type=%s, name="ULTRA-ESP"\nsize=%sMiB, type=%s, name="ULTRA-SISTEMA-A"\nsize=64MiB, type=%s, name="ULTRA-DATOS"\n' \
        "$esp_type" "$sys_mib" "$linux_type" "$linux_type" | sfdisk -q "$img"
    # part NAME 1 -> first sector, part NAME 2 -> size in sectors.
    part() { sfdisk --dump "$img" | sed -n "s/.*start= *\([0-9]*\), size= *\([0-9]*\).*name=\"$1\".*/\\$2/p"; }
    for name in ULTRA-ESP ULTRA-SISTEMA-A ULTRA-DATOS; do
        [[ -n $(part "$name" 1) ]] || { echo "ROJO: la particion $name no aparece en la tabla GPT"; exit 1; }
    done
    offset=$(( $(part ULTRA-ESP 1) * 512 )) sectors=$(part ULTRA-ESP 2)
    mkfs.fat -F 32 -s 1 -n ULTRA-ESP --offset=$(part ULTRA-ESP 1) "$img" $((sectors / 2)) > /dev/null
    mmd -i "$img@@$offset" ::/EFI ::/EFI/BOOT
    mcopy -i "$img@@$offset" "$art/bzImage-tiny64" ::/EFI/BOOT/BOOTX64.EFI
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
    bmo_mib=$(( sys_mib + 20 + 64 + 32 ))
    gen_erofs="$base/artifacts/generico/sistema-a.erofs"
    [[ $perfil = pc && -f $gen_erofs ]] && bmo_mib=$(( bmo_mib + $(stat -c %s "$gen_erofs") / 1048576 + 40 ))
    truncate -s $(( 1 + 64 + bmo_mib + 16 + 1 ))M "$bmo"
    printf 'label: gpt\nstart=2048, size=64MiB, type=%s, name="EFI system partition"\nsize=%sMiB, type=%s, name="Basic data partition"\nsize=16MiB, type=%s, name="Basic data partition"\n' \
        "$esp_type" "$bmo_mib" "$basic_type" "$basic_type" | sfdisk -q "$bmo"
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
    # Each kernel brings its own system image, named after uname -r, so generico and pc
    # can live side by side on A:\ with one shared datos.ext4.
    mcopy -i "$bmo@@$a" "$art/sistema-a.erofs" "::/ULTRA/sistema-$release.erofs"
    mcopy -i "$bmo@@$a" "$art/datos.ext4" ::/ULTRA/datos.ext4
    [[ $(mtype -i "$bmo@@$a" "::/ULTRA/sistema-$release.erofs" | sha256sum | cut -d' ' -f1) = $(sha256sum < "$art/sistema-a.erofs" | cut -d' ' -f1) ]] ||
        { echo 'ROJO: sistema-a.erofs dentro de BMO no coincide'; exit 1; }
    mlabel -i "$bmo@@$a" -s :: | grep -q 'BMO' || { echo 'ROJO: la particion de prueba no se llama BMO'; exit 1; }
    echo "VERDE: disco de prueba $(basename "$bmo"): EFI de Windows simulada, FAT32 BMO con ULTRA en archivos y particion vacia"
    sums+=("$img" "$bmo" "$art/sistema-a.erofs" "$art/datos.ext4")
    # The PC's own version, end to end: both kernels on one disk. The firmware starts the
    # generic one by default (EFI\BOOT on the simulated Windows partition); the tuned one
    # waits as BMO\EFI\ULTRA\ULTRA-PC.EFI, for the entry the generic init creates itself.
    gen="$base/artifacts/generico"
    if [[ $perfil = pc && -f $gen/bzImage-tiny64 && -f $gen/sistema-a.erofs && -f $gen/kernelrelease.txt ]]; then
        doble="$art/disco-bmo-doble.img" grel=$(cat "$gen/kernelrelease.txt")
        cp "$bmo" "$doble"
        mcopy -o -i "$doble@@$win" "$gen/bzImage-tiny64" ::/EFI/BOOT/BOOTX64.EFI
        mcopy -o -i "$doble@@$a" "$gen/bzImage-tiny64" ::/EFI/ULTRA/ULTRA.EFI
        mcopy -i "$doble@@$a" "$art/bzImage-tiny64" ::/EFI/ULTRA/ULTRA-PC.EFI
        mcopy -i "$doble@@$a" "$gen/sistema-a.erofs" "::/ULTRA/sistema-$grel.erofs"
        [[ $(mtype -i "$doble@@$a" ::/EFI/ULTRA/ULTRA-PC.EFI | sha256sum | cut -d' ' -f1) = $(sha256sum < "$art/bzImage-tiny64" | cut -d' ' -f1) ]] ||
            { echo 'ROJO: ULTRA-PC.EFI del disco doble no es el kernel pc recien compilado'; exit 1; }
        echo "VERDE: disco doble $(basename "$doble"): generico ($grel) por defecto y pc en BMO\\EFI\\ULTRA\\ULTRA-PC.EFI"
        sums+=("$doble")
    elif [[ $perfil = pc ]]; then
        echo 'ROJO: falta el perfil generico compilado; compilarlo antes que pc para el disco doble'; exit 1
    fi
fi
sha256sum "${sums[@]}" > "$art/SHA256SUMS"
printf 'VERDE: kernel %s e init C compilados. Arranque QEMU todavia pendiente.\n' "$perfil"
touch "$art/build-complete"
