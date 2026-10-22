#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-2.0-only
# Root, disposable VM only: private mount namespace, synthetic NTFS and ext4.
# Never accepts a host block device or reads accounts/actual games.
set -euo pipefail
if [[ ${1:-} != --interno ]]; then
    [[ $(id -u) == 0 && $# == 2 ]] || { echo 'Uso: sudo bash probar-warframe.sh ROOTFS WINDOWS-FIXTURE'; exit 2; }
    exec unshare --mount --fork bash "$0" --interno "$(realpath "$1")" "$(realpath "$2")"
fi
mount --make-rprivate /
sources=$2; fixture=$3
[[ -d $sources/bin && -f $fixture && ! -b $fixture ]]
lab=$(mktemp -d /var/tmp/ultra-warframe.XXXXXX)
root=$lab/root
mkdir -p "$root"/{usr,dev,proc,etc,windows,juegos,tmp,test} "$lab/original"
loops=(); steam_pid=''
cleanup() {
    [[ -z $steam_pid ]] || { kill "$steam_pid" 2>/dev/null || true; wait "$steam_pid" 2>/dev/null || true; }
    # Explicit order, no lazy unmount of a live backing image.
    for path in "$root/juegos/Warframe" "$root/windows" "$root/juegos" "$lab/original" "$root/proc" "$root/dev" "$root/usr"; do
        if mountpoint -q "$path"; then umount -R "$path" || true; fi
    done
    for device in "${loops[@]}"; do losetup -d "$device" || true; done
    echo "Banco sintetico conservado: $lab"
}
trap cleanup EXIT
cp --reflink=auto --sparse=always "$fixture" "$lab/windows.img"
device=$(losetup --find --show "$lab/windows.img"); loops+=("$device")
mount -t ntfs3 "$device" "$lab/original"
game='Program Files (x86)/Steam/steamapps/common/Warframe'
mkdir -p "$lab/original/$game/Tools"
printf 'WARFRAME-SINTETICO\n' > "$lab/original/$game/Warframe.x64.exe"
printf 'LANZADOR-SINTETICO\n' > "$lab/original/$game/Tools/Launcher.exe"
chown -R 1000:1000 "$lab/original/$game"
cat > "$lab/original/Program Files (x86)/Steam/steamapps/appmanifest_230410.acf" <<'MANIFEST'
"AppState"
{
    "appid" "230410"
    "StateFlags" "4"
    "buildid" "12345"
    "SizeOnDisk" "100"
    "LastOwner" "CUENTA-SINTETICA-NO-COPIAR"
    "InstalledDepots"
    {
        "230411"
        {
            "manifest" "9876543210"
            "size" "100"
        }
    }
}
MANIFEST
umount "$lab/original"
losetup -d "$device"; loops=()
before=$(sha256sum "$lab/windows.img" | cut -d' ' -f1)
device=$(losetup --find --show --read-only "$lab/windows.img"); loops+=("$device")
mount -t ntfs3 -o ro,nosuid,nodev,noexec,uid=1000,gid=1000,dmask=0022,fmask=0133 "$device" "$root/windows"
truncate -s 512M "$lab/juegos.ext4"
mkfs.ext4 -q -F "$lab/juegos.ext4"
data=$(losetup --find --show "$lab/juegos.ext4"); loops+=("$data")
mount -t ext4 -o nosuid,nodev "$data" "$root/juegos"
mount --bind /usr "$root/usr"
mount -o remount,bind,ro "$root/usr"
mount --rbind /dev "$root/dev"
mount -t proc proc "$root/proc"
for dir in bin sbin; do ln -s usr/bin "$root/$dir"; done
for dir in lib lib64; do ln -s usr/lib "$root/$dir"; done
cp /etc/passwd /etc/group "$root/etc/"
install -m 0755 "$sources/bin/ultra-preparar-warframe" "$root/test/preparar"
install -m 0755 "$sources/bin/ultra-steam-warframe" "$root/test/biblioteca"
install -m 0755 /usr/bin/sleep "$root/test/steam"
install -d -m 0700 -o 1000 -g 1000 "$root/juegos/usuarios/ultra"
run_root() { chroot "$root" /usr/bin/bash /test/preparar; }
run_user() { chroot "$root" /usr/bin/env HOME=/juegos/usuarios/ultra /usr/bin/setpriv --reuid=1000 --regid=1000 --clear-groups --no-new-privs --bounding-set=-all --inh-caps=-all --ambient-caps=-all /usr/bin/bash /test/biblioteca; }
reject() { if "$@"; then echo 'FALLO: se acepto un caso que debe rechazarse'; exit 1; fi; }
count=0
pass() { count=$((count+1)); echo "PASS $count $*"; }
ln -s /tmp "$root/juegos/.ultra-warframe"
reject run_root; rm "$root/juegos/.ultra-warframe"; pass 'rechaza destino simbolico privilegiado'
mkdir "$root/juegos/.ultra-warframe"; chmod 0777 "$root/juegos/.ultra-warframe"
reject run_root; chmod 0700 "$root/juegos/.ultra-warframe"; pass 'rechaza padre modificable por usuario'
run_root; pass 'overlay real NTFS solo lectura + ext4'
run_root; pass 'preparacion repetida idempotente'
stat -c '%n %u:%g %a' "$root/juegos" "$root/juegos/Warframe" "$root/juegos/Warframe/Warframe.x64.exe" "$root/windows/$game/Warframe.x64.exe"
chroot "$root" setpriv --reuid=1000 --regid=1000 --clear-groups bash -c 'echo MODIFICADO > /juegos/Warframe/Warframe.x64.exe; echo NUEVO > /juegos/Warframe/nuevo.txt'
grep -q MODIFICADO "$root/juegos/Warframe/Warframe.x64.exe"
grep -q WARFRAME-SINTETICO "$root/windows/$game/Warframe.x64.exe"
[[ ! -e $root/windows/$game/nuevo.txt ]]; pass 'escrituras del usuario aisladas de C:'
run_user; pass 'biblioteca sin cuentas ni manifiestos Windows'
library=$root/juegos/usuarios/ultra/Juegos/SteamLibrary
[[ $(readlink "$library/steamapps/common/Warframe") == /juegos/Warframe ]]
grep -q '"StateFlags" "4"' "$library/steamapps/appmanifest_230410.acf"
grep -q '"manifest" "9876543210"' "$library/steamapps/appmanifest_230410.acf"
! grep -q 'LastOwner\|CUENTA-SINTETICA' "$library/steamapps/appmanifest_230410.acf"
[[ $(stat -c %u "$library/steamapps/compatdata") == 1000 ]]; pass 'enlace al juego y compatdata ext4'
rm "$library/steamapps/common/Warframe"; mkdir "$library/steamapps/common/Warframe"
reject run_user; [[ -d $library/steamapps/common/Warframe ]]; pass 'conserva instalaciones previas en el destino'
rmdir "$library/steamapps/common/Warframe"; ln -s /juegos/Warframe "$library/steamapps/common/Warframe"
printf '\n"sentinela" "conservar"\n' >> "$library/steamapps/appmanifest_230410.acf"
run_user; grep -q sentinela "$library/steamapps/appmanifest_230410.acf"; pass 'no sobreescribe manifiesto existente'
chroot "$root" setpriv --reuid=1000 --regid=1000 --clear-groups /test/steam 60 &
steam_pid=$!
sleep 0.2
reject run_user; pass 'rechaza preparacion con Steam activo'
kill "$steam_pid"; wait "$steam_pid" || true; steam_pid=''
umount "$root/juegos/Warframe"
reject run_user; pass 'rechaza biblioteca sin overlay'
printf '000\n' > "$root/juegos/.ultra-warframe/windows-build"
reject run_root; pass 'rechaza mezcla con otra version Windows sin borrar la capa'
printf '12345\n' > "$root/juegos/.ultra-warframe/windows-build"
run_root; grep -q MODIFICADO "$root/juegos/Warframe/Warframe.x64.exe"; pass 'capa persistente tras desmontar/remontar'
umount "$root/juegos/Warframe"
umount "$root/windows"
reject run_root; pass 'rechaza Windows ausente'
losetup -d "$device"; loops=("$data")
after=$(sha256sum "$lab/windows.img" | cut -d' ' -f1)
[[ $before == "$after" ]]; pass 'SHA256 NTFS completo inalterado'
printf 'RESULTADO: %s comprobaciones; sin juego real, cuenta ni GPU fisica\n' "$count"
