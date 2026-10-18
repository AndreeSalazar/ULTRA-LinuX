#!/bin/bash
# SPDX-License-Identifier: GPL-2.0-only
set -euo pipefail
base="$HOME/linux-reconstruido"
src="$base/kernel"
out="$base/build/tiny64-baseline"
# Userspace lives outside O=: Kbuild needs $out/init as a directory for init/main.c.
user="$base/build/init-c"
lab=$(cd -- "$(dirname -- "$0")" && pwd)
mkdir -p "$out" "$user" "$base/artifacts"
[[ -d "$out/init" || ! -e "$out/init" ]] || rm -f "$out/init"
rm -f "$base/artifacts/build-complete"
exec > >(tee "$out/build.log") 2>&1
trap 'echo "ROJO: build fallo en linea $LINENO"' ERR
expected=32c52ec87d6245ee2d12ed5dd509e40062939b50
[[ $(git -C "$src" rev-parse HEAD) = "$expected" ]] || { echo 'ROJO: commit inesperado'; exit 1; }
[[ -z $(git -C "$src" ls-files --deleted) ]] || { echo 'ROJO: archivos fuente ausentes'; exit 1; }
gcc --version | head -n 1
# Guardians: warnings that hide bugs are errors, the analyzer reads every path,
# and undefined behaviour traps at the faulty instruction instead of continuing.
warn=(-std=c17 -Wall -Wextra -Werror -Wconversion -Wsign-conversion -Wshadow -Wformat=2
      -Wvla -Wnull-dereference -Wimplicit-fallthrough -Wstrict-prototypes -fanalyzer)
trap_ub=(-fsanitize=undefined -fsanitize-trap=all -fstack-protector-strong
         -D_FORTIFY_SOURCE=3 -ftrivial-auto-var-init=zero -fno-strict-aliasing)
# Static userspace built separately from the unmodified kernel sources.
gcc "${warn[@]}" "${trap_ub[@]}" -O2 -static "$lab/init.c" -o "$user/init"
printf '%s
' "$(gcc --version | head -n 1)" "${warn[*]}" "${trap_ub[*]}" -O2 -static > "$user/guardianes.txt"
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
    for applet in $("$busybox" --list); do
        case $applet in busybox|init|linuxrc) continue;; esac
        echo "slink /bin/$applet busybox 0777 0 0"
    done
    for f in "$lab"/rootfs/etc/*; do echo "file /etc/${f##*/} $f 0644 0 0"; done
} > "$out/initramfs.list"
if [[ ! -f "$out/.config" ]]; then
    make -C "$src" O="$out" ARCH=x86_64 CC=gcc HOSTCC=gcc tinyconfig
fi
for option in 64BIT PRINTK TTY VT VT_CONSOLE VGA_CONSOLE FONT_8x16 INPUT INPUT_KEYBOARD KEYBOARD_ATKBD SERIO SERIO_I8042 SERIAL_8250 SERIAL_8250_CONSOLE UNIX98_PTYS POSIX_TIMERS EPOLL SIGNALFD TIMERFD EVENTFD FILE_LOCKING PROC_SYSCTL PCI ACPI BINFMT_ELF BLK_DEV_INITRD DEVTMPFS DEVTMPFS_MOUNT PROC_FS SYSFS SHMEM TMPFS FUTEX MULTIUSER; do
    "$src/scripts/config" --file "$out/.config" --enable "$option"
done
"$src/scripts/config" --file "$out/.config" --set-str INITRAMFS_SOURCE "$out/initramfs.list" --enable INITRAMFS_COMPRESSION_NONE --disable MODULES --disable DEBUG_INFO --disable DEBUG_INFO_BTF --disable WERROR
make -C "$src" O="$out" ARCH=x86_64 CC=gcc HOSTCC=gcc olddefconfig
for option in 64BIT SERIAL_8250_CONSOLE BINFMT_ELF BLK_DEV_INITRD VT_CONSOLE POSIX_TIMERS EPOLL ACPI; do
    grep -qx "CONFIG_${option}=y" "$out/.config" || { echo "ROJO: falta CONFIG_$option"; exit 1; }
done
make -C "$src" O="$out" ARCH=x86_64 CC=gcc HOSTCC=gcc -j2 bzImage
cp "$out/arch/x86/boot/bzImage" "$base/artifacts/bzImage-tiny64"
cp "$out/.config" "$base/artifacts/tiny64.config"
cp "$user/guardianes.txt" "$base/artifacts/guardianes.txt"
make -s -C "$src" O="$out" ARCH=x86_64 kernelrelease > "$base/artifacts/kernelrelease.txt"
sha256sum "$base/artifacts/bzImage-tiny64" "$user/init" > "$base/artifacts/SHA256SUMS"
printf 'VERDE: kernel e init C compilados. Arranque QEMU todavia pendiente.\n'
touch "$base/artifacts/build-complete"
