#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-2.0-only
# Repack a fresh virtual FAT32 disk sized for today's EROFS; no block devices.
set -euo pipefail
out=${1:?salida}; kernel=${2:?kernel}; system=${3:?erofs}; data=${4:?datos}; release=${5:?release}
root=$(cd -- "$(dirname -- "$0")/.." && pwd)
[[ $out == "$root/build/"*/disco-bmo.img && ! -e $out && ! -L $out ]]
[[ $(realpath -m -- "$out") == "$out" && $release =~ ^[0-9A-Za-z.+_-]+$ ]]
for file in "$kernel" "$system" "$data"; do [[ -f $file && ! -L $file ]]; done
[[ $(stat -c %s "$system") -lt 4294967296 ]]
basic=EBD0A0A2-B9E5-4433-87C0-68B6B72699C7
esp=C12A7328-F81F-11D2-BA4B-00A0C93EC93B
bytes=$(( $(stat -c %s "$system") + $(stat -c %s "$kernel") + $(stat -c %s "$data") ))
mib=$(( (bytes+1048575)/1048576+256 ))
truncate -s $((1+64+mib+16+1))M "$out"
printf 'label: gpt\nstart=2048, size=64MiB, type=%s, name="EFI system partition"\nsize=%sMiB, type=%s, name="Basic data partition"\nsize=16MiB, type=%s, name="Basic data partition"\n' "$esp" "$mib" "$basic" "$basic" | sfdisk -q "$out"
read -r win win_sectors bmo bmo_sectors < <(python3 - "$out" <<'PY'
import json,subprocess,sys
p=json.loads(subprocess.check_output(['sfdisk','--json',sys.argv[1]]))['partitiontable']['partitions']
print(p[0]['start'],p[0]['size'],p[1]['start'],p[1]['size'])
PY
)
mkfs.fat -F 32 -s 1 --offset="$win" "$out" $((win_sectors/2)) >/dev/null 2>&1
mkfs.fat -F 32 -s 1 -n BMO --offset="$bmo" "$out" $((bmo_sectors/2)) >/dev/null 2>&1
win=$((win*512)); bmo=$((bmo*512))
printf 'NO NAME    ' | dd of="$out" bs=1 seek=$((bmo+71)) conv=notrunc status=none
export MTOOLS_SKIP_CHECK=1
mmd -i "$out@@$win" ::/EFI ::/EFI/BOOT ::/EFI/Microsoft ::/EFI/Microsoft/Boot
mcopy -i "$out@@$win" "$kernel" ::/EFI/BOOT/BOOTX64.EFI
fake=$(mktemp);trap 'rm -f -- "$fake"' EXIT
printf 'no es Windows: marca de prueba\n' > "$fake"
mcopy -i "$out@@$win" "$fake" ::/EFI/Microsoft/Boot/bootmgfw.efi
mmd -i "$out@@$bmo" ::/EFI ::/EFI/ULTRA ::/ULTRA
mcopy -i "$out@@$bmo" "$kernel" ::/EFI/ULTRA/ULTRA.EFI
mcopy -i "$out@@$bmo" "$system" "::/ULTRA/sistema-$release.erofs"
mcopy -i "$out@@$bmo" "$data" ::/ULTRA/datos.ext4
[[ $(mtype -i "$out@@$bmo" "::/ULTRA/sistema-$release.erofs" | sha256sum | cut -d' ' -f1) == $(sha256sum "$system" | cut -d' ' -f1) ]]
echo "BMO-VIRTUAL-PREPARADO: FAT32 ${mib} MiB, margen 256 MiB"
