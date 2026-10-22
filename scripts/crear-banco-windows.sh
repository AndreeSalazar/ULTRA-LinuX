#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-2.0-only
# Ordinary image only. Never accepts a block device or an existing output folder.
set -euo pipefail
output=${1:?carpeta nueva windows-prueba}
[[ $output == "$HOME/linux-reconstruido/windows-prueba" && ! -e $output ]] || exit 2
mkdir -m 0700 "$output"
disk=$output/windows.img
truncate -s 256M "$disk"
mkntfs -F -Q -L WINDOWS-LAB "$disk" > "$output/formato.txt" 2>&1
ntfslabel --new-serial=d8bab58cbab567a0 "$disk"
printf 'WINDOWS-LECTURA-LAB\n' > "$output/prueba.txt"
ntfscp "$disk" "$output/prueba.txt" /prueba.txt
[[ $(blkid -p -s UUID -o value "$disk") == D8BAB58CBAB567A0 ]]
gzip -1 -c "$disk" > "$disk.gz"
cd "$output"
sha256sum windows.img windows.img.gz > SHA256SUMS
echo 'VERDE: Windows ficticio creado; ningun archivo de C: incluido'
