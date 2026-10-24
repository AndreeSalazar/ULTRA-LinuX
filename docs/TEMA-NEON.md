# BMO-X / Neón (acento hielo)

2026-10-24, America/Lima. Diseño y autoría: **Eddi Salazar**.

**Estado:** el tema está en el código del escritorio. Falta reconstruir las
imágenes y pasar las pruebas de arranque en QEMU y en el PC. Hasta entonces, las
imágenes y el video de Neón vienen del [prototipo](maquetas/neon/vivo.html) y no
son capturas del sistema.

![Recorrido animado de Neón](maquetas/png/neon/recorrido.gif)

## Colores

| Uso | Color |
| --- | --- |
| Fondo y paneles | `#06050a`, `#0f0e16`, líneas `#2a2733` / `#3b3747` |
| Acento principal (foco, acciones) | hielo `#5ad8ff`, claro `#bff3ff`, oscuro `#2b9fd0` |
| Datos (etiquetas de la barra, CPU en MangoHud) | cyan `#00f0ff` |
| Alertas, apagar, avisos críticos | rojo `#ff2a4f` |
| Texto | `#ece9dc`, secundario `#a19eae`, tenue `#625f70` |
| Acento alternativo | amarillo `#fcee0a` (categoría Ambiente) |

Categorías del centro BMO-X (Super+Espacio):

| Categoría | Color |
| --- | --- |
| Aplicaciones | hielo |
| Discos | cyan |
| Juegos | rojo |
| Pantalla | verde `#3dff9a` |
| Audio | violeta `#c03bff` |
| Ambiente | amarillo |
| Sistema | coral `#ff7a90` |

En cada categoría, la fila elegida se pinta con su color y el texto pasa a negro.

## Qué cambia en cada archivo

| Archivo | Cambio |
| --- | --- |
| `desktop/hyprland.lua` | `rounding = 0`; borde hielo → rojo → cyan; brillo hielo en la ventana activa; animaciones (abajo). Atajos sin cambios |
| `desktop/waybar.jsonc` | Barra pegada arriba; etiquetas CPU, RAM, NET y VOL; escritorios `01`–`05`; título de ventana a 40 caracteres; módulo **EDDI**; alerta de CPU al 80 % |
| `desktop/waybar.css` | Bloque BMO-X inclinado (degradado, GTK no tiene `clip-path`); escritorio activo relleno de hielo; apagar en rojo; dock negro con borde hielo |
| `desktop/fuzzel.ini`, `bin/ultra-ambiente` | Menús negros, borde y selección del color de la categoría, coincidencias en rojo, JetBrains Mono |
| `desktop/mako.conf` | Avisos negros con borde hielo, sin redondeo, Chakra Petch; críticos en rojo |
| `desktop/gtk.css`, `gtk-settings.ini` | GTK negro y hielo, sin redondeo, Chakra Petch |
| `desktop/foot.ini` | JetBrains Mono y paleta Neón |
| `desktop/MangoHud.conf` | Colores Neón; métricas, atajos y registro sin cambios |
| `desktop/wallpaper.svg` | Ciudad nocturna original con el aro del autor, de [`crear-fondo.py`](maquetas/neon/crear-fondo.py) |
| `desktop/AUTOR.txt`, `desktop/emblema-autor.svg` | Perfil del autor; el módulo EDDI de la barra abre `AUTOR.txt` |
| `crear-tema.py` | Iconos y cursores recoloreados; se puede ejecutar otra vez sobre una imagen existente. El nombre interno `ULTRA-Nucleo` se conserva por compatibilidad |
| `fuentes/` | Rajdhani y Chakra Petch con sus licencias OFL |
| `bin/ultra-tema` | Migración `neon-1` |
| `scripts/build-baseline.sh`, `scripts/actualizar-escritorio.sh` | `ttf-jetbrains-mono`, fuentes en `/usr/share/fonts/ultra-neon`, `fc-cache`, iconos y cursores también en la actualización rápida |

La migración `neon-1` funciona igual que `nucleo-1`, en el primer inicio:

- guarda los foot, fuzzel y GTK que ya tenga el usuario en
  `~/.config/ultra/tema-respaldo-neon-1/`;
