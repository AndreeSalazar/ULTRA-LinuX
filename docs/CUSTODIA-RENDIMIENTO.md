# Custodia: primer ahorro medido, ticket de un byte inline

Actualizacion posterior: [primer consumidor local y lector C de recursos](RECURSOS-C-PRODUCCION.md).
Los resultados y limites del microbenchmark descrito abajo se conservan.

2026-10-23, America/Lima. Cambio aplicado a la biblioteca candidata de owner y
probado con Custodia; sin consumidor desplegado, imagen nueva ni cambios de A:.
No se alteran HZ, interrupciones, governor o mitigaciones. Se reduce trabajo por
pedido; no se han medido ciclos de CPU ni ticks del kernel.

## Cambio y contrato

Owner almacena los payloads de exactamente un byte dentro del propio slot.
Alloc reinicializa el byte a cero; free mantiene incremento de generacion y
retirada sin vuelta a cero. Identidad del pool y limites no cambian. Release
distingue el byte interior del heap y omite liberaciones de slots vacios.
Destroy no libera un puntero interior. Los demas tamaños siguen usando calloc.

Custodia ya usaba ese byte para tickets: no necesita cambiar su protocolo,
titular, rango, cuotas o retirada. No se reutiliza la identidad antigua.
Un fallo forzado de calloc ya no obliga a fallar un ticket de un byte, porque
esa ruta no llama al asignador. Las pruebas mantienen el fallo esperado para
payloads mayores. No se elimina ninguna comprobacion de la operacion.

## Metodo y evidencia

[Evidencia completa y hashes](../evidencias/custodia/custodia-inline-c71ce1555dc44713a2ac99d4ad0b2c43/SHA256SUMS).
Referencia cf52e2f, preservada antes del cambio y contrastada con Git. GCC
16.2.1 20260810, kernel del taller 7.2.11-arch1-1 x86-64, QEMU WHPX con 4 CPU
virtuales y 4 GiB configurados. Host y VM introducen variabilidad; no son
mediciones nativas de ULTRA ni pruebas estadisticas universales.

`medir-custodia.c` usa CLOCK_MONOTONIC y CLOCK_PROCESS_CPUTIME_ID. Incluye en
ambas rutas comprobaciones de estado, comparacion completa de bytes y checksum.
Los buffers externos se preparan fuera del intervalo.

- hot: reutiliza un gestor/prestamo para todos los pedidos; incluye creacion,
  copia inicial, retirada y destruccion final dentro del total.
- cycle: crea gestor, presta, completa un pedido y destruye en cada iteracion.

Ambas versiones usan static-pie y las mismas banderas de produccion, O2 u O3,
sin ASan. Se ejecutan en el mismo aislamiento: 12 pares por caso, orden
before/after alternado y un calentamiento previo en proceso separado por ruta.
Ese calentamiento no conserva el heap del siguiente proceso. No se descartan
muestras medidas. No se mide SSD, colas 4/16 ni otros tamaños de payload.

El conteo usa binarios distintos con wrappers de calloc/free, reiniciados tras
el arranque de libc y preparacion de relojes, y capturados antes de imprimir.
Los tiempos instrumentados no se usan para comparar velocidad. RSS es pico
del proceso, no memoria exclusiva por ruta. La resolucion reportada del reloj
no demuestra precision real de un nanosegundo en la VM.

## Comparacion mas larga

Reduccion por par = 100 * (1 - tiempo_after / tiempo_before); tabla con su
mediana, no cociente de medianas ni mejora de FPS.

| Caso por muestra | O2: reduccion mediana pareada | O3: reduccion mediana pareada |
| --- | --- | --- |
| hot, 64 B, 1000000 pedidos | 10.00 % | 14.69 % |
| hot, 4096 B, 200000 pedidos | 4.44 % | 5.31 % |
| cycle, 64 B, 100000 iteraciones | -2.92 % (regresion observada) | 19.90 % |

Ejemplo hot/64 O2: medianas totales de 39.04 ms antes y 35.09 ms despues. No
son percentiles por pedido o fotograma; 12 pares no permiten declarar una cola
p99 fiable. Minimos/maximos, CPU, RSS y muestras se conservan en bench-largo/.

El conteo separado de hot/64 pasa de 1000004 llamadas calloc y free no-NULL a
4 de cada una. Son cuatro de inicio/payload; los tickets dejan de reservar y
liberar heap por pedido. hot/4096 pasa de 200004 a 4; cycle, de 500000 a
400000, porque sigue creando gestores continuamente. El runner comprueba ambos
contadores. No se afirma ahorro de RAM residente a partir de ellos.

Se conserva como candidato medido para reutilizacion del gestor. La regresion
O2/cycle impide describirlo como mejora universal o promocion general: un
consumidor que cree/destruya continuamente necesita evaluar ese coste antes
de integrarlo. Prueba nativa y carga representativa siguen pendientes.

## Iteraciones y regresiones conservadas

bench-final/ guarda la primera variante inline con free(NULL) sobre slots
vacios; su comparacion corta mostro -7.12 % en cycle/O3. Su fuente queda en
owner-candidato1.txt, contrastado con el hash. Tras omitir liberaciones vacias,
bench-v2/ conserva la segunda variante con corridas cortas y bench-largo/ las
mas largas. No se ocultan resultados cortos ni se escoge la mejor muestra.
metodo-corto/ conserva los fuentes de aquellas mediciones, contrastados con
sus hashes; los binarios no se copian a Git.

Pasan en prod-O2, prod-O3, diag-O1 y diag-O2:

- Owner: 674038 checks por perfil, incluidas transiciones 1/2 bytes, puesta a
  cero, asignador fallando, caducados y destroy con objeto inline vivo.
- Agotamiento de identidad/generacion y 1000 ciclos de cuatro controles ASM.
- Custodia: 50076 checks dirigidos, 192000 operaciones por perfil contra el
  ledger (531404 checks) y 8574533 checks de fronteras/guard pages.
- Cuatro mutantes detectados; inspeccion HEX y negativos pasan. Los tres
  controles ASan de callers invalidos siguen diagnosticando limites abiertos.

No hubo fallos inesperados en regresiones positivas. Los checks repetidos no
son clases independientes de UB; los errores de minas/mutantes son controles
esperados. Fuentes/herramientas finales y baseline se contrastaron con hashes.

Reproduccion: `scripts/medir-custodia.sh` requiere ULTRA_CUSTODIA_BASE con
guardian/ de la referencia y ULTRA_CUSTODIA_BENCH_OUT nuevo. Antes se ejecutan
campo-punteros.sh y campo-custodia.sh con salidas nuevas en el taller.
Pendientes: consumidor real, medida nativa, otras colas/cancelaciones, identidad
autenticada y concurrencia. Ver [reglas y estados](REGLAS-RENDIMIENTO.md).
