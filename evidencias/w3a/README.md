# W3a: cierre del 2026-10-19

Compilacion de los perfiles generico y pc con el cargador corregido.
Kernel generico: 7.3.0-rc7-g32c52ec87d62; PC: 7.3.0-rc7-pc-g32c52ec87d62.
NVIDIA 615.78.08 solo en pc. EROFS: 212070400 bytes generico,
633470976 bytes pc. Los manifiestos SHA256SUMS vinculan los artefactos.

## Pruebas

| Salida | VERDE | ROJO |
|---|---:|---:|
| [generico BMO](generico-pruebas.txt) | 62 | 0 |
| [generico disco](generico-disco-pruebas.txt) | 56 | 0 |
| [pc BMO](pc-pruebas.txt) | 66 | 0 |
| [seleccion automatica](pc-auto-pruebas.txt) | 11 | 0 |
| [recuperacion](pc-auto-falla-pruebas.txt) | 6 | 0 |

Cada proceso de prueba termino con codigo 0. Los archivos *-serie.log son
las consolas correspondientes, con finales de linea y espacios finales
normalizados para Git; originales en build/. La recuperacion invalida el EFI del perfil
PC en una copia desechable del disco de QEMU, sin acceder a A:.
La ventana Wayland cubrio el 97 % de la pantalla; teclado y cierre de cage
con salida 0 comprobados. [Captura de PC en QEMU](pc-ventana-qemu.png).

La [regresion del cargador](../revision-cargador/c7c17edf010d4aca9274509e26fa96e0/informe.txt)
exige cero hallazgos: dependencia requerida ausente, omisiones entre hijos,
mensajes incompletos, contadores grandes y metadata ausente. Intercepta
la syscall de carga: no carga modulos reales.

## Instalacion

Se copiaron los dos perfiles a la particion FAT32 BMO, A:, disco 0,
particion 2. No se crearon entradas de firmware. Se conservaron copias
anteriores de EFI/EROFS en build/antes-w3a (fuera de Git).

[Verificacion SHA256](verificacion-instalacion.txt): los cuatro archivos
instalados coinciden con los artefactos locales probados. La imagen
persistente A:/ULTRA/datos.ext4 conserva el hash anterior:
4377A2E945ED062473E1004028917116F72F3C21E89ECDF92DC1DA7540321B12.

## Limites y siguiente paso

QEMU no tiene una GPU NVIDIA: nvidia-smi termina con el error esperado.
Esto comprueba presencia y ejecucion del programa, no aceleracion, Vulkan
ni salida fisica por RTX 3060. Faltan el arranque real, nvidia-smi, driver
DRM activo y monitor conectado a RTX. Los 11674 avisos de objtool observados
en la construccion PC son lineas, no un recuento de funciones unicas;
el estado SRSO requiere microcodigo y consulta de mitigaciones en el PC.

La sesion grafica sigue pendiente de retirar root y verificar credenciales,
capacidades y descriptores. Estas pruebas no certifican aislamiento global
ni ausencia de todo comportamiento indefinido en C. La propuesta del
propietario sigue en [Autoridad personal](../../docs/AUTORIDAD-PERSONAL.md).
