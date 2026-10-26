# Banco LLVM: C17 / C++20

Codigo incorrecto deliberado en `casos.json`, bajo el alcance CASOS-MUSEO de la
carpeta laboratorio. No ejecutar estos ejemplos en produccion.

```
python3 src/guardian/laboratorio/llvm/probar.py
ultra-carpetas --consulta cobertura_ub /ruta/al/proyecto '{"limite":5}'
```

Solo usuario sin root en QEMU; Bubblewrap obligatorio para cada compilacion y
ejecucion. Fuentes fijadas por este corpus, sin flags de proyectos externos.
Clang/Clang++ del sistema, C17/C++20, O0/O2, ASan y UBSan en carriles separados.
`vptr` se activa explicitamente para C++; no esta incluido en `undefined`.

Los casos `*-volatile` conservan una exclusion documentada de Clang: NULL y
alineacion no se instrumentan sobre punteros a tipos volatile. Sus ocho variantes
malas son `known_gap`, **no passed**. Un SIGSEGV sin el diagnostico concreto no
acredita deteccion. El banco no pretende que volatile quite la UB.

Cada ejecucion conserva fuentes, flags, logs y `report.json` en su run independiente.
`build/llvm-ub/current.json` es una copia atomica del ultimo informe; no borra los
anteriores. El proceso devuelve cero cuando no hay fallos inesperados; **esto no
significa que los known_gap hayan quedado protegidos**. Consultar los estados.

El aislamiento oculta home/proyecto y separa red, PIDs y otros namespaces.
Las herramientas/bibliotecas se montan en solo lectura; solo /work persiste
escrituras. Hay limites de CPU, archivo y tiempo; no un limite AS virtual que
impida arrancar el shadow de ASan. No incluye un presupuesto RSS por cgroup,
ni reemplaza el aislamiento de QEMU ni una auditoria del kernel.

TSan, MSan, CFI/LTO y fuzzing quedan pendientes. Tampoco cubre OpenGL, Vulkan,
los juegos ni todo el codigo del sistema. La consulta SG lee evidencia historica
del corpus; no ejecuta casos ni declara seguro el proyecto abierto.

[Investigacion, referencias y siguientes pasos](../../../../docs/EVOLUCION-CASM-LLVM-SG.md).
