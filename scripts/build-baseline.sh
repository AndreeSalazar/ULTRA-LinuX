#!/bin/bash
# SPDX-License-Identifier: GPL-2.0-only
set -euo pipefail
# Profiles: produccion is what gets installed; vigia adds the kernel's own
# detectors (KASAN, UBSAN, KFENCE, lockdep) so the kernel reports its mines.
perfil=${1:-produccion}
case $perfil in
    produccion) dir=tiny64-baseline; extra=(KFENCE); required=(KFENCE) ;;
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
for file in "$lab/init.c" "$lab/caja/caja.c" "$lab/guardian/ultra_guard.c"; do
    ultra_revisar "$file" || { echo "ROJO: $file no pasa la revision de guardianes"; exit 1; }
done
# Static userspace built separately from the unmodified kernel sources.
gcc "${ULTRA_WARN[@]}" "${ULTRA_PROD[@]}" -static-pie "$lab/init.c" "$lab/guardian/ultra_guard.c" -o "$user/init"
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
    for d in /dev /proc /sys /tmp /run /root /etc /bin /sbin; do echo "dir $d 0755 0 0"; done
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
    make -C "$src" O="$out" ARCH=x86_64 CC=gcc HOSTCC=gcc tinyconfig
fi
for option in 64BIT PRINTK TTY VT VT_CONSOLE VGA_CONSOLE FONT_8x16 INPUT INPUT_KEYBOARD KEYBOARD_ATKBD SERIO SERIO_I8042 SERIAL_8250 SERIAL_8250_CONSOLE UNIX98_PTYS POSIX_TIMERS EPOLL SIGNALFD TIMERFD EVENTFD FILE_LOCKING PROC_SYSCTL PCI ACPI NET SMP RELOCATABLE RANDOMIZE_KSTACK_OFFSET NAMESPACES UTS_NS PID_NS NET_NS SECCOMP SECCOMP_FILTER SECURITY SECURITY_LANDLOCK X86_USER_SHADOW_STACK HARDENED_USERCOPY INIT_ON_ALLOC_DEFAULT_ON INIT_STACK_ALL_ZERO STACKPROTECTOR STACKPROTECTOR_STRONG RANDOMIZE_BASE RANDOMIZE_KSTACK_OFFSET_DEFAULT SLAB_FREELIST_RANDOM SLAB_FREELIST_HARDENED LIST_HARDENED BUG_ON_DATA_CORRUPTION SCHED_STACK_END_CHECK FORTIFY_SOURCE BINFMT_ELF BLK_DEV_INITRD DEVTMPFS DEVTMPFS_MOUNT PROC_FS SYSFS SHMEM TMPFS FUTEX MULTIUSER "${extra[@]}"; do
    "$src/scripts/config" --file "$out/.config" --enable "$option"
done
"$src/scripts/config" --file "$out/.config" --set-str INITRAMFS_SOURCE "$out/initramfs.list" --set-str LSM landlock --enable INITRAMFS_COMPRESSION_NONE --disable MODULES --disable SLUB_TINY --disable DEBUG_INFO --disable DEBUG_INFO_BTF --disable WERROR
make -C "$src" O="$out" ARCH=x86_64 CC=gcc HOSTCC=gcc olddefconfig
for option in 64BIT SERIAL_8250_CONSOLE BINFMT_ELF BLK_DEV_INITRD VT_CONSOLE POSIX_TIMERS EPOLL ACPI PID_NS NET_NS SECCOMP_FILTER SECURITY_LANDLOCK X86_USER_SHADOW_STACK HARDENED_USERCOPY INIT_ON_ALLOC_DEFAULT_ON RANDOMIZE_BASE RANDOMIZE_KSTACK_OFFSET_DEFAULT STACKPROTECTOR_STRONG SLAB_FREELIST_RANDOM SLAB_FREELIST_HARDENED LIST_HARDENED FORTIFY_SOURCE "${required[@]}"; do
    grep -qx "CONFIG_${option}=y" "$out/.config" || { echo "ROJO: falta CONFIG_$option"; exit 1; }
done
grep -qx 'CONFIG_LSM="landlock"' "$out/.config" || { echo 'ROJO: Landlock no esta en la lista de LSM activos'; exit 1; }
make -C "$src" O="$out" ARCH=x86_64 CC=gcc HOSTCC=gcc -j2 bzImage
cp "$out/arch/x86/boot/bzImage" "$art/bzImage-tiny64"
cp "$out/.config" "$art/tiny64.config"
cp "$user/guardianes.txt" "$art/guardianes.txt"
make -s -C "$src" O="$out" ARCH=x86_64 kernelrelease > "$art/kernelrelease.txt"
sha256sum "$art/bzImage-tiny64" "$user/init" "$user/caja" > "$art/SHA256SUMS"
printf 'VERDE: kernel %s e init C compilados. Arranque QEMU todavia pendiente.\n' "$perfil"
touch "$art/build-complete"
