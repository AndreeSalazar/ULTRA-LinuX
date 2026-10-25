#!/bin/sh
# SPDX-License-Identifier: GPL-2.0-only
# Servidor MCP del SUPER GRAFO (ULTRA Carpetas) por stdio, para Claude Code u otro cliente MCP.
# Uso: scripts/super-grafo-mcp.sh [carpeta]   (por defecto, la raiz de este repositorio)
# stdout es solo del protocolo: la compilacion, si hace falta, escribe en stderr.
set -eu
raiz=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
bin="$raiz/src/apps/ultra-carpetas/target/release/ultra-carpetas"
if [ ! -x "$bin" ]; then
    echo "super-grafo: compilando ultra-carpetas (solo la primera vez)..." >&2
    cargo build --release --quiet --manifest-path "$raiz/src/apps/ultra-carpetas/Cargo.toml" >&2
fi
exec "$bin" --mcp "${1:-$raiz}"
