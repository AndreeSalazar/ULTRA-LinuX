#!/bin/bash
# SPDX-License-Identifier: GPL-2.0-only
# Workshop-only transient cgroup; actual workload remains the calling non-root user.
set -euo pipefail
[[ $(id -u) != 0 ]] || { echo 'ROJO: ejecutar desde el usuario del taller, no root' >&2; exit 125; }
script_dir=$(cd -- "$(dirname -- "$0")" && pwd)
task_binary=$(realpath -- "$1"); shift
exec sudo -n systemd-run --quiet --pipe --wait --collect --service-type=exec \
    --uid="$(id -u)" --gid="$(id -g)" \
    --setenv=ULTRA_GRAFICA_CGROUP=1 \
    -p MemoryMax=512M -p MemorySwapMax=0 -p TasksMax=64 -p RuntimeMaxSec=35s \
    /usr/bin/bash "$script_dir/aislar-grafica-software.sh" "$task_binary" "$@"
