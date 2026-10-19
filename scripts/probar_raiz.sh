#!/bin/bash
# SPDX-License-Identifier: GPL-2.0-only
# Workshop only: helper binary and a populated throwaway system tree as arguments.
set -euo pipefail
helper=$(realpath -- "$1")
tree=$(realpath -- "$2")
[[ -x $helper && -x $tree/usr/bin/setpriv && -d $tree/proc ]]
before=$(readlink /proc/self/ns/mnt)
set +e
"$helper" "$tree" /usr/bin/true > /tmp/ultra-raiz-rechazo-$$.log 2>&1
result=$?
set -e
[[ $result == 2 ]] || { echo 'ROJO: helper no rechazo usuario sin root'; exit 1; }
rm -- /tmp/ultra-raiz-rechazo-$$.log
echo 'VERDE [RAIZ]: invocacion sin root rechazada'
sudo unshare --mount --propagation private /bin/bash -s "$helper" "$tree" <<'INNER'
set -euo pipefail
helper=$1; tree=$2
mount --bind /proc "$tree/proc"
set +e
chroot "$tree" /usr/bin/setpriv --reuid=1000 --regid=1000 --clear-groups --no-new-privs --inh-caps=-all --ambient-caps=-all --bounding-set=-all /usr/bin/unshare --user --map-root-user /usr/bin/true
result=$?
set -e
[[ $result != 0 ]] || { echo 'ROJO: control chroot inesperadamente valido'; exit 1; }
echo 'VERDE [RAIZ]: control chroot reproduce bloqueo de userns'
"$helper" "$tree" /usr/bin/setpriv --reuid=1000 --regid=1000 --clear-groups --no-new-privs --inh-caps=-all --ambient-caps=-all --bounding-set=-all /usr/bin/bash -c '[[ $(id -u) == 1000 ]]; /usr/bin/unshare --user --map-root-user /usr/bin/cat /proc/self/uid_map'
echo 'VERDE [RAIZ]: user namespace permite mapa interno 0 -> UID inicial 1000'
INNER
[[ $(readlink /proc/self/ns/mnt) == "$before" ]]
echo 'VERDE [RAIZ]: namespace del proceso de control conservado'
