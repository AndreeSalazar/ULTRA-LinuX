#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-2.0-only
# Perfil Native de Linux x86-64, distinto del metal64 0.2 de compilar.sh.
set -euo pipefail
casm="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec python3 -B "$casm/native/driver.py" "$@"
