# Ethernet: recorrido, fallos y medicion W3g

El PC identifica una Realtek `10ec:8168`, RTL8168h/8111h, XID `541`, con
`r8169`. La evidencia original esta en `A:/ULTRA/informe/PC-configuracion.txt`
y su dmesg del perfil PC. Esos informes historicos no contienen una prueba de
transferencia ni acreditan velocidad negociada, perdida, latencia o saturacion.
Las cuatro imagenes instaladas antes de W3g coincidieron con los hashes de W3f:
esta investigacion no partio de una instalacion incompleta de Brave.

## Como recibe y transmite

```mermaid
flowchart LR
  cable[Ethernet y PHY] --> nic[Tarjeta y DMA]
  nic --> ring[Descriptores RX]
  ring --> napi[r8169 y NAPI]
  napi --> gro[Buffers skb y GRO]
  gro --> ip[IP y TCP/UDP]
  ip --> socket[Socket del programa]
  socket --> tls[TLS y HTTP en HTTPS]
```

En envio se recorre el camino inverso, pasando tambien por las colas `qdisc` y
los descriptores TX. No existe una correspondencia uno a uno entre una llamada
`send`, un segmento TCP, una trama Ethernet y una lectura de la aplicacion.
GRO/TSO agrupan trabajo; TCP ofrece un flujo, UDP conserva datagramas. HTTPS
anade autenticacion y cifrado mediante TLS; no sustituye la gestion de Ethernet.

Se reviso el C local fijado en el commit del kernel
`32c52ec87d6245ee2d12ed5dd509e40062939b50`, en
`drivers/net/ethernet/realtek/r8169_main.c`: seleccion de firmware,
`rtl8169_start_xmit`, recepcion y `rtl8169_poll`. RX comprueba propiedad del
descriptor, usa `dma_rmb`, contabiliza errores CRC/longitud, descarta ciertos
estados y entrega buffers a GRO. TX verifica espacio, trata fallos DMA y utiliza
barreras antes de entregar descriptores al hardware. Esta lectura no demuestra
ausencia de errores en todos los estados DMA o revisiones de silicio.

El aviso historico `jumbo features ... tx checksumming: ko` describe las
prestaciones **jumbo** de esa revision. No demuestra que falle el checksum de
todo el trafico con MTU 1500. No se habilitan jumbo frames en esta ronda.

## Correcciones de produccion

- Faltaba el firmware Realtek solicitado para XID 541: `rtl_nic/rtl8168h-2.fw`.
  Se incorpora `linux-firmware-realtek` en ambos perfiles. El kernel tiene
  `FW_LOADER_COMPRESS` desactivado: se descomprimen los `.zst` de `rtl_nic` y
  se exige el archivo plano antes de generar EROFS. No basta con instalar el paquete.
- Ambos perfiles buscan firmware en `/sistema/usr/lib/firmware`, disponible
  antes de cargar modulos. El generico conserva su funcion sin NVIDIA.
- `ultra-red-contadores`, C propio con el perfil GCC y guardianes del proyecto,
  lee bytes, paquetes, errores, descartes, carrier, MTU y velocidad nominal
  reportada. Lo ejecuta el observador como UID 1000 sin capacidades. No captura
  paquetes, genera trafico IP ni modifica la interfaz. `if_nameindex` puede
  consultar netlink internamente para enumerar las interfaces.
- Se validan longitudes y conversiones a `uint64_t` antes de multiplicar/sumar.
  No se usan conversiones que confundan texto invalido, desbordamiento o un
  archivo ausente con cero. El observador limita salida a 64 interfaces.
- El informe del escritorio conserva la preparacion de red y agrega consultas
  de solo lectura de `ethtool`: driver, enlace, offloads, colas, coalescencia y
  estadisticas. Tambien recoge contadores TCP/IP y softnet. Las consultas tienen
  limite de tiempo y tamano; prestaciones no soportadas quedan visibles.
- Se borra el log anterior de preparacion y se cierra el descriptor del bloqueo
  de sesion antes de invocar DHCP. Asi sus posibles procesos persistentes no
  heredan ese bloqueo; no se afirma haber reproducido un bloqueo en dhcpcd.

El C nuevo pertenece al observador de usuario. No instrumenta retroactivamente
el controlador, el firmware, dhcpcd, Brave ni TLS. Los perfiles de kernel generico
y PC tienen KFENCE, que muestrea ciertas asignaciones; no equivalen al perfil
vigia con KASAN/UBSan. Los guardianes no aprenden reglas automaticamente.

