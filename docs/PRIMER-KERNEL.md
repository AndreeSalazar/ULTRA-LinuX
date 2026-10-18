# Primera compilación y pantalla en QEMU

## Qué se hereda y qué es propio

Se conserva el kernel oficial fijado en el registro de origen. `src/init.c` es un primer programa propio en C, ejecutado como PID 1 dentro de Linux; no sustituye el kernel. El constructor usa GCC, Kbuild y Make. Arch es el taller de compilación, no una afirmación de que el sistema final ya sea una distribución completa.

El perfil `tiny64-baseline` es una referencia mínima para QEMU: consola, teclado PS/2, UART e initramfs incorporado. No pretende mantener todo el ecosistema en esa configuración. El futuro perfil de escritorio/metal deberá añadir controladores, red, almacenamiento, bibliotecas y funciones de compatibilidad.

## Estado comprobado

- El init se compiló con GCC como ELF x86-64 estático, con avisos tratados como errores.
- La configuración habilita 64 bits, ELF, initramfs y consola serie/VGA.
- La construcción se detuvo ordenadamente para preparar aceleración.
- Todavía no hay kernel completo ni prueba de arranque de este perfil.
- Los lanzadores de compilación y pantalla están preparados; falta validarlos con el kernel terminado.

## Acción manual: SVM y reinicio

Windows informó `VirtualizationFirmwareEnabled=false`. QEMU admite WHPX, pero falta habilitar la virtualización en firmware y reiniciar. HypervisorPlatform quedó solicitado con NoRestart; no se reinició automáticamente.

1. Guarda el trabajo de Windows.
2. Reinicia y entra en la BIOS con Supr/Delete.
3. Busca **SVM Mode** y actívalo. MSI documenta la ruta general **OC → CPU Features → SVM Mode**; la ubicación puede variar según la versión de BIOS. No se requiere flashear el firmware para este paso.
4. Guarda los cambios y vuelve a Windows. En Administrador de tareas → Rendimiento → CPU, comprueba que Virtualización aparece habilitada.
5. Inicia el taller con el lanzador siguiente. Su prueba WHPX debe aprobarse antes de abrir el disco; si falla, se muestra el motivo y no se cambia silenciosamente a TCG.

Referencias: https://us.msi.com/faq/2581 y https://www.qemu.org/docs/master/system/whpx.html.

## Reanudar y ver

En PowerShell, desde esta carpeta:

```powershell
.\preparar-entorno.ps1 -Accion Iniciar -Acelerador whpx
.\preparar-entorno.ps1 -Accion Estado
.\construir-kernel.ps1 -Accion Compilar
.\construir-kernel.ps1 -Accion Estado
```

El primer comando abre el taller de compilación; no muestra aún el kernel reconstruido. Cuando el registro muestre la construcción terminada:

```powershell
.\construir-kernel.ps1 -Accion Descargar
.\construir-kernel.ps1 -Accion Ver
```

Descargar exige la marca de construcción completa y verifica SHA-256. Ver exige el artefacto verificado y abre otra máquina QEMU sin disco físico. La consola esperada muestra `LINUX RECONSTRUIDO: INIT C READY`. Prueba `help`, `version`, `pid`, `mem` y `echo hola`. `halt` solicita apagar esa prueba.

El init limita la entrada a 255 caracteres; una línea demasiado larga debe descartarse con un motivo. Esta comprobación se encuentra implementada, pero su prueba de ejecución sobre el kernel nuevo sigue pendiente. También quedan pendientes la interacción por teclado, la captura de pantalla y la comprobación de montaje de proc/sys/dev en el arranque real del perfil.

## Cómo mejorar con evidencia

Primero guardar kernel, configuración, versión de GCC, hashes y resultado de arranque. Después cambiar una cosa por experimento y comparar con esa referencia: tiempo de arranque, memoria, rendimiento y latencia bajo cargas definidas. Un cambio favorable a una carga no demuestra mejora universal.

Arch inspira la organización y el control de paquetes; CachyOS aporta ejemplos de perfiles y cambios del kernel que se pueden estudiar. No se han aplicado sus parches ni se declara superioridad. Fuente: https://wiki.cachyos.org/features/kernel/.

El paso al metal se preparará después de demostrar este arranque y de contar con un perfil completo para la placa. El núcleo mínimo de esta prueba no se presenta como sistema de escritorio ni imagen instalable en SSD.