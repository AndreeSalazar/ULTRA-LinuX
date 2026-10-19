# Banco de investigacion de C

Aqui viven las minas de ejecucion y compilacion, controles de operaciones
validas, pruebas de hilos, dominios ctype y aislamiento. El header local
incluye la API real de produccion. Los casos con UB son deliberados y solo
se ejecutan mediante el aislamiento de laboratorio.

`perfil-gcc.sh` contiene diagnostico ASan+UBSan y comparacion cruda. La entrada
`scripts/guardianes-lab.sh` carga primero produccion y despues este perfil.
TSan se prueba por separado. El comparador crudo nunca es un candidato de
produccion. Usar el perfil de produccion en un experimento significa probar
sus banderas dentro del laboratorio, no instalar el experimento en el sistema.

Desde PowerShell en la raiz del proyecto:

```powershell
./probar-guardianes.ps1 -Ronda 8
./probar-syscalls.ps1 -Ronda 8
```

Cada llamada crea un identificador unico, su arbol remoto y sus salidas.
Los informes quedan bajo `evidencias/laboratorio-c/ronda8-ID/` o
`evidencias/syscalls/ronda8-ID/`. El lanzador muestra la ruta solo despues
de finalizar correctamente. Los fallos conservan los registros remotos y
no copian informes de ejecuciones anteriores. No hay limpieza automatica.

Los scripts de cada suite siguen aceptando sus directorios ULTRA_*_OUT para
uso manual; quien los invoque directamente debe asignar uno por ejecucion.
Los procesos simultaneos aun comparten CPU y memoria de la VM: separar
directorios evita colisiones de archivos, no implementa cuotas agregadas.

El banco es extensible a familias generales de C, pero solo acredita los
casos que realmente prueba. Las 42 lineas de investigacion propuestas
siguen pendientes hasta obtener evidencia; moverlas de carpeta no las resuelve.
Ver [agenda de investigacion](../../../docs/PENDIENTES-GUARDIANES.md).
