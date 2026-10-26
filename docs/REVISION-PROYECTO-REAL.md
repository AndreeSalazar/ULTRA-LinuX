# Revision de los archivos reales de ULTRA

Esta entrega aplica el banco al codigo propio. Se seleccionan 18 unidades C:
arranque/PID 1, particiones, hardware, tarjeta, EFI, perfiles, logo, sistema,
red, recursos, caja, implementaciones de guardianes y tres archivos del
frontend metal64 de CASM. El inventario distingue cabeceras, Rust, CASM y
laboratorios deliberadamente defectuosos. C++ propio en este alcance son casos
del museo; no se afirma haber revisado fuentes de Hyprland, Proton o los juegos.

## Que se comprobo

Cada unidad pasa por GCC O0/fanalyzer, GCC O2/avisos, Clang O0/analizador
estatico SARIF y Clang O2/avisos. Compilan dentro de Bubblewrap, con proyecto
en solo lectura, sin red del anfitrion y salidas limitadas al banco; no se
ejecutan PID 1, herramientas de discos ni servicios reales para analizarlos.
Las 72 comprobaciones finales quedan limpias en ese alcance estatico.

La revision inicial tenia tres diagnosticos:

- Caja: tipo del argumento de `setrlimit`. Ahora coincide con la API de glibc.
- Consola: se documenta y anota localmente la variable que conserva la propiedad
  de descriptores 0..2 para el analizador de GCC. No es estado leido por la app;
  quitarla producia un falso positivo de fuga en GCC. No se desactivan avisos
  globalmente ni se altera la permanencia de stdio.
- EFI: Clang perdia la relacion entre el numero de bytes copiados por el
  envoltorio de memcpy y la cuenta de BootOrder. El recorrido no demuestra UB
  real. La revision manual si encontro que longitudes impares se truncaban y
  cualquier error de lectura se trataba como lista ausente.

BootOrder ahora se decodifica explicitamente como UINT16 little endian,
rechazando registros parciales y exceso de capacidad antes de escribir salida.
Solo ENOENT se considera ausencia; los demas errores impiden eliminar una entrada.
La lectura preserva errno, reintenta EINTR y detecta un byte de exceso.
La creacion informa si la entrada existe pero BootOrder no se pudo actualizar.

El decodificador pasa 132355 combinaciones de longitud/capacidad y cuatro
contratos NULL/vacio tanto endurecido como con ASan/UBSan. Otro banco redirige
EFI a archivos temporales y comprueba ausencia, contenido valido, capacidad,
longitud impar, exceso, error de lectura y rechazo de borrado tras error.
Ninguna prueba escribe en el firmware fisico.

## Usarlo en el escritorio

La imagen PC incluye una fotografia de las fuentes revisadas y el SUPER GRAFO
actualizado. El menu ofrece **ULTRA · Revision del codigo**. Tambien:

```bash
ultra-revisar-proyecto abrir    # copia propia editable y SUPER GRAFO
ultra-revisar-proyecto estado   # fuentes/cabeceras/revisor/perfil y versiones vigentes
ultra-revisar-proyecto revisar  # repetir 72 comprobaciones, sin sudo
```

La copia vive en `~/.local/share/ultra/proyectos/ultra-<id>/` (o XDG_DATA_HOME),
no en las carpetas de juegos. La fotografia de EROFS permanece intacta.
`build/compile_commands.json` se genera con las rutas de esa copia para que el
grafo resuelva los includes y los compiladores. El inventario CSV indica el
alcance de las categorias del repositorio completo; no todas se distribuyen
en esta fotografia reducida.

En el proyecto Windows o en su copia Linux:

```bash
python3 scripts/revisar-proyecto.py inventario
python3 scripts/revisar-proyecto.py revisar
python3 scripts/revisar-proyecto.py validar
```

Inventario funciona sin ejecutar compiladores; revisar exige Linux, usuario sin
root y Bubblewrap. Validar compara SHA-256 de las fuentes seleccionadas, sus
cabeceras propias, perfil y revisor, y las versiones de GCC/Clang. Una modificacion
de cabecera se comprueba en la prueba de la herramienta instalada.
No autentica un informe frente a alguien que pueda falsificarlo ni acredita
que no cambiaron todas las bibliotecas/cabeceras de sistema. El alcance y la
identidad deben ampliarse al incorporar dependencias externas.

Cada ronda conserva logs y SARIF en `build/revision-proyecto/run-*`; el informe
actual se reemplaza atomicamente. Timeout, diagnosticos y errores del compilador
impiden aprobar. El instalador de Carpetas requiere una revision actual limpia
antes de publicar esta fotografia. El constructor conserva las auditorias ELF
de produccion y los bancos de arranque: una revision estatica limpia no los sustituye.

## Limites

Esto no instrumenta todos los paquetes del sistema, no certifica todo C/C++,
no modifica el backend LLVM ni promete cero UB o mejoras de FPS. La proteccion
de runtime sigue siendo la del perfil de produccion y las APIs adoptadas.
El generico conserva su papel de recuperacion; la entrega aqui actualizada es PC.
TSan, MSan, fuzzing con cobertura, AST de vida util C++ y dependencias externas
siguen el [plan del SUPER GRAFO](EVOLUCION-CASM-LLVM-SG.md).

## Entrega comprobada

El perfil PC `7.3.0-rc7-pc-g32c52ec87d62` se reconstruyo con el nuevo PID 1 y
caja. Pasaron sus auditorias ELF, el arranque FAT32/UEFI en QEMU, Hyprland,
SUPER GRAFO instalado, cabecera cambiada/restaurada, persistencia tras reiniciar
y apagado limpio. La grafica de esta VM es software; no mide rendimiento RTX.

Se instalo en A: y se verificaron los hashes del kernel y EROFS contra la
entrega probada. El instalador conserva `datos.ext4` y los informes previos
se respaldan en el proyecto Windows. La imagen mide 2057428992 bytes, 320 KiB
mas que la entrega anterior. Evidencias: `evidencias/revision-real/2026-10-25/`.
