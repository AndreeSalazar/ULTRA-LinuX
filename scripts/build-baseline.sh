#!/bin/bash
# SPDX-License-Identifier: GPL-2.0-only
set -euo pipefail
base="$HOME/linux-reconstruido"
src="$base/kernel"
out="$base/build/tiny64-baseline"
lab=$(cd -- "$(dirname -- "$0")" && pwd)
mkdir -p "$out" "$base/artifacts"
rm -f "$base/artifacts/build-complete"
exec > >(tee "$out/build.log") 2>&1
trap 'echo "ROJO: build fallo en linea $LINENO"' ERR
expected=32c52ec87d6245ee2d12ed5dd509e40062939b50
[[ $(git -C "$src" rev-parse HEAD) = "$expected" ]] || { echo 'ROJO: commit inesperado'; exit 1; }
[[ -z $(git -C "$src" ls-files --deleted) ]] || { echo 'ROJO: archivos fuente ausentes'; exit 1; }
gcc --version | head -n 1
# Static userspace built separately from the unmodified kernel sources.
gcc -std=c17 -static -Os -Wall -Wextra -Werror "$lab/init.c" -o "$out/init"
cat > "$out/initramfs.list" <<CPIO
dir /dev 0755 0 0
dir /proc 0755 0 0
dir /sys 0755 0 0
nod /dev/console 0600 0 0 c 5 1
nod /dev/ttyS0 0600 0 0 c 4 64
file /init $out/init 0755 0 0
CPIO
if [[ ! -f "$out/.config" ]]; then
    make -C "$src" O="$out" ARCH=x86_64 CC=gcc HOSTCC=gcc tinyconfig
fi
for option in 64BIT PRINTK TTY VT VT_CONSOLE VGA_CONSOLE FONT_8x16 INPUT INPUT_KEYBOARD KEYBOARD_ATKBD SERIO SERIO_I8042 SERIAL_8250 SERIAL_8250_CONSOLE BINFMT_ELF BLK_DEV_INITRD DEVTMPFS DEVTMPFS_MOUNT PROC_FS SYSFS SHMEM TMPFS FUTEX MULTIUSER; do
    "$src/scripts/config" --file "$out/.config" --enable "$option"
done
"$src/scripts/config" --file "$out/.config" --set-str INITRAMFS_SOURCE "$out/initramfs.list" --enable INITRAMFS_COMPRESSION_NONE --disable MODULES --disable DEBUG_INFO --disable DEBUG_INFO_BTF --disable WERROR
make -C "$src" O="$out" ARCH=x86_64 CC=gcc HOSTCC=gcc olddefconfig
for option in 64BIT SERIAL_8250_CONSOLE BINFMT_ELF BLK_DEV_INITRD VT_CONSOLE; do
    grep -qx "CONFIG_${option}=y" "$out/.config" || { echo "ROJO: falta CONFIG_$option"; exit 1; }
done
make -C "$src" O="$out" ARCH=x86_64 CC=gcc HOSTCC=gcc -j2 bzImage
cp "$out/arch/x86/boot/bzImage" "$base/artifacts/bzImage-tiny64"
cp "$out/.config" "$base/artifacts/tiny64.config"
make -s -C "$src" O="$out" ARCH=x86_64 kernelrelease > "$base/artifacts/kernelrelease.txt"
sha256sum "$base/artifacts/bzImage-tiny64" "$out/init" > "$base/artifacts/SHA256SUMS"
printf 'VERDE: kernel e init C compilados. Arranque QEMU todavia pendiente.\n'
touch "$base/artifacts/build-complete"
