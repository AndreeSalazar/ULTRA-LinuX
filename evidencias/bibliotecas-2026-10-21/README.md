# Gestor general de carpetas y discos

Ronda del 21 de octubre de 2026, hora de Lima; registros de construccion en UTC
del 22. Ambos perfiles estan instalados en A: y el respaldo anterior permanece
en el proyecto Windows. [Uso del gestor](../../docs/BIBLIOTECAS.md).

## Resultado

**87 comprobaciones finales sin fallos:**

| Banco | Comprobaciones |
|---|---:|
| Validacion de selecciones y limites de entrada | 18 |
| Montajes reales, persistencia y permisos en el taller aislado | 11 |
| Imagen generica: dos discos, selector con teclado y reinicio | 29 |
| Imagen PC: dos discos, selector con teclado y reinicio | 29 |

Las pruebas nativas arrancan los sistemas publicados con sus ayudantes
incorporados, sin inyectarlos y sin iniciar udev previamente. No reciben discos
fisicos ni cuentas reales. El almacenamiento sintetico de juegos reutiliza su
cache de pruebas anterior; el NVMe Windows se clona de una imagen ficticia.

![Seleccion entre dos discos ficticios](generico/discos.png)

La prueba abre Super+J, entra al gestor, elige con teclado una carpeta del
segundo disco y verifica su configuracion persistente. Tras reiniciar activa
esa carpeta y una biblioteca Steam del primer disco. Los prefijos/caches Linux
admiten escritura, los originales permanecen protegidos y PID1 desmonta las
capas antes de liberar sus fuentes. El NTFS completo del primer disco conserva:

`c2a01733f9690b3a91acdbc577f95f7d67c72e9f0b90e6e7c578427ee4c4300f`

En el segundo disco se comprueba el contenido original del juego y la ausencia
de escrituras fuera de su capa. Su NTFS contiene el ext4 del home: ese archivo
cambia intencionadamente con la persistencia, por lo que no se afirma que el
hash de todo ese disco sea invariable. La vista publica es de solo lectura y
el montaje privado conserva escritura para el almacenamiento Linux.

## Casos de seguridad y diagnostico

Se rechazan rutas absolutas, recorrido de padres, UUID manipulados, caracteres
de control, entradas excesivas y enlaces simbolicos en el camino privilegiado.
El banco Linux monta realmente un origen con espacio, coma y Unicode usando
un descriptor fijado y alias seguros, y comprueba directorios opacos para los
prefijos y caches de Steam. Las capas se conservan ante un cambio de version.

El montaje no eleva la lectura de carpetas Unix privadas: se prueban una carpeta
root 0700, un hijo 0755 dentro de ella y un steamapps privado dentro de una
biblioteca publica. Todos son rechazados antes de exponerlos mediante overlay.

La prueba nativa planta un modulo Python de usuario benigno que crearia un
marcador al importarse. El exportador aislado lo ignora; el modulo se retira
al terminar la comprobacion. El lector de configuracion se ejecuta como UID1000,
con capacidades retiradas y sin la entrada de la consola privilegiada.

La prueba inicial encontro que lsblk devolvia FSTYPE/UUID vacios incluso con
udev iniciado. El gestor definitivo consulta blkid en modo de lectura como
alternativa. Se conserva el diagnostico preliminar en `registros.zip`; no se
cuenta como resultado final. El banco Linux tambien detecto y corrigio el orden
de desmontaje del alias antes de reutilizarlo.

## Construccion e instalacion

Linux 7.3-rc7, commit upstream `32c52ec87d6245ee2d12ed5dd509e40062939b50`;
construcciones generica **#48** y PC **#34**. Se conservan los perfiles y sus
controladores existentes. La imagen doble de PC incorpora tambien el sistema
generico de rescate actualizado; los manifiestos registran sus hashes.

`ayudantes-verificados.json` compara doce archivos instalados, seis por perfil,
con las fuentes del proyecto. `instalacion-A.json` verifica los dos EFI y los
dos EROFS copiados. `datos.ext4` mantiene el hash anterior:

`e5ee9c8e24d8077c769d3a54c7a204f7be479c1904b287cab2ff286d17200160`

No se formatearon particiones, no se modificaron entradas de firmware y no
se reemplazo la imagen personal de D:. Taller y servidor local apagados.

## Limites

El gestor selecciona carpetas; Steam/Heroic deben registrarlas y verificar los
juegos. No se ha ejecutado un juego real ni autenticado clientes en esta ronda.
No se acredita compatibilidad universal con Proton, antitrampas, FPS ni VRAM
de la RTX. La seleccion de carpetas no elimina las limitaciones de seguridad
del kernel/controlador documentadas en rondas anteriores.

Los cambios comparten los 16 GiB persistentes actuales. Una actualizacion puede
necesitar copiar archivos completos y agotar ese espacio. No se borran capas
ni partidas automaticamente. Las nuevas selecciones se aplican al volver a
abrir el escritorio; los montajes activos se liberan al apagar/reiniciar.
