#!/bin/sh
# SPDX-License-Identifier: GPL-2.0-only
# Servidor MCP del SUPER GRAFO (ULTRA Carpetas) por stdio, para Claude Code u otro cliente MCP.
# Uso: scripts/super-grafo-mcp.sh [carpeta]   (por defecto, la raiz de este repositorio)
# stdout es solo del protocolo: la compilacion, si hace falta, escribe en stderr.
set -eu
raiz=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
carpeta=${1:-$raiz}
# Resolver el argumento antes de cambiar de carpeta para fijar rust-toolchain.toml.
carpeta=$(CDPATH= cd -- "$carpeta" && pwd)
cd "$raiz/src/apps/ultra-carpetas"
echo "super-grafo: comprobando compilacion incremental..." >&2
# Cargo comprueba los cambios aun si existe el ejecutable de una revision anterior.
# Si falla, no arrancar silenciosamente ese binario viejo.
cargo build --release --locked --quiet >&2
bin="${CARGO_TARGET_DIR:-target}/release/ultra-carpetas"
exec "$bin" --mcp "$carpeta"
