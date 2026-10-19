# SPDX-License-Identifier: GPL-2.0-only
# Production entry point; repository and flat VM staging layouts.
ultra_profile_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
if [[ -f "$ultra_profile_dir/guardian/produccion/perfil-gcc.sh" ]]; then
    ULTRA_GUARD_ROOT="$ultra_profile_dir/guardian"
else
    ULTRA_GUARD_ROOT="$ultra_profile_dir/../src/guardian"
fi
source "$ULTRA_GUARD_ROOT/produccion/perfil-gcc.sh"
unset ultra_profile_dir
