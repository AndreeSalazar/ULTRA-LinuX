#!/usr/bin/bash
# SPDX-License-Identifier: GPL-2.0-only
# Run in the QEMU workshop. Only new regular image files; no physical disks.
set -euo pipefail
base=$(realpath "$HOME/linux-reconstruido")
output=${1:-$base/juegos}
[[ $output == "$base/juegos" || $output == "$base/juegos-prueba" || $output == "$base/juegos-grafica" ]] || exit 2
[[ ! -e $output && ! -L $output ]] || { echo 'ROJO: el banco ya existe; no se sobrescribe'; exit 1; }
mkdir -m 0700 "$output"
disk=$output/ntfs-lab.img
data=$output/datos-lab.ext4
mount_dir=$output/montaje
disk_gib=8; data_gib=6
if [[ $output == "$base/juegos-grafica" ]]; then disk_gib=14; data_gib=12; fi
truncate -s "${disk_gib}G" "$disk"
truncate -s "${data_gib}G" "$data"
[[ -f $disk && ! -L $disk && -f $data && ! -L $data ]] || exit 1
mkntfs -F -Q -L ULTRA-LAB "$disk" > "$output/formato-ntfs.txt" 2>&1
mkfs.ext4 -q -F -L ULTRA-JUEGOS -U ecbc9f71-f43c-4d20-9b71-a7929f264ba8 "$data"
mkdir -m 0700 "$mount_dir"
mounted=0
cleanup() { ((mounted==0)) || sudo umount "$mount_dir"; }
trap cleanup EXIT
sudo mount -t ntfs3 -o loop,rw,nosuid,nodev,noexec "$disk" "$mount_dir"
mounted=1
sudo cp --sparse=always "$data" "$mount_dir/ULTRA-juegos.ext4"
sudo mkdir -p "$mount_dir/Cyberpunk 2077/bin/x64"
printf 'ULTRA-LAB-GAME-ORIGINAL\n' | sudo tee "$mount_dir/Cyberpunk 2077/bin/x64/Cyberpunk2077.exe" >/dev/null
sudo umount "$mount_dir"
mounted=0
rmdir "$mount_dir"
uuid=$(blkid -p -s UUID -o value "$disk")
[[ $uuid =~ ^[0-9A-Fa-f]{16}$ ]] || exit 1
printf '{"uuid":"%s","image_uuid":"ecbc9f71-f43c-4d20-9b71-a7929f264ba8","image_bytes":%s}\n' "$uuid" "$((data_gib*1073741824))" > "$output/ntfs-lab.json"
gzip -1 -c "$disk" > "$disk.gz"
cd "$output"
sha256sum ntfs-lab.img ntfs-lab.img.gz ntfs-lab.json > ntfs-lab.sha256
echo 'VERDE: banco creado; ejecutable ficticio; ningun volumen fisico conectado'
