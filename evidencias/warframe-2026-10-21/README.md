# Steam, Warframe y diagnostico de juegos

Reconstruccion del 21 de octubre de 2026, hora de Lima. Los registros del taller
y la instalacion usan UTC del 22 de octubre. Ambos perfiles estan copiados en A:.

## Resultado

**286 comprobaciones completadas sin fallos en la ronda final:**

| Banco | Comprobaciones |
|---|---:|
| Arranque, guardianes y escritorio generico | 115 |
| Arranque, guardianes y escritorio PC | 116 |
| Warframe, montajes y persistencia en generico | 20 |
| Warframe, montajes y persistencia en PC | 20 |
| Aislamiento y casos de rechazo en el taller | 15 |

Los resultados de arranque incluyen el menu de juegos abierto con Super+J,
su cierre con Escape, Wayland, Brave con sandbox y HTTPS, portales, persistencia
y los casos de guardianes ya existentes. Los tests de Warframe utilizan los
ayudantes incorporados a las imagenes, sin sustituirlos durante el arranque.
El almacenamiento de juegos del banco reutiliza datos sinteticos de pruebas
anteriores; no contiene cuentas reales. El NVMe se clona de una imagen ficticia.

![Menu de juegos en el perfil generico](generico/juegos.png)

Se comprueba que UID1000 modifica la capa ext4, sin modificar los originales
NTFS; la biblioteca contiene los metadatos publicos del juego y excluye la cuenta
Windows sintetica. Un proceso ficticio llamado `steam` impide preparar la
biblioteca. La capa persiste tras reiniciar y PID1 la desmonta antes de Windows.
El NTFS completo conserva el hash en ambos perfiles:

`c2a01733f9690b3a91acdbc577f95f7d67c72e9f0b90e6e7c578427ee4c4300f`

La primera revision nativa revelo que faltaban `pgrep` y `free`: el banco del
taller tenia esas herramientas, pero el sistema publicado no. Se agrego
`procps-ng 4.0.7-1` explicitamente, se hizo obligatorio comprobar procesos antes
de importar y se reforzo el test para exigir una lectura real de RAM. Se
reempaquetaron ambos perfiles y se repitieron los arranques finales. El registro
preliminar se conserva dentro de `registros.zip`, sin contarlo como ronda final.
Los avisos D-Bus al notificar desde el chroot sin escritorio del banco de
aislamiento son no fatales; el resultado funcional se comprueba por separado.

## Artefactos e instalacion

Linux 7.3-rc7 del commit upstream `32c52ec87d6245ee2d12ed5dd509e40062939b50`;
construcciones generico **#47** y PC **#33**. NVIDIA mantiene 615.78.08.
`ayudantes-verificados.json` comprueba por hash diez archivos instalados, cinco
por perfil, contra las fuentes del proyecto. Los manifiestos por perfil guardan
los paquetes y los hashes de los artefactos.

`instalacion-A.json` verifica los dos EFI y los dos sistemas EROFS contra sus
manifiestos. `datos.ext4` conserva el hash previo:

`e5ee9c8e24d8077c769d3a54c7a204f7be479c1904b287cab2ff286d17200160`

Se respaldo la instalacion anterior y los informes en el proyecto Windows.
No se formatearon particiones, no se cambiaron entradas de firmware y no se
reemplazo el archivo personal de juegos de D:. El taller y su servidor local
se apagaron al terminar.

## Limites

No se ejecuta Warframe real, no se inicia sesion en una cuenta y QEMU no recibe
los discos fisicos C: o D:. El perfil PC virtual no dispone de la RTX: estos
resultados no prueban Proton, antitrampas, FPS, VRAM ni latencia del juego real.
Los 16 GiB persistentes pueden resultar insuficientes para sus actualizaciones.
Las escrituras de la capa no se sincronizan de vuelta a la instalacion Windows.

El aviso NVIDIA SRSO sigue amarillo: 11.674 lineas objtool `naked return`, no
funciones unicas. Esta integracion no resuelve esa cobertura pendiente en hardware.
Tampoco convierte los casos de guardianes probados en una garantia universal.

Para el siguiente arranque: [guia de Steam y Warframe](../../docs/STEAM-WINDOWS.md).
