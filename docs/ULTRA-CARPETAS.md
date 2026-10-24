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
| **Ctrl+,** | Ajustes: temas con vista previa ([guía](TEMAS-APPS.md)), iconos y dirección del grafo |
| **Tab** / **Ctrl+B** | Iconos ↔ grafo / mostrar u ocultar el explorador |
| **D**, **Inicio**, **I**, **F12** | Blender ↔ Houdini, encuadrar, estilo de iconos, carriles |
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

## Tres modos que se combinan: explorador, iconos y grafo

![Grafo estilo Blender con explorador e iconos Material](maquetas/png/neon/carpetas-grafo-blender.png)

*Captura real: tema TITAN Taller, iconos Material, grafo Blender con `assets` y `src`
desplegados con doble clic. El explorador de la izquierda se abrió solo.*

- **Explorador** (Ctrl+B): árbol lateral como el de VS Code y el de tu TITAN. El
  triángulo despliega, el clic selecciona y el doble clic abre. Comparte las carpetas
  desplegadas y la selección con el grafo: lo que abres en uno se abre en el otro.
- **Iconos** (por defecto): iconos libres donde tú los dejes, con rejilla y candado.
- **Grafo** (Tab): cada carpeta y archivo es un nodo, como en Blender y Houdini.
  - **Dirección** (D o Ajustes): **Blender**, de izquierda a derecha, o **Houdini**, de
    arriba abajo. Al cambiarla se vuelve al acomodo automático.
  - **Nodos:** cabecera del color de su tipo (el tema decide), conectores de entrada y
    salida, y cables curvos como los *noodles* de Blender. El botón `+`/`-` o el doble
    clic despliega la carpeta en el propio grafo.
  - **Peso:** cada nodo dice cuánto ocupa (sumando lo ya leído debajo). Su barra es la
    parte de su carpeta que ocupa; en rojo, si pasa del 60 %. Así se ve de un vistazo
    qué conviene limpiar u optimizar.
  - **Ratón:** la rueda acerca y aleja sobre el puntero; el botón central o el derecho
    mueve la vista; arrastrar un nodo lo fija a mano (Ctrl+Z deshace); arrastrar sobre el
    fondo hace una banda de selección.
  - **Teclas:** Inicio encuadra todo y Reacomodar vuelve al acomodo automático.
- **En tiempo real:** la carpeta abierta y las desplegadas se vigilan (una vez por
  segundo, en el hilo del disco). Lo nuevo aparece solo y lo borrado se va.

![Grafo estilo Houdini con iconos Lucide y el semáforo de carriles](maquetas/png/neon/carpetas-grafo-houdini.png)

## Iconos (I o Ajustes)

| Estilo | Qué es |
| --- | --- |
| Neón | Dibujados con formas, como hasta ahora |
| **Material** | Los iconos de archivos de VS Code ([Material Icon Theme](https://github.com/material-extensions/vscode-material-icon-theme), MIT): logos de Rust, C, Python, TOML…, y carpetas especiales (`src`, `docs`, Imágenes, Música, Descargas…) |
| **Lucide** | Iconos de líneas ([Lucide](https://github.com/lucide-icons/lucide), ISC) teñidos con los colores del tema |

Son 78 SVG incrustados en el programa, con sus licencias en
`src/apps/ultra-carpetas/iconos/`. El programa nunca abre un SVG de fuera, así que
el lector SVG (resvg) solo ve archivos conocidos. Una prueba comprueba que ninguno
lleva scripts, enlaces externos ni imágenes incrustadas, y otra que cada icono que
pide el núcleo existe.

## Carriles con semáforo (F12)

Cada parte del programa va en su carril, con presupuesto y límites, para que ninguna
choque con otra ni haya que pelear después con la optimización. F12 muestra el
último tiempo y el máximo de cada carril con su luz.

| Carril | Verde hasta | Amarillo hasta | Qué consume y cómo se contiene |
| --- | --- | --- | --- |
| **Disco** | 20 ms | 200 ms | E/S, en **su propio hilo**: la interfaz nunca espera al disco. Respuestas viejas (de otra carpeta) se descartan por generación |
| **Acomodo** | 1 ms | 4 ms | CPU, O(n log n) con n ≤ 600 nodos; solo se calcula si cambió el estado |
| **Vista** | 1,5 ms | 5 ms | CPU y memoria: en caché por versión del estado |
| **Pintado** | 6 ms | 12 ms | CPU (egui): solo se pinta lo que cae en pantalla; con poco zoom se omiten textos e iconos; solo se repinta con eventos |
| **Iconos SVG** | 8 ms | 30 ms | CPU una vez por icono y tamaño; después es textura en la GPU |

**Límites duros** (por diseño, no por suerte):

- 5000 entradas por carpeta;
- 600 nodos en el grafo;
- 64 carpetas desplegadas, que son las que se vigilan;
- 1 MiB por archivo de orden y 64 KiB de ajustes.

**Seguridad en el carril de disco:**

- Las rutas son relativas a la carpeta abierta y se validan dos veces: el núcleo solo
  pide lo que leyó, y el hilo rechaza tramos vacíos, `.`, `..`, rutas absolutas y enlaces.
- No se despliega a través de un enlace, para no salir de la carpeta ni entrar en bucles.
- Nada pasa por una shell.

Las primeras medidas en la pantalla virtual (no en tu PC) quedaron todas en verde:

- disco, unos 65 µs;
- acomodo, unos 40 µs;
- vista, unos 70 µs;
- pintado, unos 0,5 ms (el primer cuadro, con texturas, unos 7 ms).

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
