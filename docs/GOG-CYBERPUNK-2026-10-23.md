# Cyberpunk de Personal: reconocimiento en Heroic

## Causa comprobada

`D:\Cyberpunk 2077` contiene el manifiesto GOG del juego base **1423049311**
y el ejecutable. ULTRA prepara esa instalacion como `/juegos/Cyberpunk2077`,
con originales de Personal en solo lectura y escrituras en ext4.

El importador anterior solo enumeraba carpetas elegidas manualmente. El
catalogo nativo actual enumera carpetas de C: pero ninguna de Personal. Por
eso el acceso directo podia iniciar Cyberpunk mientras el importador de GOG
no ofrecia su carpeta. Tener un juego montado no lo registra en Heroic.

## Correccion

Al abrir **Super+J > GOG / Heroic**, `ultra-gog-cyberpunk` comprueba el home
persistente ext4, el montaje overlay, el ID del juego, build, idioma y presencia
del ejecutable Windows. Registra la carpeta Linux en
`~/.config/heroic/gog_store/installed.json` antes de iniciar Heroic.
La opcion **Importar juegos GOG** tambien ofrece Cyberpunk aunque Personal
no este en la seleccion manual. El importador general sigue disponible.

Se usa el esquema de instalacion de la version empaquetada **2.22.4**,
contrastado con
[GOGLibraryManager.importGame y refreshInstalled](https://github.com/Heroic-Games-Launcher/HeroicGamesLauncher/blob/v2.22.4/src/backend/storeManagers/gog/library.ts).
El codigo incluye plataforma, build, idioma y DLC de los manifiestos locales.
No se inventa una version comercial: se conserva el build local como version
de respaldo, equivalente al fallback del
[importador gogdl](https://github.com/Heroic-Games-Launcher/heroic-gogdl/blob/main/gogdl/imports.py).
No se descarga el juego ni se modifica su contenido para registrarlo.

La correccion local es especifica de Cyberpunk y de esta version de Heroic;
el marcador de version se instala junto al cliente. No es un importador
universal ni una comprobacion completa de integridad de todos los archivos.

Se conservan los demas registros, ajustes de la entrada existente y prefijos.
La primera modificacion conserva una copia local acotada de installed.json.
Con Heroic abierto se rechaza la escritura; cierra el cliente y vuelve a abrir
GOG desde el menu. Un JSON desconocido o un ejecutable redirigido se rechazan.
El registro es idempotente y no reescribe una entrada que ya coincide.

No se leen tokens, claves ni auth.json. Heroic mantiene la cuenta, el catalogo
de juegos de esa cuenta y las operaciones con GOG. Si el juego no aparece en
el catalogo de la cuenta o el usuario no ha iniciado sesion en Heroic, este
registro local no cambia esa situacion. No se toca la instalacion Windows.

Los mensajes de preparacion se consultan desde **Registros de juegos**,
apartado `gog-cyberpunk`. El congelamiento al cargar partida y el Xid 109
documentados en `CARGAS-GPU-RED-2026-10-23.md` son una investigacion distinta:
reconocer la carpeta no demuestra resolver el bloqueo grafico.

## Validacion

- Ocho pruebas de registro: juego/DLC, reparacion de ruta Windows, conservacion
  de otros juegos y ajustes, idempotencia, cliente abierto, JSON incompleto,
  producto incorrecto y rechazo de enlaces/ejecutable incorrecto.
- Cinco pruebas del selector y descubrimiento, incluida la opcion Cyberpunk
  sin seleccion manual de Personal.
- La opcion `--gog-cyberpunk` del banco de arranque prepara dentro de QEMU
  un ext4 de 64 MiB, overlay y archivos GOG ficticios. Comprueba el programa
  instalado como UID1000 sin privilegios, el registro y su repeticion.
  No conecta discos personales ni usa cuentas o ejecutables de juegos reales.

El perfil **PC #41** paso **164 comprobaciones de arranque, cero fallos**, mas
las trece pruebas de componentes. Se instalo en A: verificando SHA256 del
kernel y EROFS, y conservando el SHA256 de datos.ext4. Evidencia:
`evidencias/gog-cyberpunk/2026-10-23`. El perfil generico no se reconstruyo en
este cambio. No se ejecutaron juegos reales ni se utilizaron cuentas en QEMU.
La visualizacion del juego en la cuenta personal necesita la siguiente
apertura de Heroic en el PC.

**No desinstalar todavia `D:\Cyberpunk 2077`**: sigue siendo la base leida
por Linux. Registrar el juego no equivale a copiarlo a una biblioteca nativa
independiente; esa migracion necesita resolver primero la capacidad disponible.
