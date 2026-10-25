# Heronite como gestor principal

Solicitud: reemplazar la interfaz de Heroic y reunir Steam/GOG en Heronite.

## Entrega

Heronite **0.15.0**, paquete x86-64 oficial, fuente etiquetada en
`ed27c7d181befb55cf122332f25145bf515025ba`.
SHA-256 del paquete:
`512b515a0af6c5eb9243096835672aef0f9defb010a2c5772d1c11f6ac64d657`.

La API de GitHub devolvio esta version mas reciente que la 0.14.0 que aparecia
en la pagina web de la investigacion anterior. [Release oficial](https://github.com/RafaelGoulartB/Heronite/releases/tag/v0.15.0).

El instalador retira exclusivamente `/opt/heroic` de la imagen del sistema.
Conserva los pequenos ejecutables de tiendas existentes en
`/usr/lib/heronite-tools`; Heronite puede descargar sus herramientas cuando falten.
Se incluyen las bibliotecas necesarias para la interfaz y la ventana de login.
Las comprobaciones `ldd` de ambos ejecutables deben pasar antes de empaquetar.

Steam permanece instalado: los juegos Steam se ejecutan mediante su cliente.
Heronite organiza su biblioteca y solicita los lanzamientos. No reemplaza Steam,
Proton ni sus servicios de cuenta. No se promete ahorro total del cliente Steam.

## Uso y persistencia

- Super+Espacio > Juegos > **Heronite / Steam y GOG**.
- Cyberpunk abre primero Heronite para actualizar/verificar y despues jugar.
- Warframe solicita `sideload_steam_230410`, identificador comprobado en el codigo
  de Heronite; Steam realiza el lanzamiento.
- Una sola entrada Heronite visible en el lanzador de aplicaciones. Los accesos
  anteriores de Steam siguen resolviendo a la biblioteca unificada.

El adaptador `ultra-heronite` corre como UID1000, conserva el entorno del HUD y
prepara bibliotecas antes de abrir. Cuando Heronite esta abierto no modifica sus
metadatos. Un bloqueo de inicio serializa preparaciones simultaneas.

En el primer inicio usa `heronite-rs --help`: en la version fijada ejecuta primero
la migracion nativa y termina antes de abrir una ventana. El migrador copia
`~/.config/heroic` a `~/.local/share/heronite` mediante una carpeta temporal y
renombrado final. Conserva el original y adapta rutas a herramientas copiadas.
El adaptador corrige solo las referencias globales a antiguos ejecutables
incluidos en `/opt/heroic`. No cambia runners personalizados, prefijos o partidas.
No crea un segundo prefijo de Cyberpunk. La copia inicial de herramientas puede
ocupar espacio y tardar; no es una tarea que se repita en cada arranque.

No se eliminan datos anteriores de Heroic: son respaldo para una vuelta atras.
No ejecutar el gestor anterior sobre los datos nuevos. Los datos importados,
sesiones, credenciales, partidas y herramientas permanecen privados y fuera de Git.

El registro de juegos GOG y la observacion de recursos leen el almacen de Heronite.
La observacion conserva lectura de Heroic como compatibilidad antes de migrar.
El registro de inicio esta en `~/.cache/ultra/heronite.log` y puede revisarse en
Juegos > Registros de juegos; no compartir sin revisar posibles rutas personales.

## Consumo

Se elimina la interfaz Electron de Heroic. Heronite tiene renderizado GPU: medir
RAM/PSS, VRAM y CPU a igual biblioteca, escena y FPS. No suponer menos vatios por
estar escrito en Rust ni sumar RSS compartido como memoria exclusiva.
No se activa inicio automatico de Heronite. Sus opciones upstream de overlay y
replay estan desactivadas por defecto; no se habilitan en esta integracion.
El perfil optativo Ahorro del escritorio se incluye sin imponerlo al usuario.
MangoHud excluye los ejecutables de Heronite y su login, conservando el HUD en
los juegos. Se actualiza tambien la lista de exclusion de configuraciones ULTRA
ya existentes, sin borrar sus limites de FPS u otras preferencias.

## Validacion y limite

La migracion del ejecutable nativo se prueba sin red en un home temporal dentro
del taller: original inalterado, herramientas reubicadas, prefijo y metadatos GOG
conservados, segundo inicio idempotente y ausencia de cuentas.
Los bancos del adaptador y lanzadores verifican preservacion, errores y rutas.
Las pruebas graficas QEMU usan renderizado software y datos ficticios; no
demuestran compatibilidad final de cada juego con la RTX. La importacion de la
cuenta real ocurre en el primer inicio del usuario, manteniendo el original.

Entrega instalada en A: FAT32, perfil PC, verificada por SHA-256. Kernel y
`datos.ext4` identicos a los anteriores. El banco final paso 180 comprobaciones,
incluyendo apertura real en Wayland sin superposicion del HUD, UID1000,
NoNewPrivs, reinicio y desmontaje. Las [evidencias](../evidencias/heronite/2026-10-25/)
incluyen captura real QEMU, hashes, bancos y origen del paquete.
