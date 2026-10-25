#!/usr/bin/bash
# SPDX-License-Identifier: GPL-2.0-only
# ULTRA Carpetas (Rust + egui + wgpu: Vulkan). Se compila aqui desde src/apps/ultra-carpetas,
# con Cargo.lock y Rust 1.97.0 fijados. Antes se prueba el nucleo (src/apps/carpetas-nucleo) y se
# comprueba que sigue siendo no_std compilandolo para x86_64-unknown-none, como BMO-X.
# Si falla, avisa en amarillo y el sistema sigue con Thunar.
set -euo pipefail
sistema=$1; art=$2; base=$3
lab=$(cd -- "$(dirname -- "$0")" && pwd)
fuente="$lab/../src/apps/ultra-carpetas"
nucleo="$lab/../src/apps/carpetas-nucleo"
destino="$base/carpetas-target"
command -v cargo > /dev/null || sudo pacman -S --needed --noconfirm rustup > "$art/carpetas-herramientas.log" 2>&1 ||
    { echo 'AMARILLO [CARPETAS]: sin Rust; se queda Thunar'; exit 0; }
if ! (cd "$nucleo" && CARGO_TARGET_DIR="$destino-nucleo" cargo test --locked &&
       CARGO_TARGET_DIR="$destino-nucleo" cargo build --release --locked --target x86_64-unknown-none) > "$art/carpetas-nucleo.log" 2>&1; then
    tail -20 "$art/carpetas-nucleo.log"; echo 'ROJO [CARPETAS]: el nucleo no pasa sus pruebas o ya no es no_std'; exit 1
fi
if ! (cd "$fuente" && CARGO_TARGET_DIR="$destino" cargo build --release --locked) > "$art/carpetas-compilar.log" 2>&1; then
    tail -20 "$art/carpetas-compilar.log"; echo 'AMARILLO [CARPETAS]: no compilo; se queda Thunar'; exit 0
fi
(cd "$fuente" && CARGO_TARGET_DIR="$destino" cargo test --release --locked) > "$art/carpetas-pruebas.log" 2>&1 ||
    { tail -20 "$art/carpetas-pruebas.log"; echo 'ROJO [CARPETAS]: fallan las pruebas'; exit 1; }
sudo install -m 0755 "$destino/release/ultra-carpetas" "$sistema/usr/local/bin/ultra-carpetas"
sudo chroot "$sistema" /usr/bin/ldd /usr/local/bin/ultra-carpetas > "$art/carpetas-bibliotecas.txt"
! grep -q 'not found' "$art/carpetas-bibliotecas.txt" || { echo 'ROJO [CARPETAS]: faltan bibliotecas en la imagen'; exit 1; }
echo "VERDE [CARPETAS]: $(sha256sum "$destino/release/ultra-carpetas" | cut -c1-16) instalado"
