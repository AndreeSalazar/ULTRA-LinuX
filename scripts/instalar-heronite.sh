#!/usr/bin/bash
# SPDX-License-Identifier: GPL-2.0-only
set -euo pipefail
sistema=${1:?}; art=${2:?}; base=${3:?}
version=1.0.0
sha=a25a3cdab9ebbd685fc3d734c82864421dbead4992d84c627dba0c569cb9fa3c
url=https://github.com/RafaelGoulartB/Heronite/releases/download/v$version/heronite-$version-1-x86_64.pkg.tar.zst
mkdir -p "$base/heronite-cache"
exec 7>"$base/heronite-cache/descarga.lock"; flock 7
archive="$base/heronite-cache/heronite-$version.pkg.tar.zst"
if [[ ! -f $archive ]]; then
    curl --fail --location --retry 4 --connect-timeout 15 --max-time 300 -o "$archive.part" "$url"
    printf '%s  %s\n' "$sha" "$archive.part" | sha256sum -c -
    mv "$archive.part" "$archive"
fi
printf '%s  %s\n' "$sha" "$archive" | sha256sum -c -
# Native helpers remain available without the Electron interface. Existing user
# settings can refer to the former bundled paths; the session adapter maps them.
sudo install -d -m 0755 "$sistema/usr/lib/heronite-tools"
for tool in gogdl legendary nile; do
    old="$sistema/opt/heroic/resources/app.asar.unpacked/build/bin/x64/linux/$tool"
    if [[ -f $old && ! -L $old ]]; then
        sudo install -m 0755 "$old" "$sistema/usr/lib/heronite-tools/$tool"
    fi
done
# The immutable image seeds a writable, per-user official AppImage. Its signed
# updater replaces that copy; rebuilding ULTRA never overwrites an updated copy.
appsha=21b98fd88a11a87a64fcd6b2d0e8c5e02bd329ea4423e7283af7db65f1a7b095
appurl=https://github.com/RafaelGoulartB/Heronite/releases/download/v$version/Heronite-x86_64.AppImage
app="$base/heronite-cache/Heronite-$version.AppImage"
if [[ ! -f $app ]]; then
    curl -fL --retry 4 --connect-timeout 15 --max-time 300 "$appurl" -o "$app.part"
    printf '%s  %s\n' "$appsha" "$app.part" | sha256sum -c -
    mv "$app.part" "$app"
fi
printf '%s  %s\n' "$appsha" "$app" | sha256sum -c -
sudo install -d -m 0755 "$sistema/usr/share/ultra/heronite"
sudo install -m 0755 "$app" "$sistema/usr/share/ultra/heronite/Heronite.AppImage"
sudo bsdtar --no-same-owner -xf "$archive" -C "$sistema" usr
# Keep upstream protocol handling, but avoid a second menu entry bypassing ULTRA.
printf '\nNoDisplay=true\n' | sudo tee -a "$sistema/usr/share/applications/com.heronite.rs.desktop" >/dev/null
for binary in heronite-rs heronite-epic-login; do
    sudo chroot "$sistema" /usr/bin/ldd "/usr/bin/$binary" | tee "$art/$binary-bibliotecas.txt"
    ! grep -q 'not found' "$art/$binary-bibliotecas.txt"
done
sudo install -m 0755 "$(dirname "$0")/../src/rootfs/bin/ultra-heronite-appimage" "$sistema/usr/bin/heronite-rs"
# Only the extracted system image's known application directory is removed.
if [[ -e $sistema/opt/heroic || -L $sistema/opt/heroic ]]; then
    [[ $(realpath "$sistema/opt/heroic") == "$sistema/opt/heroic" && -d $sistema/opt/heroic && ! -L $sistema/opt/heroic ]]
    sudo rm -r -- "$sistema/opt/heroic"
fi
sudo install -d "$sistema/etc/ultra"
printf '%s\n' "$version" | sudo tee "$sistema/etc/ultra/heronite-version" >/dev/null
printf 'version=%s\nurl=%s\nsha256=%s\nappimage_url=%s\nappimage_sha256=%s\nsource_tag=v%s\n' "$version" "$url" "$sha" "$appurl" "$appsha" "$version" > "$art/heronite-origen.txt"
