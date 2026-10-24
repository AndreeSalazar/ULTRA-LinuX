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
| Ctrl+A, Esc, F5 | Seleccionar todo, quitar la selección, ejecutar (play) o volver a leer |
| **Clic derecho**, **F8** | Editor de código en el error / siguiente error ([en vivo](#en-vivo-procesos-errores-atrapados-en-los-nodos-y-editor-clic-derecho)) |
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

## Play y terminal (como VS Code)

![Botones de play en el grafo, el explorador y la barra](maquetas/png/neon/carpetas-play.png)

Los nodos que arrancan algo llevan un **▶ verde**: en el grafo, en el explorador, en
los iconos y, para la carpeta abierta, en la barra («Ejecutar: cargo run [F5]»).

| Nodo | Qué ejecuta |
| --- | --- |
| Carpeta con `Cargo.toml`, `Cargo.toml` o `src/main.rs` de un proyecto | `cargo run` en la carpeta del proyecto |
| Carpeta con `Makefile`, o el propio `Makefile` | `make` |
| `*.py` | `python3 archivo.py` |
| `*.sh` | `bash archivo.sh` |

**F5** ejecuta lo seleccionado o, si no hay nada, el proyecto abierto.
**Ctrl+`** muestra u oculta la **terminal** de abajo:

- salida en vivo, con lo grave de stderr en rojo y el progreso en ámbar;
- **Detener**, que para también a los hijos (cargo → rustc → tu programa);
- **Limpiar**;
- una línea para escribir comandos (`cargo build`, `git status`, `ls -la`…).

**Reglas de seguridad:**

- Nada arranca solo: siempre hace falta tu clic o tu tecla.
- Los ▶ usan una lista cerrada de programas (`cargo`, `make`, `python3`, `bash`).
- Nada pasa por una shell, tampoco la línea de comandos: son palabras sueltas, sin
  tuberías, `;`, comodines ni sustituciones. `ls ; rm -rf ~` es `ls` con argumentos
  raros, y no borra nada.
- La carpeta de trabajo se valida igual que las del disco: sin `..` ni enlaces.
- El proceso no recibe entrada (stdin cerrado) y corre en su propio grupo.
- Se ve sin colores ANSI (`NO_COLOR`, `CARGO_TERM_COLOR=never`). La terminal limpia
  los códigos de control.
- Un proceso a la vez, como mucho 512 líneas por cuadro y 2000 guardadas.

Esto no es un emulador de terminal completo: no hay programas interactivos (`vim`,
`htop`) ni tuberías. Para eso sigue `foot`.

## En vivo: procesos, errores atrapados en los nodos y editor (clic derecho)

![Un error de rustc marca nave.rs, src y la raíz en rojo; clic derecho abre el editor en la línea 9](maquetas/png/neon/carpetas-error-editor.png)

Las tres capturas son del programa real en la pantalla virtual (Vulkan por software),
con ratón y teclado simulados. Primero se ejecutó `cargo run` en un proyecto con un
error a propósito. Después, clic derecho en el nodo rojo, corregir la línea en el
editor, Ctrl+S y F5. Al final, otro proyecto que gasta CPU, para ver sus procesos.

**Errores atrapados en los nodos, sin buscar números.** Mientras corre el proceso se
lee su salida línea a línea:

- rustc y cargo (`error[E0425]` y su `--> src/nave.rs:9:28`);
- los panics de Rust (`thread 'main' panicked at src/nave.rs:42:13:`);
- gcc y clang (`main.c:12:7: error:`);
- los tracebacks de Python (`File "juego.py", line 8`).

Cada error se pinta en el **nodo de su archivo**:

- anillo rojo en la GPU (amarillo si es solo un aviso);
- una insignia con la cuenta ("2 errores");
- el mismo color y la insignia en el explorador y en el modo iconos;
- las carpetas de encima cuentan lo que tienen dentro, así que el rojo sube hasta
  la raíz;
- si el archivo estaba dentro de una carpeta plegada, el camino **se despliega
  solo** (una vez: si lo vuelves a plegar, se respeta).

Pasa el ratón sobre un nodo para ver el primer mensaje.

| Cómo | Qué hace |
| --- | --- |
| **Clic derecho** en un nodo, fila o icono | Abre el **editor** en la línea del error (en una carpeta, en su primer error) |
| **F8** o el botón rojo de la barra | Va al siguiente error y lo abre en el editor |
| Clic en una línea de la terminal | Las que apuntan a un archivo (`--> src/main.rs:3:5`) son enlaces |
| **Ctrl+S** / **Esc** | Guardar / cerrar el editor (con cambios sin guardar, avisa y hay que pulsar Esc otra vez) |
| Ajustes > Errores | **Parar el proceso en el primer error atrapado** (como «break on error») |

![Arreglado: tras Ctrl+S y F5 compila, queda un aviso amarillo y el programa imprime su salida](maquetas/png/neon/carpetas-arreglado.png)

**El editor** colorea el código con los colores de tu tema: Rust, C/C++/JS/WGSL,
Python, shell y TOML. Muestra números de línea; las líneas con error llevan una
franja roja y un punto, y el mensaje sale al pasar el ratón. Arriba está la lista
de errores del archivo, con clic para saltar. Al volver a ejecutar, la lista se
actualiza sola.

**Reglas del editor:**

- Abre solo archivos que el disco listó, de hasta 2 MiB y solo texto UTF-8 (un byte
  NUL = binario, no se abre).
- Ninguna ruta pasa por un enlace simbólico, ni en las carpetas de encima.
- Se comprueba que el archivo abierto es el que se miró (dispositivo + inodo).
- Guardar es atómico (temporal + `rename`) y conserva los permisos.
- **Si el archivo cambió en disco** desde que lo abriste (otro editor, `git
  checkout`), no lo pisa y te avisa.
- Nunca guarda solo: solo con Ctrl+S o con el botón.

![Procesos en vivo: motor al 99 % de CPU con su hijo sleep, leído de /proc; carriles en verde y 2 subidas a la GPU en 70 cuadros](maquetas/png/neon/carpetas-en-vivo.png)

**Procesos en vivo.** Al darle a play, el nodo que se ejecuta lleva un borde verde y
una tarjeta **EN VIVO**:

- el árbol real de procesos (cargo → rustc → tu programa → sus hijos);
- la CPU de cada uno, donde 100 % es un núcleo lleno y amarillo pasa del 90 %;
- su memoria (RSS).

Lo lee de `/proc` un hilo propio cada 500 ms. Solo mira el grupo de procesos del
play (el mismo que para «Detener»), y como mucho 64 procesos. No es un monitor de
todo el sistema: es tu programa y lo que lanza.

`cargo run` no aparece como padre en la captura porque cargo, en Linux, se reemplaza
por tu programa (`exec`); mientras compila, se ven cargo y sus rustc.

**Sin rehacer nada de más.** La salida de la terminal, las muestras de `/proc` y
teclear en el editor no rehacen la geometría del grafo. Solo un error nuevo en un
nodo la rehace, porque el nodo cambia de color. En la captura hay 2 subidas a la GPU
en 70 cuadros, con la terminal escribiendo y `/proc` muestreando.

La terminal solo dibuja las filas que se ven (hasta 2000 guardadas). El texto
coloreado del editor se guarda en caché por huella: un cuadro sin teclear no vuelve
a colorear nada. Los archivos de más de 256 KiB se editan sin colores, para que el
carril Editor siga en verde.

## Dibujo retenido en la GPU: la CPU enseña una vez y la GPU redibuja sola

![Terminal tras cargo run y carriles con 7 subidas a la GPU en 153 cuadros](maquetas/png/neon/carpetas-gpu-terminal.png)

egui es de **modo inmediato**: en cada cuadro la CPU vuelve a calcular todos los
triángulos y los manda a la GPU. Para el grafo hay un camino **retenido**, como el de
Blender y los juegos:

1. **La CPU arma la geometría una sola vez**, en coordenadas del mundo: rejilla,
   cables Bézier en 24 tramos, cuerpos, cabeceras, conectores y barras de peso
   (`gpu.rs`, función pura con pruebas).
2. **La sube a un búfer de la GPU**, y ahí se queda.
3. **Cada cuadro solo viaja la cámara:** 16 bytes (escala y desplazamiento). El
   *shader* `grafo.wgsl`, el «libreto» de la GPU, recoloca todos los vértices.
4. La geometría solo se rehace cuando **cambia el contenido** (`version_contenido`
   del núcleo). Mover y hacer zoom no lo cambian, así que no se rehace.
5. Textos, iconos y botones siguen en egui por encima. Sus letras e iconos ya son
   texturas en la GPU.

**En la prueba:**

- 4284 vértices;
- **7 subidas a la GPU en 153 cuadros**, con la cámara moviéndose y haciendo zoom una
  y otra vez;
- carril de geometría de unos 37 µs, y solo cuando algo cambia.

Lo comprobado y lo que falta:

- **Comprobado:** que la cámara no rehace la geometría, en la prueba
  `mover_la_camara_no_rehace_la_geometria` y en el contador de subidas.
- **No medido todavía:** la diferencia de FPS frente al camino CPU en tu NVIDIA. Se
  puede comparar en Ajustes > Dibujo (GPU retenido o CPU cada cuadro) mirando el
  carril Pintado en F12.

Si no hay wgpu (por ejemplo, en el modo de recuperación), se dibuja en la CPU como antes.

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
| **Geometría** | 3 ms | 10 ms | CPU solo cuando cambia el grafo; mover y zoom solo mandan 16 bytes de cámara a la GPU |
| **Terminal** | 1 ms | 5 ms | Líneas de procesos y lector de errores: como mucho 512 por cuadro, 2000 guardadas y 500 errores |
| **Procesos** | 5 ms | 50 ms | Leer `/proc` cada 500 ms en **su propio hilo**; solo el grupo del play, como mucho 64; no rehace la geometría |
| **Editor** | 4 ms | 10 ms | CPU (egui): texto coloreado en caché por huella; sin colores por encima de 256 KiB; teclear no rehace el grafo |
| **Pintado** | 6 ms | 12 ms | CPU (egui): solo se pinta lo que cae en pantalla; con poco zoom se omiten textos e iconos; solo se repinta con eventos |
| **Iconos SVG** | 8 ms | 30 ms | CPU una vez por icono y tamaño; después es textura en la GPU |

**Límites duros** (por diseño, no por suerte):

- 5000 entradas por carpeta;
- 600 nodos en el grafo;
- 64 carpetas desplegadas, que son las que se vigilan;
- 1 MiB por archivo de orden y 64 KiB de ajustes;
- 2 MiB por archivo en el editor, 500 errores atrapados por ejecución y 64 procesos
  en vivo.

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
- **Pruebas:** en el núcleo, además de las de abajo, las de errores (rustc, panics,
  gcc, Python, rutas que salen de la carpeta, parar en el primer error), editor
  (clic derecho, cambios sin guardar, F8), procesos en árbol y colores de código.
  En la capa Linux: leer y guardar del editor (enlaces, binarios, permisos, cambios
  de fuera), `/proc` con un grupo real y el anillo rojo en la geometría de la GPU.
  Las de siempre: 9 en el núcleo, al estilo Redux (acción → estado + efectos): arrastre,
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

- Probarlo en Hyprland real con NVIDIA (aquí se probó con Vulkan por software). Los
  tiempos de los carriles en la pantalla virtual no valen como medida de tu PC.
- Editor: buscar y reemplazar, varias pestañas y comentarios `/* */` de varias líneas
  coloreados enteros (ahora solo su primera línea).
- Miniaturas de imágenes, renombrar y mover archivos de verdad (con confirmación y
  deshacer), varias ventanas y arrastrar entre carpetas.
