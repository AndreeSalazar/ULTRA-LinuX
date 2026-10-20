# Equipo real: 10 ms de diferencia y correccion de DHCP tardio

La tarjeta fotografiada muestra **3290 ms** hasta completar la preparacion del
hardware. La flecha de **10 ms** comparaba con el arranque anterior de 3300 ms:
es una diferencia de aproximadamente 0,30 %, no un arranque completo de 10 ms.
No incluye el tiempo anterior al kernel (firmware/seleccion F11) ni abrir el
escritorio o Brave. Dos arranques no bastan para atribuir esa diferencia a una
optimizacion; puede ser variacion normal.

El registro confirma `uptime_ms=3290`, frente a `3300` en el arranque PC anterior.
La fase guiada de lectura/carga de modulos duro 2392531 us, aproximadamente
2,393 s, incluida en ese tiempo. El generico anterior registro 930 ms. Los
perfiles no realizan el mismo trabajo: el PC incluye NVIDIA. No se presenta la
diferencia entre ambos como mejora equivalente de rendimiento.

## Lo que si funciono en el metal

El informe del intento confirma `modo=nvidia`, `compositor=hyprland`, `salida=0`,
la RTX 3060 y firmware NVIDIA 615.78.08. Hyprland reporta backend DRM, GL 3.2,
sincronizacion explicita soportada y monitor HDMI-A-1 a 1920x1080/60 Hz. Las fotos
muestran Brave y la pagina local WebGL. El uso de GPU confirma actividad real,
pero el informe no atribuye cada operacion grafica a una aplicacion concreta ni
exporta por si solo todo `brave://gpu`.

En 24 muestras de la sesion:

| Medida | Minimo | Maximo muestreado |
|---|---:|---:|
| RAM estimada de todo el sistema | 476,48 MiB | 1057,08 MiB |
| PSS de UID1000, muestras completas | 1,75 MiB | 816,26 MiB |
| VRAM de la RTX completa | 1 MiB | 236 MiB |
| Ocupacion GPU | 0 % | 13 % |
| Temperatura GPU | 43 C | 54 C |
| Potencia GPU | 14,76 W | 43 W |
| CPU agregada, 23 intervalos | 0,49 % | 10,70 % |

El minimo inicial se tomo antes de que todas las aplicaciones estuvieran listas;
no es el consumo estable del escritorio. Los maximos son muestreados, no picos
absolutos. RAM estimada, PSS y VRAM son medidas diferentes; no se suman ni se
atribuyen enteramente a Brave o a Hyprland. No hay informe nuevo del escritorio
generico para comparar su consumo en el metal.

## Por que la pagina local abria y faltaba internet

La Realtek ya informa firmware `rtl8168h-2_0.0.2 02/26/15`. El enlace esta activo,
negociado a **100 Mb/s full duplex**. La tarjeta anuncia capacidad 1000baseT/Full,
pero el otro extremo solo anuncia 10/100. No se fuerza gigabit: hay que revisar
puerto/equipo conectado y cable para determinar la limitacion real.

El informe dice que no habia ruta IPv4. Al comenzar la medicion, a los 26,118 s
de uptime, `carrier=0`; a los 31,352 s ya era 1. La preparacion antigua subia la
interfaz, esperaba solo un segundo y, si no habia carrier, salia **sin iniciar
dhcpcd**. Cuando el enlace aparecia despues, nadie solicitaba la direccion.
Al finalizar solo habia 3 paquetes RX y 10 TX: no es una prueba de throughput
ni trafico HTTPS satisfactorio. La pagina local funciona precisamente sin red.

La regresion QEMU anterior reprodujo el defecto con la imagen anterior:
preparacion terminada en aproximadamente 1,28 s sin cable; conectar despues
no produjo una ruta. Usa una copia de disco y una NIC virtual privada; no toca A:.

## Cambios

- DHCP se inicia en segundo plano incluso sin carrier. Mantiene la supervision
  de cambios de enlace y la renovacion de concesiones despues de cerrar el
  preparador. No se limita a ampliar el segundo de espera.
- La espera inicial tiene un presupuesto de 12 s; la interfaz puede conectarse
  despues sin relanzar el escritorio. El escritorio tambien puede abrir sin red.
- El estado listo exige carrier, IPv4, ruta utilizable y una entrada nameserver.
  La presencia de DNS se distingue de comprobar acceso DNS/HTTPS real. Una ruta
  marcada `linkdown` ya no basta para mostrar una red preparada.
- El preparador cierra por si mismo el descriptor 9 antes de iniciar daemons.
- Se evita depender de `/dev/fd` al analizar rutas: la consola previa al
  escritorio no siempre tiene ese enlace. La prueba de reentrada lo detecto
  antes de instalar las imagenes finales.
- La tarjeta separa `Tuberia: 3290 ms` de `Cambio: 10 ms menos que el anterior`.

`scripts/probar_enlace_tardio.py PERFIL --antes|--despues` usa el cliente dhcpcd
real y el kernel ULTRA. La comprobacion posterior conecta el cable **despues**
del retorno del preparador, verifica IPv4/DNS, hace HTTPS como UID1000 sin
capacidades y prueba desconexion/reconexion sin volver a iniciar DHCP.
Los resultados de ambos perfiles y la regresion completa se conservan en
`evidencias/pc-real-w3h/`.

Las horas `fecha_utc` del equipo real aparecen anteriores a la fecha UTC de
compilacion del mismo kernel. No son fiables para reconstruir cronologia; los
intervalos anteriores usan uptime/reloj monotonico. Se necesita comprobar la
interpretacion del RTC y sincronizar la hora al disponer de red antes de sacar
conclusiones sobre fallos TLS por fechas. Esta ronda no ajusta el reloj a ciegas.

## Proxima comprobacion fisica

Ambos perfiles finales pasaron la regresion QEMU (100/100 generico y 104/104 PC)
y seis escenarios de enlace tardio por perfil. Se instalaron en A: y se
compararon los hashes de los dos kernels y las dos imagenes de sistema.

La verificacion posterior encontro la imagen semilla de datos: la carpeta
original estaba en la papelera. Se recupero datos.ext4 desde alli, respaldando
y verificando primero su SHA256 exacto contra el registro anterior al trabajo.
Tambien se restauraron los seis informes. La comprobacion final confirma que
los datos son los originales y las imagenes instaladas son las nuevas.
El instalador ahora respalda y verifica los datos y recuerda su presencia
fuera de A: para rechazar su ausencia en futuras actualizaciones. Las pruebas
del instalador se ejecutan sobre una unidad temporal de archivos, sin discos.

Arrancar PC, entrar con `e`, abrir Brave con Super+B y probar una pagina HTTPS.
Si se conecta el cable mas tarde, DHCP debe recuperar la red sin reiniciar.
Cerrar Hyprland y salir del menu con Q exporta el informe. Ese arranque confirmara
la correccion en la Realtek fisica; las pruebas QEMU verifican el comportamiento
con NIC virtual, no el rendimiento ni todos los estados del hardware real.

Fuentes: informes originales y fotos preservados en la evidencia; opciones de
[dhcpcd, background y gestion de enlace](https://github.com/NetworkConfiguration/dhcpcd/blob/master/src/dhcpcd.8.in).
