#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-2.0-only
# Build on the isolated Arch workshop, using the same runtime library versions.
set -euo pipefail
stage=${1:?stage}; cache=${2:?cache}
lab=$(cd -- "$(dirname -- "$0")" && pwd)
test -f "$stage/ULTRA-SISTEMA"
if ! command -v meson >/dev/null || ! command -v gdbus-codegen >/dev/null; then
    sudo pacman -S --needed --noconfirm meson glib2-devel
fi
mkdir -p "$cache"
archive="$cache/waybar-0.15.0.tar.gz"
if [[ ! -f $archive ]]; then
    curl -fL --retry 2 https://codeload.github.com/Alexays/Waybar/tar.gz/refs/tags/0.15.0 -o "$archive"
fi
echo "21c2bbef88c40473c355003582f9331d2f9b8a01efdcce0935edfc5f6b023a3e  $archive" | sha256sum -c -
src="$cache/Waybar-0.15.0"
if [[ ! -f $src/.ultra-lua ]]; then
    # Extract the pinned archive over its own generated build source directory.
    tar -xzf "$archive" -C "$cache"
    python3 "$lab/waybar-numericos-lua.py" "$src"
    touch "$src/.ultra-lua"
fi
export CXXFLAGS='-O2 -D_FORTIFY_SOURCE=3 -fstack-protector-strong -fcf-protection=full'
export LDFLAGS='-Wl,-z,relro,-z,now'
opts=()
[[ ! -f $cache/build/meson-private/coredata.dat ]] || opts+=(--reconfigure)
meson setup "${opts[@]}" "$cache/build" "$src" --buildtype=release --wrap-mode=nofallback \
    -Doptimization=2 -Dman-pages=disabled -Dtests=disabled -Dcava=disabled \
    -Dexperimental=true -Dlibnl=enabled -Dpulseaudio=enabled
ninja -C "$cache/build" -j2
candidate=/usr/local/libexec/waybar-lua-candidata
sudo install -m 0755 "$cache/build/waybar" "$stage$candidate"
sudo chroot "$stage" "$candidate" --version
sudo chroot "$stage" /usr/bin/ldd "$candidate" > "$cache/bibliotecas.txt"
! grep -q 'not found' "$cache/bibliotecas.txt"
sudo mv "$stage$candidate" "$stage/usr/bin/waybar"
sha256sum "$archive" "$src/src/modules/hyprland/workspace.cpp" "$stage/usr/bin/waybar" > "$cache/SHA256SUMS"
sudo install -m 0644 "$cache/SHA256SUMS" "$stage/etc/ultra/waybar-lua.sha256"