- instala los nuevos;
- no vuelve a pisar los cambios que el usuario haga después.

## Personalizar (Super+Shift+P)

`ultra-personalizar` cambia el escritorio sin root. Se abre con **Super+Shift+P**,
con **Super+Espacio > Ambiente > Personalizar escritorio** o con clic derecho en
la luna de la barra.

| Opción | Valores |
| --- | --- |
| Acento | hielo, amarillo, cyan, rojo, verde, violeta, coral, blanco o un hex propio (`#RRGGBB`) |
| Fondo | Neón con el acento elegido, **Demonio** (tu Tux dentro del aro sobre la ciudad en rojo), Núcleo clásico, imagen propia (PNG, JPG o SVG de `~/Imágenes` o `~/Pictures`) o color liso |
| Efectos | vivos o reducidos |
| Borde de la ventana activa | gira o quieto |
| Huecos entre ventanas | compacto, normal o amplio |
| Grosor del borde | 1, 2 o 3 px |
| Tamaño del cursor | 24, 28, 32 o 48 px |
| Tamaño de la barra | compacta, normal o grande |
| Opacidad de ventanas inactivas | 100 %, 92 % u 85 % |
| Esquinas | rectas, suaves (6 px) o redondas (12 px) |
| Animación al abrir | emergente, deslizar o zoom |
| Velocidad de animaciones | rápida, normal o lenta |
| Desenfoque | ligero, fuerte o no |
| Posición de la barra | arriba o abajo (con la barra abajo, el dock sube) |
| Dock | visible u oculto |
| Reloj | 24 h, 12 h o con segundos |
| Posición de avisos | arriba a la derecha, arriba al centro o abajo a la derecha; esquivan barra y dock |
| Letra de la interfaz | Neón (Rajdhani y Chakra Petch), Chakra Petch, mono (JetBrains Mono) o clásica (DejaVu Sans) |

### Perfiles de un toque

**Super+Shift+P > Perfiles** cambia muchas opciones a la vez:

| Perfil | Qué cambia |
| --- | --- |
| Neón hielo | Todo al tema de fábrica |
| Demonio | Acento rojo, fondo Demonio, borde que gira, inactivas al 92 % |
| Sobrio | Efectos reducidos, borde quieto, sin desenfoque, animaciones rápidas |
| Minimal | Sin dock, barra compacta, huecos compactos, borde de 1 px |
| Juego | Efectos reducidos, borde quieto, sin desenfoque ni dock, avisos arriba al centro |

**Guardar el actual como…** crea un perfil tuyo en `~/.config/ultra/perfiles/`
(minúsculas, números y guiones, hasta 24 caracteres y 16 perfiles). Se valida
igual que `personal.json`: un perfil dañado vuelve a valores seguros. Por
terminal: `ultra-personalizar perfil aplicar|guardar|borrar NOMBRE`.

El fondo Demonio sale de `scripts/crear_fondo_demonio.py`, que genera
`src/rootfs/desktop/wallpaper-demonio.png` a partir de tu Tux.

El menú se agrupa en Perfiles y cinco apartados: Colores y fondo, Ventanas, Animaciones y
efectos, Barra y dock, y Avisos, letra y cursor.

Super+Shift+A sigue alternando los efectos. Con personalización guardada,
`ultra-ambiente` vuelve a aplicar tu desenfoque, esquinas y animaciones para el
modo elegido, en vez de pisarlos.

Cómo funciona:

- Las elecciones van a `~/.config/ultra/personal.json`.
- A partir de ellas se generan copias del tema en `~/.config/ultra/tema-personal/`:
  Waybar, mako, fuzzel y el fondo.
- Hyprland recibe los cambios en vivo con `hyprctl eval`; la barra, los avisos y
  el fondo se reinician solos.
- Al iniciar sesión, `ultra-iniciar-escritorio` vuelve a aplicar lo guardado.
- **Sin elecciones guardadas no se genera nada**: el escritorio usa `/etc/ultra`
  tal cual y las pruebas de arranque no cambian.
- Un `personal.json` dañado o editado a mano vuelve a valores seguros. Las imágenes
  deben ser archivos propios, no enlaces y de menos de 64 MiB.
