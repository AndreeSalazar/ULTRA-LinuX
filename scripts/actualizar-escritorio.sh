#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-2.0-only
# Repack only a previously built laboratory image. Never accesses host disks.
set -euo pipefail
perfil=${1:?generico o pc}
[[ $perfil == generico || $perfil == pc ]] || exit 2
modo=${2:-completo}
[[ $modo == completo || $modo == fuentes ]] || exit 2
base="$HOME/linux-reconstruido"
art="$base/artifacts/$perfil"
lab=$(cd -- "$(dirname -- "$0")" && pwd)
[[ -f $art/build-complete && -d $art/sistema-a ]] || { echo 'ROJO: completar la construccion inicial'; exit 1; }
if systemctl is-active --quiet linux-reconstruido-build; then
    echo 'ROJO: esperar a que termine la construccion completa'; exit 1
fi
exec 8>"$art/tema.lock"
flock -n 8 || { echo 'ROJO: otra actualizacion de este escritorio esta en curso'; exit 1; }
cd "$art"
sha256sum --check --status SHA256SUMS
release=$(cat kernelrelease.txt)
[[ $release =~ ^[0-9A-Za-z.+_-]+$ ]] || exit 2
kernel_before=$(sha256sum bzImage-tiny64)
rm -f build-complete
if [[ $modo == completo ]]; then
sudo pacman --config "$art/pacman-sistema.conf" --root "$art/sistema-a" --dbpath "$art/sistema-a/var/lib/pacman" \
    -S --needed --noconfirm papirus-icon-theme otf-font-awesome ttf-jetbrains-mono rustup gcc git pkgconf grim slurp wl-clipboard cliphist mako libnotify gawk procps-ng lib32-libxtst lib32-libxrandr mesa-utils vulkan-validation-layers xdg-desktop-portal-gtk gnome-keyring libsecret
if sudo pacman --root "$art/sistema-a" --dbpath "$art/sistema-a/var/lib/pacman" -Q woff2-font-awesome >/dev/null 2>&1; then
    sudo pacman --root "$art/sistema-a" --dbpath "$art/sistema-a/var/lib/pacman" -R --noconfirm woff2-font-awesome
fi
fi
{ find "$lab/rootfs/bin" "$lab/rootfs/desktop" "$lab/rootfs/fuentes" -maxdepth 1 -type f -print0 | sort -z | xargs -0 sha256sum;
  sha256sum "$lab/actualizar-escritorio.sh"; } > fuentes-escritorio.sha256
sudo install -m 0644 "$lab/rootfs/desktop/"* "$art/sistema-a/etc/ultra/"
sudo install -m 0644 fuentes-escritorio.sha256 "$art/sistema-a/etc/ultra/escritorio-fuentes.sha256"
sudo install -m 0755 "$lab/rootfs/bin/ultra-ambiente" "$art/sistema-a/usr/local/bin/ultra-ambiente"
sudo install -m 0755 "$lab/rootfs/bin/ultra-hyprland" "$art/sistema-a/usr/local/libexec/ultra-hyprland"
for helper in ultra-preparar-windows ultra-preparar-warframe ultra-preparar-bibliotecas ultra-bibliotecas.py ultra-iniciar-escritorio ultra-crear-fondo ultra-preparar-limites; do
    sudo install -m 0755 "$lab/rootfs/bin/$helper" "$art/sistema-a/usr/local/libexec/$helper"
done
for helper in ultra-graficos ultra-windows ultra-steam-warframe ultra-bibliotecas ultra-pantallas ultra-discos ultra-personalizar ultra-aplicaciones ultra-espacio ultra-incidente ultra-captura ultra-portapapeles ultra-modo-juego; do
    sudo install -m 0755 "$lab/rootfs/bin/$helper" "$art/sistema-a/usr/local/bin/$helper"
