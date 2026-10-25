#!/usr/bin/bash
# SPDX-License-Identifier: GPL-2.0-only
# Integra el escritorio actual en una copia extraida de EROFS dentro del taller.
# El llamador prepara artefactos y caches en un disco virtual; nunca usa /juegos.
set -euo pipefail
perfil=${1:?pc o generico}; revision=${2:?identificador de entrega}
[[ $perfil == pc || $perfil == generico ]]
[[ $revision =~ ^[a-z0-9][a-z0-9-]{0,31}$ ]]
base="$HOME/linux-reconstruido"; art="$base/artifacts/$perfil"
lab=$(cd -- "$(dirname -- "$0")" && pwd); stage="$art/sistema-nativo"
[[ -f $stage/ULTRA-SISTEMA && -f $art/pacman-sistema.conf ]]
: "${ULTRA_BUILD_CACHE:?cache de compilacion del taller}"
: "${CARGO_HOME:?cache Cargo del taller}"
: "${RUSTUP_HOME:?toolchains del taller}"
export CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-2}
sudo pacman --root "$stage" --dbpath "$stage/var/lib/pacman" -Q nvidia-utils lib32-nvidia-utils > "$art/main-nvidia-antes.txt" 2>/dev/null || [[ $perfil != pc ]]
# Los proyectos C/C++ necesitan las cabeceras que la imagen reducida omitía.
sed 's@usr/include/\*@@g' "$art/pacman-sistema.conf" > "$art/pacman-main.conf"
runtime=(rustup gcc git pkgconf gtk3 gtk-layer-shell libdbusmenu-gtk3 python-pillow vulkan-swrast)
sudo pacman --config "$art/pacman-main.conf" --root "$stage" --dbpath "$stage/var/lib/pacman" \
    -S --needed --noconfirm --ignore nvidia-utils,lib32-nvidia-utils "${runtime[@]}" > "$art/main-runtime.log" 2>&1
sudo pacman --config "$art/pacman-main.conf" --root "$stage" --dbpath "$stage/var/lib/pacman" \
    -S --noconfirm --ignore nvidia-utils,lib32-nvidia-utils glibc gcc linux-api-headers >> "$art/main-runtime.log" 2>&1
# La reinstalacion de glibc puede regenerar los locales. Conservar Peru y
# comprobar el resultado, no solo la variable LC_TIME del escritorio.
if ! grep -qx 'es_PE.UTF-8 UTF-8' "$stage/etc/locale.gen"; then
    printf '\nes_PE.UTF-8 UTF-8\n' | sudo tee -a "$stage/etc/locale.gen" > /dev/null
fi
sudo localedef --prefix="$stage" -i es_PE -f UTF-8 es_PE.UTF-8
sudo chroot "$stage" /usr/bin/env LC_ALL=es_PE.UTF-8 /usr/bin/locale charmap | grep -qx UTF-8
sudo chroot "$stage" /usr/bin/env LC_TIME=es_PE.UTF-8 /usr/bin/date -d @0 '+%B' | grep -qx diciembre
motor=
for package in awww swww; do
    if sudo pacman --config "$art/pacman-main.conf" --root "$stage" --dbpath "$stage/var/lib/pacman" \
        -S --needed --noconfirm "$package" >> "$art/main-runtime.log" 2>&1; then motor=$package; break; fi
done
[[ -n $motor ]] || { echo 'ROJO: no se pudo instalar motor de fondo vivo'; exit 1; }
echo "VERDE [MAIN]: dependencias y fondo $motor"
bash "$lab/instalar-eww.sh" "$stage" "$art" "$base"
[[ -x $stage/usr/local/bin/eww ]] || { echo 'ROJO: eww no construido'; exit 1; }
bash "$lab/instalar-carpetas.sh" "$stage" "$art" "$base"
[[ -x $stage/usr/local/bin/ultra-carpetas ]] || { echo 'ROJO: ULTRA Carpetas no construido'; exit 1; }
sudo install -d "$stage/etc/ultra" "$stage/usr/share/applications"
find "$lab/rootfs/desktop" -maxdepth 1 -type f -exec sudo install -m 0644 -t "$stage/etc/ultra" {} +
find "$lab/rootfs/desktop" -maxdepth 1 -name '*.desktop' -type f -exec sudo install -m 0644 -t "$stage/usr/share/applications" {} +
for source in "$lab/rootfs/bin/"*; do
    name=${source##*/}
    for directory in usr/local/bin usr/local/libexec; do
        [[ ! -f $stage/$directory/$name ]] || sudo install -m 0755 "$source" "$stage/$directory/$name"
    done
done
for helper in ultra-fondo-vivo ultra-tarjetas; do
    sudo install -m 0755 "$lab/rootfs/bin/$helper" "$stage/usr/local/bin/$helper"
done
sudo install -d "$stage/usr/share/fonts/ultra-neon" "$stage/usr/share/licenses/ultra-neon"
sudo install -m 0644 "$lab/rootfs/fuentes/"*.ttf "$stage/usr/share/fonts/ultra-neon/"
sudo install -m 0644 "$lab/rootfs/fuentes/"OFL-*.txt "$stage/usr/share/licenses/ultra-neon/"
sudo python "$lab/rootfs/crear-tema.py" "$stage"
sudo chroot "$stage" /usr/bin/fc-cache -f
sudo pacman --root "$stage" --dbpath "$stage/var/lib/pacman" -Q > "$art/sistema-paquetes.txt"
sudo install -m 0644 "$art/sistema-paquetes.txt" "$stage/etc/ultra/sistema-paquetes.txt"
sudo pacman --root "$stage" --dbpath "$stage/var/lib/pacman" -Q nvidia-utils lib32-nvidia-utils > "$art/main-nvidia-despues.txt" 2>/dev/null || [[ $perfil != pc ]]
cmp "$art/main-nvidia-antes.txt" "$art/main-nvidia-despues.txt"
for test in probar_personalizar probar_modo_juego probar_fondo_vivo probar_tarjetas probar_registro_steam probar_lanzador_cyberpunk; do
    # Los bancos buscan src/rootfs; el taller usa lab/rootfs para empaquetar.
    [[ -d $base/src/rootfs ]] || ln -s ../lab/rootfs "$base/src/rootfs"
    [[ $(readlink -f "$base/src/rootfs") == "$lab/rootfs" ]] || { echo 'ROJO: banco apunta a otras fuentes rootfs'; exit 1; }
    python "$lab/$test.py" > "$art/main-$test.log" 2>&1
done
{ find "$lab/rootfs" "$base/src/apps" -type f ! -path '*/target/*' ! -path '*/__pycache__/*' -print0 | sort -z | xargs -0 sha256sum;
  sha256sum "$lab/actualizar-main.sh" "$lab/instalar-eww.sh" "$lab/instalar-carpetas.sh"; } > "$art/main-integracion-fuentes.sha256"
sudo install -m 0644 "$art/main-integracion-fuentes.sha256" "$stage/etc/ultra/main-integracion-fuentes.sha256"
echo 'VERDE [MAIN]: aplicaciones, pruebas y fuentes listas; empaquetando'
bash "$lab/actualizar-almacen-nativo.sh" "$perfil" "$revision"
