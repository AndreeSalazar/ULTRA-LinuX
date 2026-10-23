# Bibliotecas recuperables y audio maestro

## Evidencia nativa

El informe PC / NVIDIA / Hyprland del 23 de octubre registra una biblioteca
Steam rechazada por `origen/version cambio` y cero bibliotecas nuevas
registradas. También se eligió `Program Files (x86)` como carpeta general:
antes no se buscaban bibliotecas dentro de esa selección.

Las 120 muestras muestran RAM del sistema entre 587 y 5967 MiB, VRAM entre
1 y 2882 MiB, uso GPU hasta 99 %, temperatura hasta 75 °C y potencia hasta
169,65 W. Son máximos muestreados, no límites absolutos ni consumos exclusivos
del escritorio. Los contadores Ethernet indican enlace de 100 Mb/s sin errores
ni descartes en las líneas revisadas. No equivalen a una prueba de velocidad
de Internet. Los informes personales completos se conservan fuera de Git.

## Cambios

- Al cambiar los manifiestos de Windows, la capa anterior se renombra dentro
  de `/juegos/.ultra-bibliotecas/` con sufijo `.anterior-...`. Se crea una capa
  limpia y se conserva la anterior completa, incluidos prefijos y partidas.
  Sus cambios no se mezclan automáticamente con los archivos nuevos. Una
  identidad distinta o una capa redirigida siguen provocando rechazo.
- Se buscan bibliotecas Steam dentro de las carpetas elegidas: hasta tres
  niveles, 2048 directorios y 128 selecciones resultantes, sin seguir enlaces
  simbólicos. No se escanea el disco entero ni se inventan manifiestos.
- Se registra el bit de instalación de Steam aunque haya otros estados como
  actualización pendiente. No se cambia el estado original ni se evita la
  verificación que necesite Steam.
- La preparación específica de Warframe desaparece de los menús. Su lugar
  abre el panel general de bibliotecas con estado real. El acceso para jugar
  Warframe sigue disponible.
- `Importar juegos GOG` abre Heroic y la vista preparada elegida. Heroic
  permite importar juegos ya instalados; la selección del juego y de su
  carpeta se completa allí. No se confunde una carpeta existente con un
  juego registrado o perteneciente a una cuenta.
- Super+A y Super+Espacio > Audio ofrecen 200, 300, 400 y 500 %, además de
  los niveles anteriores. Subir volumen tiene límite 500 % y hay retorno a
  100 %. No se fuerza 500 % al iniciar; la amplificación digital puede saturar.

## Uso después de actualizar

Entra de nuevo al escritorio para preparar las vistas. Cierra Steam por
completo y ábrelo desde Juegos para registrar las bibliotecas. Si faltan
juegos, revisa el panel: la carpeta debe contener `steamapps` y sus
`appmanifest_*.acf`, o estar dentro del alcance de búsqueda de una selección.
Una copia de un EXE sin manifiesto no se convierte en instalación Steam.
Para GOG selecciona la carpeta existente desde la nueva opción de importación.

Las capas anteriores ocupan espacio; no se eliminan automáticamente. No se
formatean discos y los originales de Windows continúan en solo lectura.

Fuentes de comportamiento de las herramientas:
[Heroic, importación de juegos existentes](https://github.com/Heroic-Games-Launcher/HeroicGamesLauncher),
[WirePlumber, control de volumen](https://pipewire.pages.freedesktop.org/wireplumber/daemon/getting_started.html).

## Verificación y despliegue

Instalados en A: genérico #53 y PC #39, con los mismos nombres de versión
del kernel que antes. Se contrastaron por SHA256 los dos kernels y las dos
imágenes EROFS contra sus artefactos. La huella de `datos.ext4` antes y después
coincide; los informes se respaldaron en el proyecto Windows.

| Prueba final | Resultado |
| --- | --- |
| Arranque genérico, escritorio, Brave y guardianes | 156 verdes, 0 rojos |
| Arranque PC, escritorio, Brave y guardianes | 157 verdes, 0 rojos |
| Bibliotecas, carpeta padre, registro Steam, teclado y reinicio | 34 comprobaciones |
| USB, 500 %, límite, silencio, muestras PCM y persistencia | 16 comprobaciones |
| Almacenamiento en C: ficticio y protección de Windows | 18 comprobaciones |

Los bancos usan discos ficticios y ninguna cuenta. La prueba completa de
un cliente Steam recién descargado no pasó: el banco de 6 GiB falló al extraer
su runtime actualizado. No se atribuye todavía a una causa demostrada. Otro
intento con 12 GiB excedió la espera de detección USB. La prueba posterior
centrada en audio, sin descargar clientes, terminó correctamente. No se afirma
que la apertura de ese cliente Steam nuevo haya quedado validada.

El primer arranque genérico excedió una espera; la repetición completa pasó.
La prueba de teclado se corrigió para esperar que abra el panel actual antes
de enviar teclas. Los resultados de esas ejecuciones previas se distinguen de
las pruebas finales.

Evidencia y hashes: [verificación](../evidencias/bibliotecas-audio-2026-10-23/verificacion.json),
[despliegue](../evidencias/bibliotecas-audio-2026-10-23/despliegue.json),
[manifiesto](../evidencias/bibliotecas-audio-2026-10-23/SHA256SUMS).

Para repetir los casos principales:

```text
python scripts/probar_audio_juegos.py generico --bibliotecas
python scripts/probar_audio_juegos.py pc --solo-audio
python scripts/probar_audio_juegos.py pc --almacen-c
```
