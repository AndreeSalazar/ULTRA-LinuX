# Cyberpunk de Personal al almacenamiento Linux

El traslado conserva los archivos de `D:\Cyberpunk 2077` y la capa de cambios
que Heroic ya tenía en el ext4. No es una reinstalación ni una descarga del
juego completo. El origen contiene **4.991 archivos** y **91.350.992.227 bytes**,
unos 85 GiB.

La ruta que usan Heroic y el acceso directo sigue siendo
`/juegos/Cyberpunk2077`. Su base pasa a la partición Linux identificada por
UUID `19a78f63-4252-45e8-b032-5b6e53f67af5`, anteriormente F: en Windows.
Las actualizaciones, mods y escrituras siguen usando la capa existente de
`/juegos/overlay`; las partidas y prefijos del home no se cambian.

## Copia y verificación

La copia se realiza en una carpeta provisional, con D: adjuntado en solo
lectura y sin red ni clientes de cuentas en QEMU. El destino virtual expone
solo el intervalo de bytes de la partición Linux, excluyendo EFI y A:.
No se vuelve a formatear el disco.

Se calcula un manifiesto SHA-256 de todos los archivos del destino, incluyendo
rutas relativas, tamaños y hashes. Se calcula el mismo manifiesto desde
Windows sobre el origen actual. La carpeta solo se publica como base local
cuando ambos resultados coinciden. El original de D: se retira después de
verificar también el montaje, el acceso como UID1000 y la imagen de arranque.

## Base protegida y actualizaciones

La base queda en `/juegos/.ultra-cyberpunk-original/Cyberpunk2077`, detrás de
un directorio root con modo 0700. Un recibo root con modo 0600 registra la
verificación, el producto GOG, el build y el UUID del almacenamiento.

Los archivos de la base tienen UID1000 para que las comprobaciones de permisos
del overlay permitan actualizar el juego. El directorio privado impide que el
usuario modifique directamente esa base: sus escrituras mediante la ruta
visible se guardan en la capa superior. Este comportamiento reproduce la
separación que antes proporcionaba el montaje privado del original de D:.

El preparador valida el recibo, la protección del directorio, el ejecutable PE
y el manifiesto GOG antes de seleccionar la base Linux. Sin una copia validada
sigue disponible el camino anterior de Personal. No se inventa un estado
«actualizado» ni se borra la actualización parcial de Heroic.

Las pruebas de almacenamiento añaden tres casos: iniciar sin el disco
Personal, escribir como UID1000 sin modificar la base y rechazar un recibo
con permisos inseguros. Se ejecutan con archivos ficticios, sin cuentas,
discos personales ni conexión a Internet.

Windows no puede abrir este ext4 directamente; esta copia es para ULTRA Linux.
Una actualización pendiente aún puede necesitar verificación o descarga del
parche correspondiente. La carga real de una partida se comprueba al arrancar
el perfil PC en el hardware del usuario.

## Resultado

Los **4.991 archivos** coinciden por SHA-256 entre el destino Linux y el
origen leído desde Windows. Se comprobó el montaje del juego real y una
escritura como UID1000 mediante su capa superior. ext4 pasó la revisión final.

Después de la autorización explícita del usuario para borrar el original,
se retiró exclusivamente `D:\Cyberpunk 2077` y se liberaron
**91.360.669.696 bytes**, unos **85,1 GiB**, en D:.
El almacenamiento Linux conserva unos **230,3 GiB disponibles**.
Las partidas, prefijos y la capa anterior de Heroic se mantienen.

La imagen PC de A: está actualizada y verificada por SHA-256; `datos.ext4`
permanece idéntico. El banco de arranque pasó **186 comprobaciones** y el banco
de almacenamiento pasó **9 escenarios**. El kernel y los seis ejecutables C
auditados permanecen iguales. El genérico anterior sigue como recuperación.

Para comprobarlo en el PC: F11 → **ULTRA LinuX PC (Zen 3)** →
**Super+J → Cyberpunk 2077** o **GOG / Heroic**.
La carga real de una partida y la actualización pendiente de Heroic siguen
necesitando esa prueba física; no se ejecutó el juego en QEMU.

[Evidencias del traslado](../evidencias/cyberpunk-nativo/2026-10-24).
