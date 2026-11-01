#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-2.0-only
# Official Linux bootstrap; its host/runtime updates live in the user's profile.
set -euo pipefail
stage=${1:?stage}; art=${2:?art}; base=${3:?base}
cache="${ULTRA_BUILD_CACHE:-$base}/discord-oficial"
mkdir -p "$cache"
archive="$cache/discord-bootstrap-0.1.0.tar.gz"
sha=f25a97173c895a3f64d49261609f9c91309b20c1d4eb7670a2cb4caf808041c0
url='https://discord.com/api/download/stable?platform=linux&format=tar.gz'
if [[ ! -f $archive ]]; then
    curl -fL --retry 3 --connect-timeout 15 --max-time 300 "$url" -o "$archive.part"
    echo "$sha  $archive.part" | sha256sum -c -
    mv "$archive.part" "$archive"
fi
echo "$sha  $archive" | sha256sum -c -
temporary=$(mktemp -d "$cache/extraer-XXXXXX");trap 'rm -rf -- "$temporary"' EXIT
tar -xzf "$archive" -C "$temporary"
sudo install -d -m 0755 "$stage/usr/share/discord" "$stage/usr/share/icons/hicolor/256x256/apps"
# Do not run the vendor postinst: it removes user caches and stops processes.
sudo install -m 0755 "$temporary/Discord/discord" "$temporary/Discord/updater_bootstrap" "$stage/usr/share/discord/"
sudo install -m 0644 "$temporary/Discord/discord.png" "$stage/usr/share/icons/hicolor/256x256/apps/discord.png"
sudo chroot "$stage" /usr/share/discord/updater_bootstrap --version > "$art/discord-version.txt"
sudo chroot "$stage" /usr/bin/ldd /usr/share/discord/updater_bootstrap > "$art/discord-bibliotecas.txt"
! grep -q 'not found' "$art/discord-bibliotecas.txt"
printf 'url=%s\nsha256=%s\nhost=actualizacion oficial en XDG_CONFIG_HOME/discord\n' "$url" "$sha" > "$art/discord-origen.txt"
