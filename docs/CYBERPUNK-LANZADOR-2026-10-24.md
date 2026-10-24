# Cyberpunk: unificar el acceso del escritorio con Heroic

El informe del PC del 24 de octubre, 12:17 en Perú, confirma el montaje de
`/juegos/Cyberpunk2077` sobre la base Linux verificada. La inspección posterior
se hizo con QEMU sin red, dispositivo de solo lectura y montaje ext4
`ro,noload`. Había **247.335.059.456 bytes disponibles**. El registro GOG
contiene la ruta correcta y el importador informa que ya está reconocido.

El acceso del escritorio lanzaba UMU directamente con
`~/Juegos/prefijos/cyberpunk2077`, mientras Heroic tiene configurado otro
prefijo: `~/Games/Heroic/Prefixes/Cyberpunk 2077`. También elegía GE-Proton
automáticamente en lugar de respetar el runner guardado. Son dos entornos
distintos; esta inconsistencia no demuestra por sí sola la causa del cierre.

Ahora **Super+Espacio → Juegos → Cyberpunk 2077** usa el protocolo oficial
de [Heroic 2.22.4](https://github.com/Heroic-Games-Launcher/HeroicGamesLauncher/blob/v2.22.4/src/backend/shortcuts/shortcuts/shortcuts.ts),
`heroic://launch?appName=1423049311&runner=gog`. Usa su configuración existente,
incluidas las comprobaciones de actualización. No crea otro prefijo ni cambia
partidas. Si el registro no puede prepararse, se abre Heroic para revisarlo.
Un registro ya correcto puede consultarse con Heroic abierto; las escrituras
de metadatos siguen bloqueadas mientras el cliente está ejecutándose.

El modo anterior queda como **Cyberpunk / recuperación y diagnóstico**.
Activa `PROTON_LOG`, conserva su prefijo anterior y registra el código de
salida. **Registros de juegos** incluye ahora también Heroic y el nuevo acceso.
Los avisos ya no dirigen al usuario a un original que fue retirado de D:.

El aviso de Proton «unable to use parent for game drive» devuelve la ruta
original y continúa en el [código de Proton](https://github.com/ValveSoftware/Proton/blob/proton_11.0/proton).
No se considera prueba de un fallo de permisos ni motivo para abrir permisos
de todo el disco. El historial de actualización contiene un error antiguo
de espacio; no demuestra que el disco actual esté lleno.

Validación con 4 escenarios del lanzador y 9 del registro GOG, sin juegos,
cuentas ni red. La carga real de la partida requiere el PC con RTX: no se
certifica que el cambio de acceso haya resuelto una actualización incompleta
o un fallo interno del juego. No se borran capas, partidas ni prefijos.

## Entrega comprobada

El banco PC terminó con **186 comprobaciones correctas y ninguna fallida**.
Se instaló el sistema en A: y se verificaron los hashes del kernel y EROFS.
El archivo `datos.ext4` de la sesión más reciente se conservó sin cambios.
Una segunda inspección de solo lectura encontró **239.122.628.608 bytes
disponibles** (unos 223 GiB); el montaje del juego sigue presente en el informe
del PC de las 13:27 de Perú. Los registros del acceso UMU no contienen un nuevo
intento posterior al de las 12:16. No se infiere de ello un lanzamiento exitoso.

[Evidencias](../evidencias/cyberpunk-lanzador/2026-10-24).