- **Restablecer todo** borra solo esos archivos y vuelve al tema del sistema.

`scripts/probar_personalizar.py` prueba todo esto en un HOME temporal, con el tema
real y comandos de sesión falsos: generación, aplicación, rechazos, valores
dañados, menú y restablecimiento. El [prototipo](maquetas/neon/vivo.html) tiene el
mismo panel (Alt+U) para ver cada opción antes de construir la imagen.

## Capturas de pantalla

| Atajo | Captura |
| --- | --- |
| Impr Pant | Pantalla activa |
| Super+Shift+S | Región; el recuadro usa tu acento y Esc cancela |
| Super+Impr Pant | Ventana activa |

`ultra-captura` (grim, slurp y wl-clipboard) guarda en `~/Imágenes/Capturas/`,
copia la imagen al portapapeles y avisa con la miniatura. Valida la geometría
que devuelve `slurp` antes de usarla. `scripts/probar_captura.py` lo prueba con
herramientas falsas.

## Rust

La imagen incluye `rustup`, `gcc` (Rust lo usa para enlazar), `git` y `pkgconf`.

- **Primera vez:** en una terminal, `rustup default stable`. Descarga unos 300 MB
  al home.
- **Dónde queda:** con almacenamiento persistente se conserva; con el home en RAM
  se pierde al apagar.
- **Versión por proyecto:** los proyectos con `rust-toolchain.toml`, como BMO-X,
  eligen su propia versión solos.
- **PATH:** `~/.cargo/bin` ya está incluido para lo que instales con `cargo install`.

## Animaciones del sistema real

| Animación | Configuración |
| --- | --- |
| Ventana nueva y cierre | `windows` popin 90 % (0,4 s) y `windowsOut` popin 94 % (0,2 s), curva `0.16,1,0.3,1` |
| Reacomodo | `windowsMove` 0,35 s |
| Escritorios | `workspaces` slide 0,4 s; `specialWorkspace` slidevert |
| Barra, menús y avisos al aparecer | `layers` slide 0,25 s |
| Borde que gira | `borderangle` en `loop`, 6 s por vuelta |
| Alerta de CPU y escritorio urgente | `@keyframes` de GTK CSS en Waybar |

Super+Shift+A (efectos reducidos) apaga todas las animaciones, el desenfoque y
las sombras, igual que antes.

Lo que solo existe en el prototipo:

- el corte glitch al abrir ventanas;
- la barra de tiempo de los avisos;
- Equipo en vivo con tarjetas (necesitaría eww);
- el fondo animado.

## Para RING 0

- **Borde que gira:** obliga a Hyprland a redibujar mientras haya un borde visible.
  En pantalla completa no hay borde. Si se prefiere un escritorio quieto, basta
  con quitar la línea `borderangle` de `hyprland.lua`.
- **Fondo animado** (lluvia, letreros, aro): no está en el sistema real; sigue
  `swaybg` con el SVG fijo. Activarlo sería decisión de RING 0, pausándolo en
  juegos y en pantalla completa.

## Verificación pendiente

1. En el taller: `scripts/actualizar-escritorio.sh generico completo`.
2. Arranque en QEMU con las pruebas de escritorio y `--tema-nucleo`. Las pruebas
   se actualizaron así:
   - colores nuevos por categoría, comparados contra una captura sin menú,
     porque el fondo Neón ya contiene esos colores;
   - ruta del respaldo `neon-1`.
3. Comprobar en el registro de Hyprland que acepta `shadow.color_inactive` y las
   animaciones `layers`, `specialWorkspace` y `borderangle`. La configuración
   Lua se ejecutó sin errores con una API simulada, pero solo Hyprland 0.56.2
   valida los nombres.
4. Comprobar en Waybar las etiquetas con formato Pango y la clase `alerta` de la CPU.
5. Repetir en el perfil PC, con capturas y hashes en `evidencias/`.

## Volver al amarillo

```text
python3 docs/maquetas/neon/crear-fondo.py --acento amarillo > src/rootfs/desktop/wallpaper.svg
```

En los demás archivos, cambia `5ad8ff` por `fcee0a`, `2b9fd0` por `c9be08` y
`bff3ff` por `fff36b`.
