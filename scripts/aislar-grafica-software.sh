#!/bin/bash
# SPDX-License-Identifier: GPL-2.0-only
# Dedicated software graphics sandbox: no real device or desktop socket exposed.
set -euo pipefail
[[ ${ULTRA_GRAFICA_CGROUP:-} == 1 && $(id -u) != 0 ]] || {
    echo 'AISLAMIENTO-ERROR: usar ejecutar-grafica-limitada.sh desde el usuario del taller' >&2; exit 125;
}
task_cgroup=$(awk -F: '$1 == "0" {print $3}' /proc/self/cgroup)
[[ -n $task_cgroup ]] || { echo 'AISLAMIENTO-ERROR: cgroup v2 ausente' >&2; exit 125; }
task_memory=$(cat "/sys/fs/cgroup$task_cgroup/memory.max")
task_swap=$(cat "/sys/fs/cgroup$task_cgroup/memory.swap.max")
task_pids=$(cat "/sys/fs/cgroup$task_cgroup/pids.max")
[[ $task_memory == 536870912 && $task_swap == 0 && $task_pids == 64 ]] || {
    echo 'AISLAMIENTO-ERROR: limites cgroup no coinciden' >&2; exit 125;
}
echo "VERDE [AISLAMIENTO]: uid=$(id -u) memoria=$task_memory swap=$task_swap tareas=$task_pids"
binary=$(realpath -- "$1"); shift
dir=$(dirname -- "$binary")
name=$(basename -- "$binary")
[[ -x $binary ]] || { echo 'AISLAMIENTO-ERROR: ejecutable ausente' >&2; exit 125; }
exec timeout --kill-after=2s 30s prlimit --core=0 --cpu=20 --fsize=1048576 --nofile=128 --nproc=128 --stack=8388608 -- \
    bwrap --unshare-all --die-with-parent --new-session --cap-drop ALL \
    --ro-bind /usr /usr --symlink usr/lib /lib --symlink usr/lib /lib64 \
    --symlink usr/bin /bin --proc /proc --dev /dev \
    --size 67108864 --tmpfs /tmp --dir /run --dir /etc \
    --ro-bind "$dir" /minas --chdir /tmp --clearenv \
    --setenv PATH /usr/bin --setenv HOME /tmp --setenv XDG_RUNTIME_DIR /tmp \
    --setenv LIBGL_ALWAYS_SOFTWARE 1 --setenv GALLIUM_DRIVER llvmpipe \
    --setenv LP_NUM_THREADS 2 --setenv MESA_SHADER_CACHE_DISABLE true \
    --setenv ASAN_OPTIONS detect_leaks=1:halt_on_error=1 \
    --setenv UBSAN_OPTIONS halt_on_error=1:print_stacktrace=1 \
    -- /usr/bin/bash -c '
        set -euo pipefail
        task_uid=$(id -u)
        [[ $task_uid != 0 && ! -e /dev/dri ]] || { echo "AISLAMIENTO-ERROR: identidad o GPU inesperada" >&2; exit 125; }
        echo "VERDE [CREDENCIALES]: uid=$task_uid dentro del aislamiento; sin /dev/dri"
        exec "$@"
    ' -- "/minas/$name" "$@"