## Banco de fallos

`scripts/probar_red_paquetes.py` exige namespace de red con solo loopback y un
namespace de montajes privado. Crea un par veth y un segundo namespace; remonta
sysfs de forma privada para medir esas interfaces. No usa la tarjeta fisica,
las rutas del taller ni un objetivo externo. Al salir termina el proceso par y
los namespaces desaparecen. El observador C corre como UID 1000 sin capacidades.

Se ensayan siete escenarios: TCP base con escrituras/lecturas de tamanos
distintos; UDP base; perdida UDP del 100%; conexion TCP con perdida total y
timeout; TCP con retardo/duplicados; UDP con retardo/duplicados; recuperacion TCP.
La integridad de cada transferencia TCP de 1 MiB se compara mediante SHA-256.
UDP cuenta secuencias unicas y duplicadas; recibir cero bajo perdida total es el
resultado esperado. El descarte de netem se observa en `tc -s`; no se exige que
aparezca en el contador `tx_dropped` del dispositivo, que mide otro punto.

Estas pruebas ejecutan la pila virtual del kernel 7.2.11 del taller. No son una
prueba de la Realtek fisica ni un benchmark del kernel ULTRA 7.3. Las tasas
instantaneas de veth no son mejoras porcentuales frente a una version anterior.
No se altera trafico TLS en esta suite. `scripts/probar_red_tls.py`, en un
namespace separado con solo loopback, agrega tres controles HTTPS locales:
certificado desconocido rechazado; certificado confiado y nombre correcto
aceptados; nombre incorrecto rechazado incluso con certificado confiado.
Curl corre como UID 1000 sin capacidades y nunca usa `--insecure`. Las alertas
de cierre por rechazo de certificado se registran como resultado esperado.
No acredita cobertura de todas las versiones, algoritmos o estados de TLS.
La regresion QEMU aparte verifica HTTPS real y el sandbox del navegador.

Los 18 grupos C ejercitan cero, maximo uint64, desbordamiento, signos, texto,
lineas vacias, NUL embebido, byte alto, buffer sin NUL, nombres de interfaz y
datos ausentes. Se ejecutan con produccion generica, Zen 3/O3/LTO y diagnostico
ASan/UBSan; el analizador GCC tambien revisa observador y controles.

## Como medir en el PC

Arrancar cada perfil, abrir el escritorio, generar trafico normal y cerrar la
sesion. Salir del menu con Q exporta el informe a A:. Comparar el mismo cable,
puerto de router, destino, duracion y carga de escritorio. La velocidad de enlace
negociada y su duplex importan antes de cambiar afinidades o colas.

`scripts/resumir_red.py informe.txt` calcula tasas por intervalo con reloj
monotonico, nombre e indice de interfaz. Descarta saltos hacia atras, resets,
indices distintos y NA. No estima tasas con una sola muestra. Las lecturas
sysfs no son atomicas y sus bytes/paquetes no equivalen necesariamente a lo
medido en el cable. Los informes siguen separados por perfil/modo/compositor;
la misma interfaz transporta trafico de distintas aplicaciones.

Las siguientes optimizaciones necesitan esa medicion fisica: afinidad IRQ/RPS,
colas, coalescencia y offloads. Se cambia una variable, se compara throughput,
latencia de cola y CPU con igual carga, y se conserva solo una mejora repetible.
RPS ya esta compilado, pero activarlo no garantiza ganar rendimiento: puede
anadir IPIs y ser redundante con otras distribuciones de colas. No se aplican
flags de GCC de usuario al kernel ni al firmware como supuesto acelerador global.

## Fuentes primarias

- [NAPI: interrupciones, presupuestos y sondeo](https://docs.kernel.org/networking/napi.html).
- [Estadisticas de interfaces y significado de contadores](https://docs.kernel.org/networking/statistics.html).
- [Offloads de segmentacion y GRO](https://docs.kernel.org/networking/segmentation-offloads.html).
- [RSS, RPS, RFS y XPS](https://www.kernel.org/doc/html/latest/networking/scaling.html).
- [Paquete oficial linux-firmware-realtek](https://archlinux.org/packages/core/any/linux-firmware-realtek/).
- [Opciones de dhcpcd](https://github.com/NetworkConfiguration/dhcpcd/blob/master/src/dhcpcd.8.in).

Los resultados y hashes finales de esta ronda se guardan en `evidencias/red-w3g/`.
