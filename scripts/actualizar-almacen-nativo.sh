#!/usr/bin/env bash
# Repack a previously extracted EROFS stage inside the isolated build workshop.
# No physical devices: only laboratory image files are created here.
set -euo pipefail
perfil=${1:?pc o generico}
[[ $perfil == pc || $perfil == generico ]] || exit 2
revision=${2:-nativo}
[[ $revision =~ ^[a-z0-9][a-z0-9-]{0,31}$ ]] || exit 2
base="$HOME/linux-reconstruido"
art="$base/artifacts/$perfil"
lab=$(cd -- "$(dirname -- "$0")" && pwd)
stage="$art/sistema-nativo"
[[ -f $stage/ULTRA-SISTEMA && -f $art/sistema-a.erofs && -f $art/datos.ext4 ]]
[[ ! -e $art/sistema-antes-$revision.erofs && ! -e $art/disco-bmo-antes-$revision.img ]] || {
    echo 'ROJO: existe una entrega anterior conservada; revisar sin sobrescribirla'
    exit 1
}
! systemctl is-active --quiet linux-reconstruido-build
exec 8>"$art/almacen-nativo.lock"
flock -n 8
cd "$art"
release=$(cat kernelrelease.txt)
[[ $release =~ ^[0-9A-Za-z.+_-]+$ ]]
kernel_before=$(sha256sum bzImage-tiny64)
sudo install -m 0755 "$lab/rootfs/bin/ultra-preparar-juegos" "$stage/usr/local/libexec/ultra-preparar-juegos"
sudo install -m 0644 "$lab/rootfs/desktop/juegos.conf" "$stage/etc/ultra/juegos.conf"
for helper in ultra-juegos ultra-gog-cyberpunk ultra-ambiente; do
    sudo install -m 0755 "$lab/rootfs/bin/$helper" "$stage/usr/local/bin/$helper"
done
sudo install -m 0644 "$lab/rootfs/desktop/ultra-cyberpunk.desktop" "$stage/usr/share/applications/ultra-cyberpunk.desktop"
sudo install -m 0644 "$lab/rootfs/desktop/ultra-cyberpunk.desktop" "$stage/etc/ultra/ultra-cyberpunk.desktop"
sha256sum "$lab/rootfs/bin/ultra-preparar-juegos" "$lab/rootfs/desktop/juegos.conf" \
    "$lab/rootfs/bin/ultra-juegos" "$lab/rootfs/bin/ultra-gog-cyberpunk" \
    "$lab/rootfs/bin/ultra-ambiente" "$lab/rootfs/desktop/ultra-cyberpunk.desktop" "$0" > almacen-nativo-fuentes.sha256
sudo install -m 0644 almacen-nativo-fuentes.sha256 "$stage/etc/ultra/almacen-nativo-fuentes.sha256"
sudo mkfs.erofs --quiet -zlzma,level=6 -T 0 sistema-nativo.erofs "$stage"
sudo chown "$(id -u):$(id -g)" sistema-nativo.erofs
[[ $(stat -c %s sistema-nativo.erofs) -lt 4294967296 ]]
sudo fsck.erofs sistema-nativo.erofs
bmo="$art/disco-bmo-nativo.img"
[[ ! -e $bmo ]]
bmo_mib=$(( ($(stat -c %s sistema-nativo.erofs)+1048575)/1048576 + 128 ))
truncate -s $((1+64+bmo_mib+16+1))M "$bmo"
printf 'label: gpt\nstart=2048, size=64MiB, type=C12A7328-F81F-11D2-BA4B-00A0C93EC93B, name="EFI system partition"\nsize=%sMiB, type=EBD0A0A2-B9E5-4433-87C0-68B6B72699C7, name="Basic data partition"\nsize=16MiB, type=EBD0A0A2-B9E5-4433-87C0-68B6B72699C7, name="Basic data partition"\n' "$bmo_mib" | sfdisk -q "$bmo"
bpart() { sfdisk --dump "$bmo" | sed -n "s/^[^ ]*$1 : start= *\([0-9]*\), size= *\([0-9]*\).*/\\$2/p"; }
win=$(( $(bpart 1 1)*512 )); a=$(( $(bpart 2 1)*512 ))
export MTOOLS_SKIP_CHECK=1
mkfs.fat -F 32 -s 1 --offset=$(bpart 1 1) "$bmo" $(( $(bpart 1 2)/2 )) >/dev/null 2>&1
mmd -i "$bmo@@$win" ::/EFI ::/EFI/BOOT ::/EFI/Microsoft ::/EFI/Microsoft/Boot
mcopy -i "$bmo@@$win" bzImage-tiny64 ::/EFI/BOOT/BOOTX64.EFI
printf 'marca de prueba; no es Windows\n' > bootmgfw-falso.efi
mcopy -i "$bmo@@$win" bootmgfw-falso.efi ::/EFI/Microsoft/Boot/bootmgfw.efi
mkfs.fat -F 32 -s 1 -n BMO --offset=$(bpart 2 1) "$bmo" $(( $(bpart 2 2)/2 )) >/dev/null 2>&1
printf 'NO NAME    ' | dd of="$bmo" bs=1 seek=$((a+71)) conv=notrunc status=none
mmd -i "$bmo@@$a" ::/EFI ::/EFI/ULTRA ::/ULTRA
mcopy -i "$bmo@@$a" bzImage-tiny64 ::/EFI/ULTRA/ULTRA.EFI
mcopy -i "$bmo@@$a" sistema-nativo.erofs "::/ULTRA/sistema-$release.erofs"
mcopy -i "$bmo@@$a" datos.ext4 ::/ULTRA/datos.ext4
[[ $(mtype -i "$bmo@@$a" "::/ULTRA/sistema-$release.erofs" | sha256sum | cut -d' ' -f1) == $(sha256sum sistema-nativo.erofs | cut -d' ' -f1) ]]
[[ $(sha256sum bzImage-tiny64) == "$kernel_before" ]]
mv sistema-a.erofs "sistema-antes-$revision.erofs"
mv sistema-nativo.erofs sistema-a.erofs
[[ ! -e disco-bmo.img ]] || mv disco-bmo.img "disco-bmo-antes-$revision.img"
mv disco-bmo-nativo.img disco-bmo.img
python - <<'PY'
from pathlib import Path
import hashlib
p=Path('SHA256SUMS');rows=p.read_text().splitlines();names={'sistema-a.erofs','disco-bmo.img'};result=[]
for row in rows:
    digest,name=row.split(maxsplit=1)
    if Path(name).name in names:
        with Path(name).open('rb') as f:digest=hashlib.file_digest(f,'sha256').hexdigest()
    result.append(f'{digest}  {name}')
with Path('almacen-nativo-fuentes.sha256').open('rb') as f:
    result.append(f"{hashlib.file_digest(f,'sha256').hexdigest()}  {Path('almacen-nativo-fuentes.sha256').resolve()}")
p.write_text('\n'.join(result)+'\n')
PY
echo 'VERDE: sistema y BMO nativos empaquetados; kernel identico; pruebas e instalacion pendientes'
