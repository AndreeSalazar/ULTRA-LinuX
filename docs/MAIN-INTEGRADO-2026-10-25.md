# Integración del escritorio de main · 25 de octubre de 2026

La entrega parte de `b547692`, que ya contiene la integración externa `36ba99a`
y el trabajo del PR #2. Actualizar el repositorio no instala automáticamente las
aplicaciones nuevas dentro de EROFS: se reconstruyeron Carpetas y eww y se
incorporaron sus dependencias, configuraciones y herramientas.

## Qué incorpora

- ULTRA Carpetas: Rust 1.97.0, Cargo.lock, eframe/egui 0.37 y wgpu; acceso
  Super+Ctrl+E y ajustes con Ctrl+,. El núcleo separado se comprueba también para
  `x86_64-unknown-none`, sin biblioteca estándar y sin código `unsafe`.
- eww 0.6.0 fijado en `d87c2fdbfdc012e76d229e4e9ea3325bc0f23e89`, construido
  con Rust 1.76.0: tarjetas del equipo con Super+F9.
- Configuraciones Neón, personalización, modo juego, fuentes, iconos, cursores
  y herramientas de fondo vivo. Se incluye `awww`; el fondo animado es opcional.
- Cabeceras de glibc, GCC y Linux para compilar proyectos C/C++ dentro del
  escritorio. Antes la imagen reducida omitía `usr/include/*`.
- Regeneración y comprobación de `es_PE.UTF-8` tras reinstalar glibc. Instalar
  un paquete no basta para garantizar que su locale anterior siga disponible.
- Vulkan por software para el banco sin GPU física. Esto permite comprobar
  la aplicación con Lavapipe y no implica aceleración por la RTX en QEMU.

El estado y las acciones de Carpetas están implementados en Rust; React y Redux
son una referencia arquitectónica, no dependencias del programa. Su editor
puede modificar archivos al guardar. La organización de iconos solo cambia la
vista. Las coincidencias del museo señalan patrones sospechosos: no prueban por
sí mismas comportamiento indefinido ni aíslan automáticamente un programa.

## Construcción y caché

`scripts/actualizar-main.sh pc REVISION` trabaja sobre una imagen ya extraída en
`artifacts/pc/sistema-nativo` dentro del taller. Comprueba dependencias, construye
las aplicaciones, ejecuta sus bancos y genera una huella SHA-256 de las fuentes
incluidas. El empaquetador existente reconstruye EROFS y el disco FAT32 de prueba.
Conserva las versiones NVIDIA y el kernel; no se recompila el kernel en esta entrega.
Las 368 fuentes se compararon byte por byte con las usadas en el taller y también
contra el índice de Git. En 144 archivos Git normaliza exclusivamente CRLF a LF;
el [informe del índice](../evidencias/main-integrado/2026-10-25/fuentes-index.json)
lo distingue de las huellas de los bytes originales de compilación.

Los compiladores y sus objetivos se conservan en un disco virtual independiente
de 64 GiB, `vm/main-escritorio-taller.qcow2`. Se reconecta con:

```powershell
python scripts/taller_qemu.py start --disco-trabajo
```

Ese modo valida formato, tamaño y ausencia de una imagen base y da prioridad de
arranque al disco del taller. No formatea la caché. En el invitado se comprueban
serial `ULTRA-MAIN-WORK`, tamaño y etiqueta antes de montarla. El contenido
original de `artifacts/pc` permanece debajo del montaje bind; los respaldos
anteriores se conservan. Ningún disco físico de juegos se conecta al taller.
Tras verificar la instalación se retiraron solo las imágenes de la candidata
fallida y las copias desechables de QEMU de esta ronda. Se conservan el resultado
final, los respaldos anteriores y la caché reutilizable de compilación.

## Validación

- 194 comprobaciones de arranque y escritorio correctas en la ronda final,
  incluidos dos arranques, persistencia y apagado limpio.
- Núcleo Carpetas: 78 pruebas, más compilación bare-metal.
- Aplicación: 25 pruebas correctas; un benchmark está marcado como ignorado.
- Bancos de personalización, modo juego, fondo vivo, tarjetas, registro Steam y
  lanzador Cyberpunk. El banco de procesos de la barra usa procesos desechables
  en la VM y comprueba selección, confirmación y protección de la propia barra.
- Programas mínimos C y C++ compilados y ejecutados dentro de la imagen.
- Arranque UEFI/FAT32, persistencia, guardianes, cursor y escritorio real:

```powershell
python scripts/probar_arranque.py pc --bmo --gog-cyberpunk --tema-nucleo --puntero --carpetas --ram-mib 4096 --artefactos build/main-integrado2-pc
```

Las capturas usan carpetas sintéticas y renderizado por software. La prueba de
Carpetas comprueba usuario 1000, apertura mediante el atajo real, biblioteca
Vulkan del banco, ajustes y cierre. eww se abre, oculta y vuelve a abrir.

La primera candidata detectó un locale ausente y la falta del controlador Vulkan
por software; ambas comprobaciones se añadieron a la integración. Esa candidata
no se instaló en A:.

El banco de categorías espera ahora el proceso del menú con su prompt correcto
antes de enviar teclas y de capturar. Un retardo fijo podía enviar la selección
antes de que apareciera el menú al dibujar por software. La comprobación visual
espera el primer cuadro durante un máximo de diez segundos, con el mismo umbral.

## Alcance

La candidata final se instaló en A: FAT32, perfil PC, el 25 de octubre de 2026.
EROFS ocupa 1.967.075.328 bytes, dentro del límite por archivo de FAT32. Su SHA-256
es `14d702598bb1df1da67f60c51f719013897d6c50749953b8e7b0c03d85cc9ecc`.
La copia se leyó y verificó; el kernel y `datos.ext4` conservan sus hashes anteriores.
El [recibo de instalación](../evidencias/main-integrado/2026-10-25/instalacion.json)
registra esa comparación.

Se conserva el perfil genérico de recuperación y la persistencia del usuario.
Esta entrega no modifica el home nativo, los juegos, sus prefijos ni las partidas.
El autor ya pudo jugar en el PC; la validación de esta compilación nueva sobre la
RTX, su cursor y los lanzadores sigue siendo una comprobación física distinta.
El tirón al pulsar Jugar dentro de Steam y los vatios de CPU no se dan por
resueltos con estos bancos. Las herramientas Rust de proyectos requieren un
toolchain instalado por el usuario; la aplicación Carpetas ya va compilada.

Las imágenes y los informes públicos de esta entrega se guardan en
[`evidencias/main-integrado/2026-10-25`](../evidencias/main-integrado/2026-10-25/).
Los GIF y videos del prototipo del navegador siguen identificados como maquetas.
