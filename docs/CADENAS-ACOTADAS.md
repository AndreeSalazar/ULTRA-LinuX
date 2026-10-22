# Primera entrega critica: cadenas con dos limites

2026-10-22. API y primer consumidor adoptados en fuentes, verificados en QEMU
del taller. **No se reconstruyeron ni sustituyeron las imagenes instaladas en A:.**

## Problema y cambio

`ultra_copy`/`ultra_append` reciben capacidad del destino, pero requieren que el
origen sea una cadena C valida. Su strlen no puede inferir la extension del
objeto de origen. Un llamador que pasa bytes sin NUL incumple ese contrato y
puede terminar leyendo fuera del objeto. No se cambia silenciosamente la
semantica de esas interfaces ni se considera invalido todo uso existente.

[ultra_string.h](../src/guardian/produccion/ultra_string.h) agrega copy y append
con capacidad explicita de origen y destino. Comprueba terminadores dentro de
esos limites, rechaza espacio insuficiente sin truncar y admite solapamiento
mediante memmove. Todos los rechazos dejan el destino intacto; el resultado
debe comprobarse y GCC rechaza ignorarlo en el perfil estricto.

La suma del terminador no desborda: una longitud aceptada es estrictamente
menor que la capacidad de origen. Append compara contra capacidad restante
antes de sumar/desplazar. No hay reservas dinamicas ni operacion de escritura
antes de completar las comprobaciones.

Los spans tienen que describir objetos reales, vivos y accesibles, sin cambios
concurrentes. Un limite inventado no valida un puntero, su mapeo o su vida util.
No se promete lectura constante en tiempo: el trabajo depende del prefijo
examinado. No hay porcentaje de aceleracion medido en esta ronda.

## Evidencia final

[Ronda completa](../evidencias/laboratorio-c/ronda10-2a55e6d3d52a4a81a3bb8e9e2c493fc7/README.md):
GCC 16.2.1 20260810, Linux 7.2.11-arch1-1 del taller QEMU, x86-64. No es un
arranque de estos cambios sobre el kernel ULTRA 7.3 del PC.

| Perfil | Matriz | Frontera de pagina | Parser real | Contraste antiguo |
|---|---|---|---|---|
| Produccion O2, static-pie | 10368 casos | 5 controles | 5 controles | SIGSEGV/rc 139 |
| Produccion O3, static-pie | 10368 casos | 5 controles | 5 controles | SIGSEGV/rc 139 |
| ASan/UBSan O1 | 10368 casos | 5 controles | 5 controles | ASan SEGV/rc 1 |
| ASan/UBSan O2 | 10368 casos | 5 controles | 5 controles | ASan SEGV/rc 1 |

Cada matriz contiene 1308 aceptaciones y 9060 rechazos. Son las mismas
combinaciones en cuatro perfiles, no 41472 casos distintos. Se contrastan
copy/append, capacidades pequenas y cero, NUL en distintas posiciones, origen
y destino dentro del mismo arreglo, preservacion de bytes y canarios mediante
un oraculo de snapshot independiente. Hay ademas cuatro controles de argumentos
nulos por perfil.

La frontera usa mmap/mprotect con una pagina PROT_NONE real: se prueba origen
sin NUL, NUL en el ultimo byte, destino corto, destino sin NUL y ajuste exacto.
El contraste pasa deliberadamente ese origen sin terminador a la API antigua;
el fallo esperado debe tener evidencia de memoria/ASan, no solo salida distinta
de cero. Un timeout o fallo de aislamiento no acredita la deteccion.

Una prevalidacion fue rechazada por un aviso de inicializacion en el harness al
usar el wrapper memcpy del proyecto sobre su snapshot. Se inicializaron los
buffers de prueba y se repitio la revision, sin retirar avisos ni banderas.
Los resultados finales corresponden a esa fuente corregida.

Todos los programas se ejecutaron en namespaces nuevos, sin red ni home del
taller, con limites de tiempo/CPU, archivos, descriptores y stack. No se
conectaron C:, D:, A: ni el contenedor personal a las pruebas. El perfil ASan
deja actuar a su manejador para obtener diagnostico; produccion conserva el
manejador del proyecto. El presupuesto total de memoria es el de la VM de 4 GiB;
el launcher existente no impone un cgroup de 512 MiB por proceso.

## Primer consumidor y siguientes pasos

`line_value` de [tarjeta.c](../src/tarjeta.c) copia desde su arreglo temporal
con extension conocida mediante la nueva API. Un destino corto ya no queda
truncado/modificado antes de devolver false. El test incluye el archivo C real;
link-GC elimina las funciones de pantalla que no usa ese harness. No se prueba
con ello el arranque completo o toda la interfaz de la tarjeta.

El ejecutor general `probar-guardianes.ps1` incorpora las nuevas suites, informes
por ronda y controles individuales. Se repitio tambien el campo completo de
62 minas: 53 VERDE, 1 NEUTRA, 8 AMARILLO y 0 ROJO. Las 36 minas de compilacion
fueron rechazadas. No se cerraron por ello los ocho huecos globales.

Siguiente: revisar consumidores con limites de origen demostrables antes de
migrarlos; los [fallos de inicializacion del propio guardian](GUARDIAN-INSTALACION.md)
ya se verificaron en la ronda 10. La API de propiedad tiene banco propio;
su integracion en consumidores sigue pendiente.
La [promocion posterior a produccion](ELF-PRODUCCION.md) reconstruyo ambos
perfiles, verifico su tarjeta y arranque en QEMU y los copio en A:. El nuevo
arranque nativo aun necesita confirmacion del propietario.
