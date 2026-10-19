#!/bin/bash
# SPDX-License-Identifier: GPL-2.0-only
# A new namespace sandbox for every invocation, with no workshop/home mounted.
set -euo pipefail
binary=$(realpath -- "$1"); shift
dir=$(dirname -- "$binary")
name=$(basename -- "$binary")
[[ -x $binary ]] || { echo 'AISLAMIENTO-ERROR: ejecutable ausente' >&2; exit 125; }
exec timeout --kill-after=2s 15s prlimit --core=0 --cpu=10 --fsize=1048576 --nofile=128 --nproc=128 --stack=8388608 -- \
    bwrap --unshare-all --die-with-parent --new-session --cap-drop ALL \
    --ro-bind /usr /usr --symlink usr/lib /lib --symlink usr/lib /lib64 \
    --symlink usr/bin /bin --proc /proc --dev /dev \
    --size 16777216 --tmpfs /tmp --dir /run --dir /etc \
    --ro-bind "$dir" /minas --chdir /tmp --clearenv \
    --setenv PATH /usr/bin --setenv HOME /tmp \
    --setenv ASAN_OPTIONS detect_stack_use_after_return=1:detect_leaks=1:detect_invalid_pointer_pairs=2 \
    --setenv TSAN_OPTIONS halt_on_error=1:exitcode=66 \
    --setenv GLIBC_TUNABLES glibc.cpu.hwcaps=SHSTK:glibc.cpu.x86_shstk=on \
    -- "/minas/$name" "$@"
