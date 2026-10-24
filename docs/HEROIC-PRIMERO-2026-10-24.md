# Heroic primero, REDlauncher después

El recorrido solicitado es: **abrir Heroic → revisar Cyberpunk y sus
actualizaciones → pulsar Jugar → REDlauncher → juego**.

El acceso de Super+Espacio, Super+J y el archivo de escritorio ahora abren
Heroic sin enviar `heroic://launch`. La URL anterior iniciaba el juego en
cuanto el cliente estaba listo; ese es el comportamiento del
[controlador de protocolo de Heroic 2.22.4](https://github.com/Heroic-Games-Launcher/HeroicGamesLauncher/blob/v2.22.4/src/backend/protocol.ts).
El acceso no simula un botón de actualización ni inicia una descarga.

En la configuración persistente de Cyberpunk se seleccionó
`/juegos/Cyberpunk2077/REDprelauncher.exe` y `ignoreGameUpdates=false`.
Se preservaron el runner, el prefijo y el resto de la configuración, con un
respaldo privado en `/juegos/.ultra-heroic-redlauncher-20261024/`.
El modo de recuperación directa con UMU continúa disponible.

Heroic ya tenía activada la comprobación al abrirse y desactivadas las
descargas automáticas. Su registro de las **15:38:40 de Perú** informó
`Found 0 game(s) to update`: esa consulta no detectó actualizaciones pendientes.
No se inventa una versión más reciente ni se altera el build instalado para
forzar una actualización. El usuario puede revisar la ficha del juego y sus
opciones de verificación/reparación dentro de Heroic.

Las cinco DLL recuperadas anteriormente se mantienen. Aquí se cambia el
recorrido de la interfaz y el ejecutable seleccionado, no los archivos del
juego. La transición visual de REDlauncher al juego requiere prueba en el PC.

Validación: cuatro escenarios del acceso y nueve del registro GOG con
fixtures; el acceso normal abre el gestor sin iniciar el juego ni crear otro
prefijo. La configuración del disco pasó comprobación de PE y revisión ext4.
La nueva imagen PC pasó **152 comprobaciones de arranque, escritorio y
persistencia, con cero fallos**. Se instaló en A: y se verificaron los hashes
del kernel, del sistema y del archivo de datos conservado. Se retiraron la
extracción temporal de 4,9 GiB y la copia de prueba de QEMU; los artefactos
finales y los respaldos se mantienen.

[Evidencias](../evidencias/heroic-primero/2026-10-24).
