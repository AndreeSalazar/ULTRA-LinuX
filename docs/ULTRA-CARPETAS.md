# ULTRA Carpetas

2026-10-24, America/Lima. Idea y diseño: **Eddi Salazar**.

![ULTRA Carpetas con la rejilla activa](maquetas/png/neon/ultra-carpetas.png)

La captura es del programa real, dibujado con Vulkan por software (lavapipe de
Mesa) en una pantalla virtual X11 del entorno de desarrollo, no en Hyprland.
`captura.png` y `kernel.rs` se movieron con un ratón simulado; con la rejilla
activa, `kernel.rs` encajó en su celda.

Gestor de carpetas inspirado en el escritorio del Windows antiguo: **cada icono va
donde tú lo dejes y cada carpeta recuerda su orden.** Con control:

| Tecla o botón | Qué hace |
| --- | --- |
| Arrastrar | Mueve el icono, o todos los seleccionados |
| Arrastrar sobre el fondo | Banda de selección; Ctrl suma a la selección |
| Doble clic / Enter | Abre la carpeta, o el archivo con su programa (`xdg-open`) |
| **G** — Rejilla | Muestra puntos y, al soltar, encaja cada icono en su celda |
| **L** — Candado | Nada se mueve hasta quitarlo |
| **Ctrl+Z** — Deshacer | Vuelve atrás los movimientos (hasta 64) |
| Ordenar | Por nombre, tipo o tamaño, en rejilla (se puede deshacer) |
| **H** — Ocultos | Muestra u oculta los archivos que empiezan por punto |
| Retroceso, Atrás, Arriba y la ruta | Navegar; cada parte de la ruta es un botón |
| Ctrl+A, Esc, F5 | Seleccionar todo, quitar la selección, volver a leer |

Se abre con **Super+Ctrl+E**, en **Super+Espacio > Aplicaciones > Carpetas BMO-X** o
con `ultra-carpetas [carpeta]`. Thunar sigue en Super+E para copiar, mover o borrar.

## Seguridad

- **Solo ordena la vista.** No mueve, renombra ni borra archivos. Lo único que
  escribe es el orden, en `~/.config/ultra/carpetas/disposicion/<huella>.json`
  (un archivo por carpeta, permisos 0600, escritura atómica).
- Un orden dañado, enorme (más de 1 MiB), enlazado o con números absurdos (NaN,
  negativos, gigantes) se ignora y los iconos se recolocan solos.
- Lee las carpetas sin seguir enlaces: un enlace roto se muestra como enlace, y uno
  a una carpeta se abre como carpeta. Lee como mucho 5000 entradas por carpeta.
- Si algo aparece o desaparece en la carpeta, lo nuevo va a la primera celda libre
  sin mover lo que ya ordenaste, y lo borrado se olvida.

## Cómo está hecho

- **Rust**, en `src/apps/ultra-carpetas/`. Es un programa normal de tu sistema, no
  va dentro del kernel: si falla, no se cae nada más.
- **egui** dibuja la interfaz y **wgpu** habla con la GPU: **Vulkan** en Linux y
  Windows, Metal en macOS. Es el mismo código en todas partes, en Rust seguro y
  sin Vulkan crudo.
- `disposicion.rs` es la lógica pura (dónde va cada icono, rejilla, candado,
  deshacer, guardar) y `entradas.rs` lee la carpeta. Los dos tienen pruebas:
  `cargo test` en la carpeta del programa (5 pruebas).
- Toma tu acento de Personalizar y la letra Chakra Petch si está instalada. Cada
  tipo de archivo tiene su color: imágenes violeta, video rojo, audio amarillo,
  código cyan, comprimidos coral, programas verde.

## Build

`scripts/instalar-carpetas.sh`:

1. compila con Rust 1.97.0 (fijado en `rust-toolchain.toml`) y `Cargo.lock`
   (`--locked`);
2. pasa las pruebas;
3. instala `/usr/local/bin/ultra-carpetas`;
4. comprueba con `ldd` que no falta ninguna biblioteca en la imagen.

Si no hay Rust o no compila, avisa en amarillo y sigue Thunar. Corre desde
`build-baseline.sh` y desde `actualizar-escritorio.sh`.

## Pendiente

- Probarlo en Hyprland real con NVIDIA (aquí se probó con Vulkan por software).
- Miniaturas de imágenes, renombrar y mover archivos de verdad (con confirmación y
  deshacer), varias ventanas y arrastrar entre carpetas.
