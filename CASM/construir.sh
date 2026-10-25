#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-2.0-only
# Construye la semilla C con GCC o Clang; CASM usa su codificador x86-64 propio.
set -euo pipefail
casm="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec python3 -B "$casm/bootstrap.py" construir "$@"
