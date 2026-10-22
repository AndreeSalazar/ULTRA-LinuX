# Ethernet: capacidad, latencia y prioridades

## Resultado en el PC

La RTL8168h/8111h usa r8169 y anuncia 1000baseT/Full. El otro extremo anuncia
10/100, sin 1000baseT. El enlace queda en **100 Mbps, full duplex, autonegociacion
activa**. Windows, con su controlador Realtek, tambien informa 100 Mbps.
El usuario confirma que el cable va directamente al router. Su modelo no esta
identificado: no se distingue todavia entre puerto limitado, configuracion o
condiciones fisicas/negociacion del enlace. No se fuerza 1 Gbps ni se corta la red.

En el historial Linux el pico medio por intervalo es **98,34 Mbit/s**. Se registran
cero rx_errors, tx_errors, rx_missed y align_errors en la NIC; tampoco aparecen
descartes o time_squeeze en las 12 filas softnet. Estos puntos de medida no
demuestran ausencia de perdidas TCP o de congestion en el router/Internet.

TSO, GSO, GRO y checksum ya estan activados. RSS se informa off/fixed; no se
promete habilitarlo por software. RPS tiene mascara cero. No hay evidencia
actual de saturacion que justifique enviar paquetes a todos los nucleos.
XPS no aporta seleccion de cola cuando solo hay una cola de transmision.

Fuente: [Linux: scaling](https://docs.kernel.org/networking/scaling.html).
Los anuncios, velocidad, offloads y coalescencia se consultan con
[ethtool](https://man.archlinux.org/man/ethtool.8.en).

## Cambios preparados y medidos

`ultra-informe-red` amplía las consultas de solo lectura: qdisc, politica TCP,
buffers automaticos, SOFTIRQ, anillos, pause frames, EEE, downshift y afinidad
MSI. Las consultas no soportadas se muestran como tales, no como valores cero.
El informe termina con presupuesto de tiempo y limites de salida. Se comprobo
su sintaxis y su ejecucion acotada en el taller. Este cambio de fuente aun no
esta reempaquetado en A:.

El kernel ULTRA actual no tiene FQ-CoDel, FQ, CAKE ni BBR; tiene CUBIC, RPS/RFS,
XPS y BQL. El fragmento `config/red-laboratorio.config` prepara FQ-CoDel/FQ y
TBF/netem para una futura construccion de laboratorio. No se fusiona ni cambia
la politica por defecto automaticamente.

FQ-CoDel es candidato para reducir espera entre flujos en la cola de salida.
No aumenta la velocidad negociada y no controla directamente las colas del
router o del servidor de descarga. El kernel documenta la seleccion de
[default_qdisc](https://docs.kernel.org/admin-guide/sysctl/net.html) y la
[pagina de FQ-CoDel](https://man.archlinux.org/man/tc-fq_codel.8.en) describe sus parametros.

Se comparo FIFO de 1000 paquetes con FQ-CoDel por defecto en el kernel
**7.2.11 del taller**, con dos namespaces de red privados y un cuello artificial
de **10 Mbit/s**. Un flujo TCP compite con sondas UDP pequenas. Orden:
FIFO, FQ-CoDel, FQ-CoDel, FIFO; seis segundos de generacion y dos de drenaje.

| Cola | RTT p95, prueba 1 | RTT p95, prueba 2 |
|---|---:|---:|
| FIFO | 122,23 ms | 125,71 ms |
| FQ-CoDel | 22,85 ms | 24,37 ms |

Todas las sondas recibieron respuesta dentro de la ventana. Los bytes TCP
recibidos difieren: las colas tienen distinto drenaje y dinamica; no se
presentan como una medida equivalente de velocidad sostenida. Son cuatro
ejecuciones cortas en software, no estadistica de hardware o WAN.

Resultados en `evidencias/red-optimizacion-2026-10-22`. Ninguna interfaz fisica,
router, DNS, MTU, TLS, politica TCP o mascara CPU de produccion fue modificada.

## Orden de trabajo para aplicar una optimizacion

1. Comprobar modelo/puerto del router y el enlace con un cable conocido. Si
   solo anuncia 100 Mbps, ningun ajuste de colas da 1 Gbps en ese enlace.
2. Construir ULTRA de laboratorio con el fragmento, verificar simbolos y probar
   FIFO frente a FQ-CoDel en ese kernel. Registrar throughput, RTT p50/p95/p99,
   CPU, retransmisiones y descartes, en reposo y bajo carga.
3. Probar en el PC con destino LAN propio y documentar una forma de volver a la
   configuracion anterior. Separar la capacidad LAN de la velocidad del proveedor.
4. Activar una politica solo si el resultado medido compensa sus costes. No se
   activan globalmente busy polling, buffers gigantes, BBR, jumbo frames o RPS
   indiscriminado por la sola expectativa de que mas ajustes den mas velocidad.
