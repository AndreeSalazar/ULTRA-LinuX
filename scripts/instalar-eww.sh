#!/usr/bin/bash
# SPDX-License-Identifier: GPL-2.0-only
# eww (tarjetas de Equipo en vivo) no esta en los repositorios oficiales de Arch.
# Se compila aqui desde el codigo fijado: etiqueta v0.6.0 y su commit exacto, solo Wayland.
# Sin AUR. Si falta algo para compilar, avisa en amarillo y el escritorio sigue sin tarjetas
# (Super+F9 abre entonces la version de texto).
set -euo pipefail
sistema=$1; art=$2; base=$3
version=v0.6.0
commit=d87c2fdbfdc012e76d229e4e9ea3325bc0f23e89
url=https://github.com/elkowar/eww.git
cache="${ULTRA_BUILD_CACHE:-$base}/eww-cache"; binario="$cache/eww-$commit"
mkdir -p "$cache"
exec 7>"$cache/compilar.lock"
flock 7
if [[ ! -x $binario ]]; then
    # Herramientas del taller: Rust (rustup elige 1.76.0 por rust-toolchain.toml) y cabeceras GTK.
    sudo pacman -S --needed --noconfirm rustup gtk3 gtk-layer-shell libdbusmenu-gtk3 pkgconf git > "$art/eww-herramientas.log" 2>&1 ||
        { echo 'AMARILLO [EWW]: no se pudieron instalar las herramientas; sin tarjetas'; exit 0; }
    fuente="$cache/src-$commit"
    [[ -d $fuente/.git ]] || git -c advice.detachedHead=false clone --quiet --depth 1 --branch "$version" "$url" "$fuente"
    [[ $(git -C "$fuente" rev-parse HEAD) == "$commit" ]] || { echo "ROJO [EWW]: $version no apunta a $commit"; exit 1; }
    if ! (cd "$fuente" && CARGO_TARGET_DIR="$cache/target-$commit" cargo build --release --locked --no-default-features --features wayland) > "$art/eww-compilar.log" 2>&1; then
        tail -20 "$art/eww-compilar.log"; echo 'AMARILLO [EWW]: no compilo; sin tarjetas'; exit 0
    fi
    install -m 0755 "$cache/target-$commit/release/eww" "$binario"
fi
sudo install -m 0755 "$binario" "$sistema/usr/local/bin/eww"
sudo chroot "$sistema" /usr/bin/ldd /usr/local/bin/eww | tee "$art/eww-bibliotecas.txt" > /dev/null
! grep -q 'not found' "$art/eww-bibliotecas.txt" || { echo 'ROJO [EWW]: faltan bibliotecas en la imagen'; exit 1; }
printf 'version=%s\ncommit=%s\nurl=%s\nfunciones=wayland\nsha256=%s\n' "$version" "$commit" "$url" "$(sha256sum "$binario" | cut -d' ' -f1)" > "$art/eww-origen.txt"
echo "VERDE [EWW]: $version ($commit) instalado"
