#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-2.0-only
# Update the generic rescue system in the already-built PC dual disk, not host media.
set -euo pipefail
base="$HOME/linux-reconstruido/artifacts"
gen=$base/generico; pc=$base/pc
[[ -f $gen/build-complete && -f $pc/build-complete ]] || exit 1
! systemctl is-active --quiet linux-reconstruido-build || exit 1
exec 8>"$pc/tema.lock"
flock -n 8 || exit 1
cd "$pc"
sha256sum --check --status SHA256SUMS
release=$(cat "$gen/kernelrelease.txt")
[[ $release =~ ^[0-9A-Za-z.+_-]+$ ]] || exit 2
offset=$(sfdisk -J disco-bmo-doble.img | python -c 'import json,sys; p=json.load(sys.stdin)["partitiontable"]["partitions"]; assert len(p)>=2; print(p[1]["start"]*512)')
[[ $offset =~ ^[0-9]+$ ]] || exit 2
rm build-complete
export MTOOLS_SKIP_CHECK=1
mcopy -o -i "disco-bmo-doble.img@@$offset" "$gen/sistema-a.erofs" "::/ULTRA/sistema-$release.erofs"
expected=$(sha256sum "$gen/sistema-a.erofs" | cut -d' ' -f1)
[[ $(mtype -i "disco-bmo-doble.img@@$offset" "::/ULTRA/sistema-$release.erofs" | sha256sum | cut -d' ' -f1) == "$expected" ]]
mapfile -t paths < <(awk '{print $2}' SHA256SUMS)
sha256sum "${paths[@]}" > SHA256SUMS.nuevo
mv SHA256SUMS.nuevo SHA256SUMS
touch build-complete
echo 'VERDE: rescate generico del disco doble sincronizado y verificado'
