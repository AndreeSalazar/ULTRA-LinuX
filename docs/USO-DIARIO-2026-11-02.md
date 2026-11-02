# Uso diario: audio RT, USB y respaldos (2026-11-02)

Origen: informe `A:\ULTRA\informe` del 2026-11-01 (kernel 7.3.0-rc7-pc, NVIDIA + Hyprland).

## Audio en tiempo real
PipeWire registraba `Realtime scheduling disabled`: la sesión no tiene D-Bus de
sistema ni RTKit. `module-rt` de PipeWire 1.6 prueba primero `RLIMIT_RTPRIO` y
recorta la prioridad a ese límite, sin D-Bus.

`ultra-sesion` (root) fija los límites justo antes de `setpriv`:
`rtprio=88`, `nice` hasta -11 y `rttime=200 ms`. Ese último límite corta un hilo
RT que no se bloquee. `NoNewPrivs` y las capacidades vacías no cambian.
`ultra-cliente` lo muestra en el informe como `VERDE [AUDIO-RT]`.

## USB: `ultra-usb` (Super+U)
- **Supervisor root** (`--supervisor`, lo inicia `ultra-sesion`):
  - Monta en `/run/media/ultra/<etiqueta>` con `nosuid,nodev,noexec`, y además `uid=1000` en vfat, exFAT y NTFS.
  - Si no puede montar en escritura, prueba en solo lectura.
  - Solo toca discos `sd*` cuya ruta sysfs pase por USB. No toca un disco que tenga algún montaje ajeno o *holders*, como el arranque o una imagen loop.
  - Tampoco monta etiquetas `ULTRA-*`/`BMO` ni particiones EFI.
  - Nunca formatea ni repara.
- **Expulsar**: petición privada `/run/user/1000/ultra-usb-peticion`, con el mismo contrato que `ultra-sesion-control`. Solo acepta dispositivos que el supervisor montó.
- **Al cerrar la sesión**: sincroniza y desmonta todo.
- **Usuario**: `vigilar` avisa al conectar, al expulsar y al retirar sin expulsar. `menu` permite abrir, respaldar o expulsar.

## Respaldos: `ultra-respaldo`
- **Qué guarda**: `.config` sin cachés, llavero, datos de Heronite y ULTRA (sin la AppImage), `Steam/userdata`, documentos y partidas de los prefijos Wine/Proton (`Documents`, `Saved Games`, `AppData`; sin cachés de sombreadores).
- **Límites**: 512 MB por archivo y 8 GB en total.
- **Local**: diario y automático, 3 minutos después de entrar, con `nice 19` e `ionice` inactivo. Se guarda en `~/.local/share/ultra/respaldos` y se conservan los últimos 5.
- **USB**: desde Super+U, en `ULTRA-respaldos/` de la unidad; se conservan los últimos 3.
- **Restaurar**: `ultra-respaldo restaurar ARCHIVO` extrae en `~/Restaurado/<fecha>` sin sobrescribir nada.

## Evaluado y no aplicado
- **FUSE**: aunque se active `CONFIG_FUSE_FS`, la sesión corre con `NoNewPrivs=1`, y por eso `fusermount3` (setuid) no puede montar. El portal de documentos y las AppImage seguirían igual. Las AppImage ya usan *extract-and-run*.
- **Bluetooth**: el informe no muestra ningún adaptador Bluetooth (PCI/USB).
- **Red a 100 Mb/s**: el router o el switch solo anuncia 10/100. Es el cable o el puerto, no el sistema.

Pruebas: `probar_usb.py` (11) y `probar_respaldo.py` (4), ejecutadas en Windows con un adaptador mínimo. Falta validarlas en QEMU y en el PC.
