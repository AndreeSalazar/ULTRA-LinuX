#!/bin/bash
# SPDX-License-Identifier: GPL-2.0-only
set -euo pipefail
ulimit -c 0
lab=$(cd -- "$(dirname -- "$0")" && pwd)
base="$HOME/linux-reconstruido"
out="$base/build/syscalls"
mkdir -p "$out"
exec > >(tee "$out/informe.txt") 2>&1
source "$lab/guardianes.sh"
printf 'KERNEL-TALLER: '; uname -r
gcc --version | head -1
for tool in gcc python strace whatis unshare; do command -v "$tool" >/dev/null; done
! readelf -l /usr/bin/busybox | grep -q INTERP || { echo 'ROJO: BusyBox dinamico'; exit 1; }
strace -f -qq -o "$out/busybox.strace" /usr/bin/busybox sh -c \
    'busybox cat /etc/os-release; busybox echo control; busybox ls /tmp; busybox wc -c /etc/os-release; busybox uname -n; busybox sleep 0.01; busybox id' > "$out/carga.txt"
python "$lab/catalogo-syscalls.py" --kernel "$base/kernel" \
    --politica "$lab/caja/syscalls-politica.h" \
    --system-map "$base/build/x86_64-generico/System.map" --trace "$out/busybox.strace" \
    --out "$out" --header "$lab/caja/syscalls-x86_64.h"
for src in caja probar-seccomp probar-stdio probar-guardian-pid1 probar-identidad; do
    ultra_revisar "$lab/caja/$src.c"
    gcc "${ULTRA_WARN[@]}" "${ULTRA_PROD[@]}" -static-pie \
        "$lab/caja/$src.c" "$lab/guardian/ultra_guard.c" -o "$out/$src"
done
"$out/caja" --dump-filter > "$out/filtro.bpf"
python "$lab/verificar-seccomp.py" --bpf "$out/filtro.bpf" --catalogo "$out/catalogo.csv" | tee "$out/formal.txt"
sha256sum "$out/caja" "$out/filtro.bpf" "$lab/caja/seccomp-politica.h" "$lab/caja/syscalls-politica.h" \
    "$lab/caja/identidad-politica.h" "$lab/caja/caja.c" "$lab/caja/probar-identidad.c" > "$out/SHA256SUMS"
timeout --kill-after=2s 30s "$out/probar-seccomp"
timeout --kill-after=2s 10s sudo "$out/probar-identidad"
for mode in abort trap; do
    rc=0
    timeout --kill-after=1s 3s sudo unshare --pid --fork --kill-child=KILL "$out/probar-guardian-pid1" "$mode" > "$out/pid1-$mode.txt" 2>&1 || rc=$?
    want=134; [[ $mode = trap ]] && want=132
    [[ $rc -eq $want ]] && grep -q '^ROJO' "$out/pid1-$mode.txt" ||
        { echo "ROJO: guardia PID 1 $mode no termino como se esperaba (rc=$rc)"; exit 1; }
done
echo 'PID1-OK: rechazo explicito y trampa terminan sin bucle'
# Test the actual privileged box in a private outer namespace and disposable root.
root=$(mktemp -d "$out/rootfs.XXXXXX")
chmod 755 "$root"
mkdir -p "$root"/{bin,etc,proc,tmp,dev,datos}
chmod 1777 "$root/tmp"
cp /usr/bin/busybox "$root/bin/busybox"
cp "$out/caja" "$root/bin/caja"
cp "$out/probar-stdio" "$root/bin/probar-stdio"
cp "$lab/probar-caja" "$root/bin/probar-caja"
chmod 755 "$root/bin/probar-caja"
cp "$lab/os-release" "$root/etc/os-release"
printf 'CONTROL-LECTURA\n' > "$root/datos/secreto"
printf 'NO-TRUNCAR\n' > "$root/etc/sentinel"
chmod 666 "$root/etc/sentinel"
ln -s /datos/secreto "$root/tmp/caja-enlace"
cp /usr/bin/busybox "$root/tmp/caja-noexec"
chmod 755 "$root/tmp/caja-noexec"
while IFS= read -r applet; do
    case $applet in busybox|caja|probar-caja|probar-stdio) continue;; esac
    ln -s busybox "$root/bin/$applet"
done < <(/usr/bin/busybox --list)
timeout --kill-after=2s 45s sudo unshare --mount --pid --fork --kill-child=KILL --net --ipc --uts --mount-proc \
    bash -c 'set -eu; mount --make-rprivate /; mount --bind "$1" "$1"; mount -t proc proc "$1/proc"; chroot "$1" /bin/busybox env -i PATH=/bin HOME=/tmp /bin/probar-caja --fixtures; chroot "$1" /bin/busybox env -i PATH=/bin HOME=/tmp /bin/probar-stdio' _ "$root"
[[ $(cat "$root/etc/sentinel") = NO-TRUNCAR ]] || { echo 'ROJO: archivo de control modificado'; exit 1; }
echo 'DATOS-OK: archivo de control conservado; lectura protegida y ejecucion en datos rechazadas'
echo 'AUDITORIA-SYSCALLS-OK: catalogo, BPF, kernel y caja real verificados en taller'
