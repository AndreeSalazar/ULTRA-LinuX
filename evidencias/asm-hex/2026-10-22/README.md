# Inspeccion hexadecimal del banco ASM

Fecha: 2026-10-22. Base ULTRA 3e86d9f mas el inspector de esta entrega.
Inspiracion analizada: TALK-CPU-x86-64 b5b8e881a33ffb8a9af4f63285dbfa6c0ae47e87.

Se inspeccionaron los cuatro ejecutables de owner-asm de la ronda 11, sin
ejecutarlos de nuevo. Cada subcarpeta contiene informe JSON, desensamblado y
HEX de cuatro simbolos. Los hashes de los binarios coinciden con los del
[registro previo](../../laboratorio-c/ronda11-116ad4197ab843599a2c700c94378ee2/owner-asm/hashes-taller.txt).

Un unico contrato exacto: ultra_abi_good = 48 89 E0 83 E0 0F C3 (siete bytes).
Los otros simbolos estan inventariados; no tienen prueba semantica estatica.
Los autocontroles del inspector pasaron en Windows y Linux: cuerpo valido,
cuatro mutaciones rechazadas, extraccion y simbolo ausente.

GNU objdump 2.47, taller QEMU Linux 7.2.11-arch1-1, sin discos personales.
SHA256SUMS registra los archivos de esta carpeta salvo el propio manifiesto.
No se afirma rendimiento superior, seguridad de cualquier ELF o ausencia de UB.
Ver [analisis y siguientes pasos](../../../docs/HEX-TALK-CPU.md).