done
for helper in "$lab/rootfs/bin/"*; do
    name=${helper##*/}
    for dir in usr/local/bin usr/local/libexec; do
        [[ ! -f $art/sistema-a/$dir/$name ]] || sudo install -m 0755 "$helper" "$art/sistema-a/$dir/$name"
    done
done
sudo install -m 0644 "$lab/rootfs/desktop/"*.desktop "$art/sistema-a/usr/share/applications/"
# Tema Neon: fuentes OFL del proyecto, iconos y cursores recoloreados, cache de fuentes.
sudo install -d -m 0755 "$art/sistema-a/usr/share/fonts/ultra-neon" "$art/sistema-a/usr/share/licenses/ultra-neon"
sudo install -m 0644 "$lab/rootfs/fuentes/"*.ttf "$art/sistema-a/usr/share/fonts/ultra-neon/"
sudo install -m 0644 "$lab/rootfs/fuentes/"OFL-*.txt "$art/sistema-a/usr/share/licenses/ultra-neon/"
sudo pacman -S --noconfirm --needed librsvg xorg-xcursorgen > /dev/null
sudo python "$lab/rootfs/crear-tema.py" "$art/sistema-a"
sudo chroot "$art/sistema-a" /usr/bin/fc-cache -f > /dev/null
if [[ -d $art/sistema-a/etc/wireplumber/wireplumber.conf.d ]]; then
    sudo install -m 0644 "$lab/rootfs/desktop/wireplumber-usb.conf" "$art/sistema-a/etc/wireplumber/wireplumber.conf.d/51-ultra-usb.conf"
fi
for pair in foot:foot.ini fuzzel:fuzzel.ini gtk-3.0:gtk.css gtk-3.0:gtk-settings.ini; do
    dir=${pair%:*}; file=${pair#*:}; target=$file
    [[ $file != gtk-settings.ini ]] || target=settings.ini
    sudo install -m 0644 "$lab/rootfs/desktop/$file" "$art/sistema-a/home/ultra/.config/$dir/$target"
done
sudo chown -R 1000:1000 "$art/sistema-a/home/ultra/.config"
sudo pacman --root "$art/sistema-a" --dbpath "$art/sistema-a/var/lib/pacman" -Q > sistema-paquetes.txt
sudo install -m 0644 sistema-paquetes.txt "$art/sistema-a/etc/ultra/sistema-paquetes.txt"
sudo mkfs.erofs --quiet -zlzma,level=6 -T 0 sistema-nuevo.erofs "$art/sistema-a"
sudo chown "$(id -u):$(id -g)" sistema-nuevo.erofs
bytes=$(stat -c %s sistema-nuevo.erofs)
export MTOOLS_SKIP_CHECK=1
for disk in ultra-esp.img disco-bmo.img disco-bmo-doble.img; do
    [[ -f $disk ]] || continue
    table=$(sfdisk --dump "$disk")
    # All three laboratory layouts put the system/BMO partition second.
    line=$(awk '$1 ~ /2$/ && $2 == ":" {print}' <<< "$table")
    offset=$(sed -n 's/.*start= *\([0-9]*\),.*/\1/p' <<< "$line")
    sectors=$(sed -n 's/.*size= *\([0-9]*\),.*/\1/p' <<< "$line")
    [[ $offset =~ ^[0-9]+$ && $sectors =~ ^[0-9]+$ ]] || exit 1
    if [[ $disk == ultra-esp.img ]]; then
        ((bytes <= sectors*512)) || { echo 'ROJO: ampliar disco mediante construccion completa'; exit 1; }
        dd if=sistema-nuevo.erofs of="$disk" bs=512 seek="$offset" conv=notrunc status=none
    else
        mcopy -o -i "$disk@@$((offset*512))" sistema-nuevo.erofs "::/ULTRA/sistema-$release.erofs"
        [[ $(mtype -i "$disk@@$((offset*512))" "::/ULTRA/sistema-$release.erofs" | sha256sum | cut -d' ' -f1) == $(sha256sum sistema-nuevo.erofs | cut -d' ' -f1) ]]
        if [[ $disk == disco-bmo-doble.img && $perfil == pc ]]; then
            gen="$base/artifacts/generico"
            [[ -f $gen/build-complete ]]
            grel=$(cat "$gen/kernelrelease.txt")
            [[ $grel =~ ^[0-9A-Za-z.+_-]+$ ]]
            mcopy -o -i "$disk@@$((offset*512))" "$gen/sistema-a.erofs" "::/ULTRA/sistema-$grel.erofs"
            [[ $(mtype -i "$disk@@$((offset*512))" "::/ULTRA/sistema-$grel.erofs" | sha256sum | cut -d' ' -f1) == $(sha256sum "$gen/sistema-a.erofs" | cut -d' ' -f1) ]]
        fi
    fi
done
mv -f sistema-nuevo.erofs sistema-a.erofs
[[ $(sha256sum bzImage-tiny64) == "$kernel_before" ]]
mapfile -t paths < <(awk '{print $2}' SHA256SUMS)
if ! printf '%s\n' "${paths[@]}" | grep -qx fuentes-escritorio.sha256; then paths+=(fuentes-escritorio.sha256); fi
sha256sum "${paths[@]}" > SHA256SUMS.nuevo
mv -f SHA256SUMS.nuevo SHA256SUMS
sha256sum --check --status SHA256SUMS
touch build-complete
echo "VERDE: escritorio $perfil actualizado; kernel conservado; hashes verificados; arranque pendiente"
