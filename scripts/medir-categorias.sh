#!/bin/bash
# SPDX-License-Identifier: GPL-2.0-only
# Anti3M applied to the kernel: how many lines each category of the tree has,
# and how many a built profile really compiles (its .c/.S plus the headers they
# pulled in, read from Kbuild's .cmd dependency files). Runs in the workshop.
#   bash medir-categorias.sh [perfil]      perfil: generico | pc (default pc)
set -euo pipefail
perfil=${1:-pc}
base="$HOME/linux-reconstruido"
src="$base/kernel"
out="$base/build/x86_64-$perfil"
[[ -d $out ]] || { echo "ROJO: no hay compilacion del perfil $perfil en $out"; exit 1; }
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
cd "$src"

# Lines per top-level directory: every tracked C, assembly, header and Rust file.
by_dir() { awk '{split($2, p, "/"); d = ($2 ~ /\//) ? p[1] : "(raiz)"; t[d] += $1; all += $1}
                END {for (d in t) print d, t[d]; print "TOTAL", all}'; }
git ls-files -z -- '*.c' '*.h' '*.S' '*.rs' | xargs -0 wc -l | grep -v ' total$' | by_dir | sort > "$tmp/total"

# Compiled: one source per object of the profile, plus every header any object read.
find "$out" -name '*.o' -not -name '*.mod.o' -not -name 'built-in*' -printf '%P\n' | sed 's/\.o$//' |
    while read -r o; do for e in c S; do if [[ -f $o.$e ]]; then echo "$o.$e"; fi; done; done | sort -u > "$tmp/srcs"
find "$out" -name '.*.o.cmd' -print0 | xargs -0 cat | grep -o '[^ :]*\.h' |
    # Paths come relative or absolute, many times each: dedupe first, then one realpath per batch.
    # Only the ones inside the source tree count. Tokens starting with "-" are compiler flags.
    grep -v '^-' | sort -u | xargs -d '\n' realpath -m --relative-to="$src" -- | grep -v '^\.\./' | sort -u > "$tmp/hdrs"
cat "$tmp/srcs" "$tmp/hdrs" | sort -u | while read -r f; do if [[ -f $f ]]; then echo "$f"; fi; done > "$tmp/used"
xargs wc -l < "$tmp/used" | grep -v ' total$' | by_dir | sort > "$tmp/compiled"

echo "Perfil $perfil, kernel $(git rev-parse --short=12 HEAD), $(date -u +%F)"
echo "Archivos compilados: $(wc -l < "$tmp/srcs") .c/.S y $(wc -l < "$tmp/hdrs") cabeceras"
printf '%-14s %12s %12s %7s\n' categoria arbol compilado usado
join -a1 -e0 -o 0,1.2,2.2 "$tmp/total" "$tmp/compiled" | sort -k2 -nr |
    awk '{printf "%-14s %12d %12d %6.1f%%\n", $1, $2, $3, ($2 ? 100 * $3 / $2 : 0)}'
