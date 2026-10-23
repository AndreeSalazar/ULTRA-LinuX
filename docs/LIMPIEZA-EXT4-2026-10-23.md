# Diagnostico y limpieza del almacenamiento personal

El usuario informo que sus dos archivos ext4 se llenaban aparentemente solos.
Se inspeccionaron sin red, sin clientes y con dispositivos virtuales de solo
lectura, montados con `ro,noload`. No se conectaron discos fisicos a QEMU.
Los informes privados completos quedan en `build/inspeccion-sololectura-c`
y `build/inspeccion-sololectura-d`, fuera de Git.

## Causa encontrada

- C: contiene el ext4 activo de 96 GiB, seleccionado por `juegos.conf`.
  Tiene cero bytes disponibles para el usuario, aunque quedan unos bloques
  reservados al administrador.
- Las capas de bibliotecas ocupan 94.087.479.296 bytes. De ellos,
  61.891.297.280 bytes pertenecen a una capa `.anterior-*` conservada cuando
  cambio la version de la biblioteca original de Windows. La capa activa
  ocupa 32.196.136.960 bytes.
- La cache del usuario ocupa 204.365.824 bytes: no explica el llenado.
- D: conserva el contenedor anterior de 16 GiB, con 2.899.128.320 bytes
  disponibles para el usuario. No es el almacenamiento configurado actualmente.

El aislamiento overlay conserva los archivos nuevos o modificados dentro de
ext4. Por ello, tener los originales del juego en Windows no significa que
actualizarlo en Linux cueste cero espacio. La rotacion de una capa evita
mezclar versiones incompatibles, pero retiene sus archivos antiguos.
No se atribuye este resultado a corrupcion ni a una fuga de RAM.

## Mantenimiento

`scripts/limpiar_capa_almacen_personal.py` es una operacion puntual para la
capa exacta diagnosticada, no un limpiador general ni un servicio automatico.
Exige un respaldo completo verificado, UUID, tamano y fecha esperados del
contenedor. Comprueba el filesystem sin modificarlo antes del montaje;
rechaza enlaces o identidades de origen distintas; retira solo la capa
anterior; compara metadatos de todo el home y la capa activa antes/despues;
desmonta y comprueba nuevamente ext4. El resultado queda en
`build/respaldo-limpieza-20261023/limpieza.json` cuando termina correctamente.

El respaldo completo queda en ese mismo directorio, fuera de Git. Puede
contener sesiones personales y partidas: no publicarlo. No se elimina
automaticamente ni se sustituye por una imagen vacia.

El usuario autorizo por separado retirar la capa antigua respaldada y borrar
la copia/capa y las partidas de Cyberpunk en Linux. Se comprobaron en el
preparador real las subcarpetas `overlay/upper` y `overlay/work`, exclusivas
del montaje Cyberpunk. Se retiraron estas y el prefijo
`usuarios/ultra/Juegos/prefijos/cyberpunk2077`; no se desinstalo el juego de
Windows. La comparacion de metadatos del home se hace despues de esa retirada
expresamente solicitada: acredita conservar el resto, no conservar esas partidas.

## Resultado verificado

- Ext4 activo: se recuperaron **61.891.203.072 bytes** al retirar la capa
  antigua, quedando **61.645.500.416 bytes (57,41 GiB)** disponibles para
  el usuario. La comprobacion ext4 previa y posterior termino correctamente.
- D: se retiro solo `D:\ULTRA-juegos.ext4`, ya sin uso, tras volver a comparar
  SHA256 con `build/almacen-ampliado/ULTRA-juegos-respaldo.ext4`.
  Son identicos: se recuperaron **16 GiB** en D: conservando el respaldo.
- Proyecto Windows: se eliminaron **102 archivos**, **106.011.287.552 bytes
  (98,73 GiB)**, de imagenes de pruebas y sistemas antiguos `.erofs`/`.efi`.
  El plan exacto y resultado quedan en `build/plan-limpieza-proyecto.json` y
  `build/limpieza-proyecto-resultado.json`. Se validaron rutas absolutas bajo
  `build`, ancestros sin redirecciones y ausencia de uso por QEMU.
- El proyecto queda en aproximadamente **189,41 GiB**: incluye el nuevo
  respaldo personal de 96 GiB, el taller de compilacion de unos 55,65 GiB,
  el respaldo antiguo de 16 GiB y las compilaciones actuales. No confundir
  esos respaldos con archivos de prueba desechables.
- Espacio anfitrion final aproximado: C: **191,76 GiB** libres;
  D: **26,57 GiB** libres. Son instantaneas del mantenimiento.

El intento inicial de retirar la capa fue detenido por la revision automatica
hasta obtener autorizacion especifica. Otro intento fue detenido para comprobar
la exclusividad de `overlay`; se sustituyo el objetivo amplio por los dos
subdirectorios verificados. Ninguno de esos intentos ejecuto el borrado.

Liberar bloques dentro de ext4 permite volver a escribir desde ULTRA, pero
no reduce por si solo la longitud del archivo de 96 GiB en Windows. Ademas,
el respaldo completo consume espacio adicional en Windows. No confundir
espacio libre del contenedor con espacio libre del disco anfitrion.

Las futuras actualizaciones pueden volver a ocupar espacio y generar capas
anteriores. Este mantenimiento no instala una politica automatica de borrado:
una capa antigua puede conservar partidas o prefijos que deban recuperarse.
El respaldo permite recuperarlos sin mantener esa capa en el ext4 activo.
