# Guardian de fallos del kernel: CPU, eventos y diagnostico

2026-10-23, America/Lima. Propuesta de arquitectura; no es un parche instalado,
una nueva compilacion ni una captura realizada en el PC. Complementa las
[categorias de guardianes](CATEGORIAS-GUARDIANES.md).

## Que significa escuchar a la CPU

La CPU ejecuta instrucciones y entrega excepciones con un estado definido por
la arquitectura. Linux ya recibe ese estado mediante sus entradas de excepcion.
En x86-64, el diagnostico puede usar la instruccion interrumpida, registros,
codigo de error cuando exista y, en un page fault, la direccion de CR2 capturada
por el manejador correspondiente. No todas las excepciones tienen los mismos
campos. Hexadecimal es una representacion de esos bits, no otro protocolo ni
una instruccion que explique por si sola la intencion del programa.

Un page fault puede resolverse normalmente al preparar una pagina o gestionar
copy-on-write. No debe convertirse automaticamente en una alarma o panic.
Un puntero caducado que apunta a memoria aun accesible puede no generar ninguna
excepcion. Un error de logica tambien puede ejecutar instrucciones validas.
La CPU no conoce en general la vida util o los limites de un objeto C.

## Base local revisada

`scripts/build-baseline.sh` incluye y exige KFENCE en los perfiles produccion,
generico y pc. El perfil laboratorio solicita KASAN, UBSan y comprobaciones de
bloqueos. Esto describe la construccion, no demuestra las opciones activas del
kernel que esta arrancado en el PC. El script tambien desactiva DEBUG_INFO:
una futura simbolizacion debe preservar los artefactos exactos necesarios y
comprobar su compatibilidad con el perfil.

La configuracion historica `evidencias/baseline-config-previa.config` no se usa
como prueba de la configuracion actual. No se han modificado estas opciones en
esta ronda ni se han aplicado banderas del guardian de usuario a Kbuild.

## Primera etapa propuesta: evidencia estructurada

1. Inventariar configuracion final y eventos disponibles en el kernel exacto;
   registrar build-id si existe, release y hashes de artefactos. Mantener
   simbolos, modulos y fuentes coincidentes, sin confiar solo en uname.
2. Seleccionar tracepoints existentes segun disponibilidad y objetivo. Linux
   puede guardar campos binarios en sus buffers de tracing; su formato debe
   consultarse en esa compilacion. No asumir que todas las excepciones ya
   disponen de un tracepoint con todos los registros deseados.
3. Recoger un conjunto acotado de eventos con filtros, contador de perdida y
   presupuesto de memoria. Evitar trazado global permanente y formateo costoso
   en rutas criticas. Un buffer circular tambien puede sobrescribir eventos;
   ausencia de registro no prueba ausencia de fallo.
4. Decodificar fuera de la ruta critica: mostrar valores hexadecimales y, con
   artefactos coincidentes, simbolos, desplazamiento y codigo fuente disponible.
   Conservar los datos originales y la version del decodificador.
5. Clasificar: evento normal, rechazo esperado, fallo de usuario, defecto del
   kernel, informe del detector o dato insuficiente. Las acciones las conserva
   el subsistema correspondiente; observar no autoriza continuar tras
   corrupcion ni saltar una instruccion arbitrariamente.

Campos candidatos: version del registro, tipo y origen, reloj identificado,
CPU, PID/TID cuando proceda, identidad de compilacion/modulo, IP, privilegio,
codigo de error y direccion relevante solo cuando esten disponibles, mas
secuencia/perdidas. Indicar explicitamente los campos ausentes; no rellenarlos
con cero como si se hubieran medido. No es un ABI binario nuevo ya implementado.

Los registros pueden contener direcciones e informacion de procesos: acceso
local restringido y exportacion revisada. La captura en RAM no sobrevive por
si sola a reinicio o perdida de energia. Persistencia mediante pstore u otro
mecanismo requiere soporte, configuracion y pruebas propios; no se promete.

## Caza y fluidez, con perfiles separados

- Normal: evaluar KFENCE con su muestreo y coste reales; no protege cada
  asignacion. Capturas dirigidas para problemas concretos.
- Laboratorio: KASAN/UBSan y, en otra configuracion apropiada, analisis de
  carreras; ejecutar casos deliberados solo en QEMU sin discos personales.
- Fluidez: comparar carga fija con captura apagada y encendida; tiempos por
  fotograma, p95/p99, picos, CPU y memoria. Declarar perdidas y carga del propio
  diagnostico. Mas eventos no implica mejor rendimiento.

Antes de desplegar: inventario del kernel exacto, lector que rechace registros
truncados/incompatibles, pruebas de perdida y volumen, controles normales y
fallos aislados, arranque QEMU y luego validacion nativa con retorno al kernel
anterior. El primer cambio debe reutilizar infraestructura existente; solo si
falta un evento necesario se disena y revisa un tracepoint propio en la ruta
concreta. No reemplazar IDT ni el manejo de excepciones para este objetivo.

## Fuentes primarias consultadas

- [Manejo de excepciones x86 en Linux](https://www.kernel.org/doc/html/latest/arch/x86/exception-tables.html):
  contexto historico y flujo de page faults; los detalles exactos se contrastan
  con los fuentes de la version elegida.
- [Eventos de tracing](https://docs.kernel.org/trace/events.html): campos,
  formatos del flujo binario, filtros y eventos disponibles.
- [KFENCE](https://docs.kernel.org/dev-tools/kfence.html): deteccion muestreada
  y compromisos entre cobertura y coste.

Estado: diseno documentado. Inventario de kernel desplegado, recolector,
decodificador, medida y despliegue pendientes.
