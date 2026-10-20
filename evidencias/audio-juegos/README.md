# Audio USB, Steam y GOG: validacion final

20 de octubre de 2026. Total: **254 comprobaciones sin fallos**.

| Perfil | Arranque, guardianes, Brave y red | Audio, clientes y persistencia |
| --- | ---: | ---: |
| generico | 107 | 18 |
| PC | 111 | 18 |

`generico/` y `pc/` contienen resultados y capturas de Steam con formulario
visible. `consolas.zip` conserva las consolas completas, con sus rutas por
perfil y la prevalidacion anterior. Los JSON identifican el hash de la imagen verificada.
El generico reutiliza la cache del cliente del banco; PC instala sus clientes
desde almacenamiento vacio. No usan los modos de inyeccion de prevalidacion.
Heroic aparece como ventana Wayland y su panel de novedades es visible detras
de Steam. No se inicio sesion en Steam/GOG ni se ejecuto Cyberpunk real.

`audio-usb-prueba.wav` contiene la señal sintetica capturada del USB de QEMU.
No es una grabacion de microfono. Se corrigieron las longitudes del contenedor
que QEMU dejo a cero al salir; las muestras PCM capturadas no se sustituyeron.
Esta evidencia no mide latencia, continuidad o calidad del GeneralPlus fisico.

`instalado-A.json` registra los hashes instalados. `instalacion.json` acredita
que datos.ext4 conserva su hash previo y localiza el respaldo principal de
Windows. `almacen-creado.json` registra el archivo nuevo de 16 GiB en D:.
No se reparticiono ni se formateo D:, y su juego original queda como capa de
lectura del overlay. No se expuso ningun disco fisico a QEMU.

`pc-real/` conserva los informes recibidos del PC, normalizados para Git.
Los bytes originales estan en `informes-fisicos-originales.zip`; sus hashes
son los de `pc-real-originales.json`. No son mediciones de la nueva imagen.
`banco/` documenta una ejecucion verificada del generador de imagenes ficticias.

`prevalidacion/` y `steam-error-transporte.png` son diagnosticos anteriores,
incluidos como historial, no como pruebas finales aprobadas. Se corrigieron
permisos de los padres del home, enlaces /dev/fd, utilidades/bibliotecas de
32 bits y loopback. El ensayo final exige formulario dibujado: una ventana
negra o un dialogo de error no cuentan como acceso a Steam.

SHA256SUMS cubre los archivos finales normalizados de esta carpeta.
