#!/bin/bash
# SPDX-License-Identifier: GPL-2.0-only
# Dedicated software graphics sandbox: no real device or desktop socket exposed.
set -euo pipefail
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
    -- "/minas/$name" "$@"
