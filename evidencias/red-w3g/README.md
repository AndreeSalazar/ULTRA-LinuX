# Evidencia Ethernet W3g — 19 de octubre de 2026

Investigacion y alcance en [RED-W3G.md](../../docs/RED-W3G.md).

| Comprobacion | Resultado |
|---|---|
| QEMU generico, dos arranques y escritorio/Brave | 100 correctas, 0 fallidas |
| QEMU PC, dos arranques y escritorio/Brave | 104 correctas, 0 fallidas |
| Banco veth/netem en el taller | 7 escenarios correctos |
| HTTPS local con certificado/nombre validos e invalidos | 3 controles correctos |
| C del observador | 18 grupos, generico + Zen 3 + ASan/UBSan |
| GCC analizador O0 y avisos O2 | Observador y controles aceptados |
| Instalacion A: | 4 archivos iguales a los probados; datos.ext4 sin cambio |

`generico/` y `pc/` conservan consola de comprobaciones, transcript de arranque,
informe real exportado, tasas de red calculadas, versiones de paquetes y hashes
de las imagenes. El renderer de QEMU es software, con NIC virtio y NAT. No es
el rendimiento de la Realtek/RTX fisicas. Los contadores de una interfaz son
globales, no una atribucion exclusiva a Brave.

`red-paquetes.json` documenta transferencias TCP con SHA-256, secuencias UDP,
perdida total, retardo, duplicados, recuperacion y estadisticas de qdisc. Usa el
binario C generico exacto de la imagen como UID 1000 sin capacidades. Es una
prueba de veth/netem sobre el kernel 7.2.11 del taller; las tasas durante estas
transferencias cortas no constituyen un benchmark comparativo o de cable.

`red-tls.json` conserva los tres resultados HTTPS locales y los cierres de
conexion esperados tras rechazar un certificado. No se desactivo la verificacion.
La suite QEMU tambien cargo una pagina HTTPS externa con certificados validos.

`red-validacion.txt`, `controles-*`, `analizador.txt`, `compilar-*`, `elf-*` y
`entorno.txt` registran las revisiones de C. Un log de compilacion vacio significa
que no hubo diagnosticos en ese paso, no una prueba universal de ausencia de UB.
Los avisos de `-fhardened` con LTO quedan conservados. `red-perfiles.txt` confirma
RELRO, BIND_NOW/PIE, configuracion y SHA del firmware en los ejecutables/imagenes
finales; `elf-*` contiene las marcas CET, no una prueba por si sola de su
activacion durante toda ejecucion.

`fuentes.sha256` identifica la fuente de los controles originales. Luego solo
se aclaro un comentario de `red.c`: `if_nameindex` puede usar netlink internamente.
`red-binarios-finales.txt` registra el SHA de la fuente final y recompilaciones
que producen exactamente los mismos binarios generico/PC ya probados. No cambio
su comportamiento.

El firmware plano `rtl8168h-2.fw` mide 976 bytes y tiene SHA-256
`0b4beab008e308f28296c13188ee23edfeff7d865e398a7ade170495c8cec7e6`.
Procede de `linux-firmware-realtek 20260916-1`. El generico y el PC incluyen la
misma copia. La presencia y ruta se comprobaron en el arranque QEMU; cargarlo
en la Realtek fisica requiere el siguiente arranque real.

`instalacion-verificada.json` compara A: con las imagenes probadas y conserva el
hash de datos.ext4 antes/despues y la ruta del respaldo local. No se formateo,
no se regeneraron datos y no se alteraron entradas de firmware de Windows.

Los archivos originales de los registros se preservan en `registros-originales.zip`.
Las copias de texto legibles normalizan finales de linea y espacios finales para
que puedan revisarse en Git. El ZIP conserva los bytes originales de los registros.
