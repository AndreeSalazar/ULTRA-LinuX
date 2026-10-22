#!/usr/bin/bash
# SPDX-License-Identifier: GPL-2.0-only
# Run only with ejecutar-grafica-limitada.sh: disposable /tmp and no real GPU.
set -euo pipefail
helper=$(dirname -- "$(realpath -- "$0")")/ultra-graficos
[[ $(id -u) == 1000 && ! -e /dev/dri ]] || exit 125
export HOME=/tmp/usuario XDG_RUNTIME_DIR=/tmp/runtime
mkdir -p "$HOME" "$XDG_RUNTIME_DIR" /tmp/fixtures
chmod 700 "$XDG_RUNTIME_DIR"
export VK_DRIVER_FILES=/usr/share/vulkan/icd.d/lvp_icd.json
export VK_LOADER_DEBUG=layer
export WLR_BACKENDS=headless WLR_HEADLESS_OUTPUTS=1 WLR_RENDERER=pixman
export WLR_LIBINPUT_NO_DEVICES=1
bash -n "$helper"
# Package versions are recorded by the outer runner before entering this sandbox.
# Genuine Wayland + software Vulkan. No stub is used for this control.
real_status=0
cage -d -- /usr/bin/bash "$helper" validar > /tmp/real.log 2>&1 || real_status=$?
if [[ $real_status != 0 ]]; then
    wc -c /tmp/real.log
    head -n 25 /tmp/real.log
    tail -n 25 /tmp/real.log
    exit "$real_status"
fi
cat /tmp/real.log
cat "$HOME/.cache/ultra/vulkan-cubo.log"
grep -qx 'ULTRA_CUBO codigo_salida=0 fotogramas_solicitados=120' "$HOME/.cache/ultra/vulkan-cubo.log"
grep -Eq 'Insert instance layer.*VK_LAYER_KHRONOS_validation' "$HOME/.cache/ultra/vulkan-cubo.log"
! grep -Eq 'Validation Error|VUID-|SYNC-HAZARD' "$HOME/.cache/ultra/vulkan-cubo.log"
echo 'VERDE [INTEGRACION-01]: vkcube real en Wayland headless; cierre normal y validacion sin errores'

# Synthetic failure paths do not wait 30 seconds and never send GPU work.
cat > /tmp/fixtures/notify-send <<'EOF'
#!/usr/bin/bash
exit 0
EOF
cat > /tmp/fixtures/timeout <<'EOF'
#!/usr/bin/bash
[[ $1 == -k && $2 == 2 && $3 == 30 ]] || exit 99
exit 124
EOF
chmod 755 /tmp/fixtures/*
status=0
PATH=/tmp/fixtures:/usr/bin bash "$helper" cubo || status=$?
[[ $status == 124 ]]
grep -qx 'ULTRA_CUBO codigo_salida=124 fotogramas_solicitados=120' "$HOME/.cache/ultra/vulkan-cubo.log"
echo 'VERDE [INTEGRACION-02]: timeout simulado se devuelve como fallo y queda registrado'
cat > /tmp/fixtures/timeout <<'EOF'
#!/usr/bin/bash
shift 3
exec "$@"
EOF
cat > /tmp/fixtures/vkcube <<'EOF'
#!/usr/bin/bash
[[ $* == '--wsi wayland --c 120' ]] || exit 98
exit 7
EOF
chmod 755 /tmp/fixtures/*
status=0
PATH=/tmp/fixtures:/usr/bin bash "$helper" cubo || status=$?
[[ $status == 7 ]]
grep -qx 'ULTRA_CUBO codigo_salida=7 fotogramas_solicitados=120' "$HOME/.cache/ultra/vulkan-cubo.log"
echo 'VERDE [INTEGRACION-03]: error simulado de vkcube se conserva como fallo'
