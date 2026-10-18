#!/bin/bash
# SPDX-License-Identifier: GPL-2.0-only
set -euo pipefail
[[ $(id -u) = 0 ]] || { echo 'ROJO: ejecutar como root dentro de Debian WSL.' >&2; exit 1; }
base=$(cd -- "$(dirname -- "$0")/.." && pwd)
export DEBIAN_FRONTEND=noninteractive
apt-get update
apt-get install -y build-essential binutils bc bison flex libssl-dev libelf-dev libncurses-dev dwarves pkg-config python3 perl git rsync cpio xz-utils zstd file curl ca-certificates gdb qemu-system-x86 busybox-static kmod ccache
workspace=/root/linux-reconstruido
mkdir -p "$workspace"
if [[ ! -e "$workspace/kernel" ]]; then
    bash "$base/scripts/preparar-linux.sh" "$workspace/kernel"
else
    expected=$(git --git-dir="$base/upstream/linux.git" rev-parse HEAD)
    actual=$(git -C "$workspace/kernel" rev-parse HEAD)
    [[ $expected = "$actual" ]] || { echo 'ROJO: la copia existente usa otro commit; revisar sin sobrescribir.' >&2; exit 1; }
fi
mkdir -p "$workspace/build/baseline" "$base/evidencias"
{
    uname -a
    gcc --version | head -n 1
    ld --version | head -n 1
    make --version | head -n 1
    python3 --version
    git -C "$workspace/kernel" rev-parse HEAD
    dpkg-query -W -f='${Package}\t${Version}\n' build-essential gcc binutils bc bison flex libssl-dev libelf-dev dwarves qemu-system-x86 busybox-static
} > "$base/evidencias/herramientas-linux.txt"
make -C "$workspace/kernel" O="$workspace/build/baseline" CC=gcc HOSTCC=gcc defconfig
make -s -C "$workspace/kernel" O="$workspace/build/baseline" CC=gcc HOSTCC=gcc kernelversion > "$base/evidencias/kernelversion.txt"
cp "$workspace/build/baseline/.config" "$base/evidencias/baseline.config"
echo 'VERDE: herramientas GCC, fuentes y configuración base listas. Kernel completo y arranque todavía pendientes.'
