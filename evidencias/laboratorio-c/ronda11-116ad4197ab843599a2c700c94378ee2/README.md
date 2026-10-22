# Ronda 11: propiedad C y puentes ASM

Fecha local: 2026-10-22. Base Git ff357ef mas las fuentes de esta entrega.
Ejecutada con probar-guardianes.ps1 -Ronda 11 en un staging nuevo de QEMU WHPX,
4 GiB, cuatro CPU, sin discos personales. Los registros incluyen versiones del
compilador/kernel y flags. No es un arranque del kernel ULTRA ni prueba GPU.

- [Nuevo banco](owner-asm-ronda11.txt): 644023 comprobaciones C y 1000 ciclos
  ASM de cuatro controles por perfil. Produccion O2/O3 static-pie y ASan/UBSan
  O1/O2 pasan. Son las mismas pruebas en cuatro configuraciones.
- [Campo historico](campo-minas-ronda11.txt): compilacion 36 verdes; ejecucion
  53 verdes, 1 neutra, 8 amarillos y 0 rojos de 62. La carrera de hilos sigue
  amarilla; no se transforma en proteccion por pasar el nuevo banco.
- Tambien pasan perfiles, evaluador, cruces, dominios, buffers, cadenas e
  instalacion. Sus resumenes estan en esta carpeta; cadenas, instalacion y
  owner-asm incluyen controles individuales y hashes del taller.

Los cinco hashes de fuentes de owner-asm/hashes-taller.txt se compararon con las
fuentes locales: coinciden. Los binarios quedan en el taller; aqui se conservan
registros sinteticos y sus hashes. SHA256SUMS cubre todos los archivos de esta
carpeta salvo el propio manifiesto, usando rutas relativas y texto LF.

Inspeccion readelf del ejecutable asm-prod-O2: segmento GNU_STACK RW, sin X.
No se afirma validacion CET, desenrollado, concurrencia, generaciones agotadas
por ejecucion, ni handles cruzados entre pools.

Ver [contrato, alcance y siguientes pasos](../../../docs/VIDA-UTIL-Y-ASM.md).
No se reconstruyeron ni instalaron imagenes de A: en esta ronda.
