#!/usr/bin/bash
# SPDX-License-Identifier: GPL-2.0-only
# Official release asset and its published GitHub SHA256, no AUR or remote setup.
set -euo pipefail
sistema=$1; art=$2; base=$3
version=2.22.4
sha=81497c4e67d42905da5839a32be053c011eae35fb1fd5ec9441249b238d8f99b
url=https://github.com/Heroic-Games-Launcher/HeroicGamesLauncher/releases/download/v$version/Heroic-$version-linux-x64.tar.xz
mkdir -p "$base/heroic-cache"
exec 7>"$base/heroic-cache/descarga.lock"
flock 7
archive="$base/heroic-cache/heroic-$version.tar.xz"
if [[ ! -f $archive ]]; then
    curl --fail --location --retry 4 --retry-all-errors --connect-timeout 15 --max-time 300 --continue-at - -o "$archive.part" "$url"
    printf '%s  %s\n' "$sha" "$archive.part" | sha256sum -c -
    mv "$archive.part" "$archive"
fi
printf '%s  %s\n' "$sha" "$archive" | sha256sum -c -
sudo install -d -m 0755 "$sistema/opt/heroic"
sudo bsdtar --no-same-owner --strip-components 1 -xf "$archive" -C "$sistema/opt/heroic"
sudo chown -R 0:0 "$sistema/opt/heroic"
sudo chmod 0755 "$sistema/opt/heroic/chrome-sandbox"
sudo chroot "$sistema" /usr/bin/ldd /opt/heroic/heroic | tee "$art/heroic-bibliotecas.txt"
! grep -q 'not found' "$art/heroic-bibliotecas.txt"
printf 'version=%s\nurl=%s\nsha256=%s\nsandbox=sin desactivar; setuid desactivado; apertura pendiente de prueba\n' "$version" "$url" "$sha" > "$art/heroic-origen.txt"
