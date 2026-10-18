#!/bin/sh
# SPDX-License-Identifier: GPL-2.0-only
set -eu
if [ "$#" -ne 1 ]; then
    echo 'Uso: preparar-linux.sh /ruta/linux/destino-nuevo' >&2
    exit 2
fi
base=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
source_repo="$base/upstream/linux.git"
destination=$1
[ -d "$source_repo" ] || { echo 'ROJO: falta el repositorio oficial.' >&2; exit 1; }
[ ! -e "$destination" ] || { echo 'ROJO: el destino ya existe; no se sobrescribe.' >&2; exit 1; }
case "$destination" in
    /mnt/[a-z]/*) echo 'ROJO: usa un sistema de archivos Linux para la copia de trabajo.' >&2; exit 1 ;;
esac
expected=$(git --git-dir="$source_repo" rev-parse HEAD)
git clone --no-hardlinks --no-checkout "$source_repo" "$destination"
git -C "$destination" switch -c experiment/base "$expected"
actual=$(git -C "$destination" rev-parse HEAD)
[ "$actual" = "$expected" ] || { echo 'ROJO: el commit no coincide.' >&2; exit 1; }
printf 'VERDE: copia de trabajo preparada en %s, commit %s. Pendiente compilar y probar.\n' "$destination" "$actual"