# ULTRA Carpetas

2026-10-24, America/Lima. Idea y diseño: **Eddi Salazar**.

![ULTRA Carpetas con la rejilla activa](maquetas/png/neon/ultra-carpetas.png)

Las capturas son del programa real, dibujado con Vulkan por software (lavapipe de
Mesa) en una pantalla virtual X11 del entorno de desarrollo, no en Hyprland. Con
un ratón y un teclado simulados se hizo esto:

1. Se activó la rejilla desde la paleta.
2. Se arrastró `captura.png`, que encajó en su celda.
3. Una banda seleccionó 13 iconos.

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
| **Ctrl+Shift+P** | Paleta de comandos |
| **Ctrl+,** | Ajustes y temas: 9 temas con vista previa ([guía](TEMAS-APPS.md)) |
| **T** | Siguiente tema |
| Alt+Izquierda | Volver a la carpeta anterior |

Se abre con **Super+Ctrl+E**, en **Super+Espacio > Aplicaciones > Carpetas BMO-X** o
con `ultra-carpetas [carpeta]`. Thunar sigue en Super+E para copiar, mover o borrar.

### Paleta de comandos (como VS Code)

![Paleta de comandos](maquetas/png/neon/ultra-carpetas-paleta.png)

**Ctrl+Shift+P** abre la paleta. Escribe unas letras en orden ("rej" encuentra
*Alternar rejilla*, "ordtam" encuentra *Ordenar por tamaño*), muévete con las
flechas y ejecuta con Enter; Esc cierra. Busca también por id (`carpetas.candado`).
Cada botón, atajo y fila de la paleta es el mismo **comando**: una sola tabla
(`COMANDOS` en el núcleo) con id, título y atajo.

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

## Cómo está hecho: como React, con el núcleo listo para BMO-X

```text
  evento (clic, tecla, arrastre)
       │  Linux lo traduce
       ▼
    Accion ──► reducir(&mut Estado, Accion) ──► Vec<Efecto>   (guardar, abrir, entrar, releer)
                      │                              │
                      ▼                              ▼
               vista(&Estado) ──► Vista       Linux los ejecuta (disco, xdg-open)
               (lo que hay que dibujar)        y dibuja la Vista con egui/wgpu
```

- **Como React + Redux:** el `Estado` es la única verdad. Todo lo que pasa es
  una `Accion`, y `reducir` calcula el estado nuevo sin tocar disco ni pantalla.
  Lo que hay que hacer fuera (guardar el orden, abrir un archivo, entrar en una
  carpeta) sale como `Efecto`. `vista` describe qué dibujar, como un render de
  React, pero como una lista de nodos y no como píxeles.
- **Como VS Code:** comandos con id, título y atajo; una paleta con búsqueda difusa;
  y un mapa de teclas (`atajo`) igual para cualquier plataforma.
- **Núcleo `no_std`** (`src/apps/carpetas-nucleo/`): solo `core` y `alloc`, sin
  archivos, hilos, reloj ni GPU, y sin `unsafe` (`#![forbid(unsafe_code)]`).
  Compila para **`x86_64-unknown-none`**, el objetivo bare-metal. En BMO-X solo
  hay que escribir la capa fina: leer su sistema de archivos, ejecutar los
  `Efecto` y pintar los `Nodo` en su framebuffer.
- **Capa Linux** (`src/apps/ultra-carpetas/`): lee carpetas, guarda el JSON, abre
  con `xdg-open` y dibuja con egui sobre wgpu. Es un programa normal, no va
  dentro del kernel.
- **egui** dibuja la interfaz y **wgpu** habla con la GPU: **Vulkan** en Linux y
  Windows, Metal en macOS. Es el mismo código en todas partes, en Rust seguro y
  sin Vulkan crudo.
- **Pruebas:** 9 en el núcleo, al estilo Redux (acción → estado + efectos): arrastre,
  rejilla, candado, deshacer, banda, abrir, paleta, JSON raro y vista. Hay 2 más en
  la capa Linux (listar carpetas y guardar en disco).
- Toma tu acento de Personalizar y la letra Chakra Petch si está instalada. Cada
  tipo de archivo tiene su color: imágenes violeta, video rojo, audio amarillo,
  código cyan, comprimidos coral, programas verde.

## Build

`scripts/instalar-carpetas.sh`:

1. prueba el núcleo y lo compila para `x86_64-unknown-none`: si deja de ser
   `no_std`, el build falla en rojo;
2. compila la app con Rust 1.97.0 (fijado en `rust-toolchain.toml`) y `Cargo.lock`
   (`--locked`), y pasa sus pruebas;
3. instala `/usr/local/bin/ultra-carpetas`;
4. comprueba con `ldd` que no falta ninguna biblioteca en la imagen.

Si no hay Rust o no compila, avisa en amarillo y sigue Thunar. Corre desde
`build-baseline.sh` y desde `actualizar-escritorio.sh`.

## Pendiente

- Probarlo en Hyprland real con NVIDIA (aquí se probó con Vulkan por software).
- Miniaturas de imágenes, renombrar y mover archivos de verdad (con confirmación y
  deshacer), varias ventanas y arrastrar entre carpetas.
