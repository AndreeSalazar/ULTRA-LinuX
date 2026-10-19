#!/bin/bash
# SPDX-License-Identifier: GPL-2.0-only
# Official upstream ZIP, pinned to the SHA256 published by its GitHub release.
set -euo pipefail
sistema=$1; art=$2; base=$3
version=1.97.59
sha=32979198ff09c4c656b0b060d929a44cdd6af167b2a141a77c2b7f36fd59e661
url=https://github.com/brave/brave-browser/releases/download/v$version/brave-browser-$version-linux-amd64.zip
cache=$base/brave-cache
mkdir -p "$cache"
archive=$cache/brave-$version.zip
if [[ ! -f $archive ]]; then
    curl --fail --location --retry 2 --connect-timeout 15 --max-time 300 -o "$archive.part" "$url"
    printf '%s  %s\n' "$sha" "$archive.part" | sha256sum -c -
    mv -- "$archive.part" "$archive"
fi
printf '%s  %s\n' "$sha" "$archive" | sha256sum -c -
sudo install -d -m 0755 "$sistema/opt/brave" "$sistema/usr/share/icons/hicolor/64x64/apps"
sudo bsdtar --no-same-owner -xf "$archive" -C "$sistema/opt/brave"
sudo chown -R 0:0 "$sistema/opt/brave"
# NNP remains set; Chromium must use its namespace sandbox, never this setuid path.
sudo chmod 0755 "$sistema/opt/brave/chrome-sandbox"
sudo install -m 0644 "$sistema/opt/brave/product_logo_64.png" "$sistema/usr/share/icons/hicolor/64x64/apps/brave-browser.png"
sudo chroot "$sistema" /usr/bin/ldd /opt/brave/brave | tee "$art/brave-bibliotecas.txt"
! grep -q 'not found' "$art/brave-bibliotecas.txt" || { echo 'ROJO [BRAVE]: bibliotecas ausentes'; exit 1; }
sudo chroot "$sistema" /usr/bin/test -s /etc/ssl/certs/ca-certificates.crt
printf 'version=%s\nurl=%s\nsha256=%s\nsandbox=userns+seccomp; setuid desactivado\n' "$version" "$url" "$sha" > "$art/brave-origen.txt"
echo "VERDE: Brave $version oficial instalado; apertura y sandbox pendientes de prueba"
