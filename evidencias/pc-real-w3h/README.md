# Equipo real y correccion de enlace tardio W3h

Analisis completo: [PC-REAL-W3H.md](../../docs/PC-REAL-W3H.md).

La foto y `arranques.txt` muestran 3290 ms hasta la fase de hardware, 10 ms
menos que el PC anterior de 3300 ms. Es una diferencia de 0,30 %, no un arranque
total de 10 ms ni una medida de latencia de red, GPU o fotogramas.

`informes/` contiene las seis copias entregadas desde A:. Los dos archivos GPU
son aliases identicos de la misma sesion y no se contabilizan dos veces.
`medicion.json` y `red.json` resumen sus muestras. La sesion fisica tuvo 24
muestras de GPU/RAM y 23 completas de PSS/CPU: RAM estimada 476,48–1057,08 MiB,
VRAM 1–236 MiB y GPU 0–13 %. Los primeros minimos son anteriores a cargar todas
las aplicaciones. No son consumos exclusivos de Brave ni picos absolutos.

`fotos/` conserva los tres PNG originales de la camara, verificados sin editar.
La segunda muestra Brave y la pagina WebGL; la tercera esta movida y no permite
diagnosticar por si sola un fallo grafico. Los informes confirman RTX 3060,
Hyprland/DRM/GL 3.2, sincronizacion explicita y salida de sesion 0.

Ethernet usa el firmware Realtek correcto y tiene enlace 100 Mb/s full duplex.
El extremo conectado solo anuncia 10/100; no se atribuye el limite a GCC o CPU.
El primer muestreo tenia carrier 0 y despues 1, pero faltaba IPv4/ruta. El
preparador antiguo esperaba un segundo y salia sin iniciar DHCP.

## Pruebas sobre las imagenes finales

| Imagen | Regresion de arranque/escritorio/Brave | Enlace tardio |
|---|---:|---:|
| Generico | 100 correctas, 0 fallidas | 6 escenarios correctos |
| PC | 104 correctas, 0 fallidas | 6 escenarios correctos |

`generico/` y `pc/` incluyen los resultados, transcripciones de consola, informe
exportado, guardianes y hashes. Las pruebas de enlace usan dhcpcd real en el
kernel ULTRA, una copia del disco y una NIC QEMU con NAT. Conectan el cable
despues de que el preparador haya retornado, exigen IPv4/DNS/HTTPS con certificados
validos, desconectan/reconectan y verifican reentrada. HTTPS se ejecuta como UID
1000 sin capacidades. No se inyectaron fuentes en esas imagenes finales.

`enlace-tardio-antes.*` reproduce el defecto con la imagen anterior: retorno en
1,28 s sin cable, sin obtener ruta cuando se conecta despues.

`prevalidacion/` preserva el ensayo inicial y su reparacion. La primera prueba
esperaba solo tres segundos para reconectar: DHCP puede retirar DNS durante la
renovacion; la prueba final espera recuperacion real dentro de un limite.
La reentrada tambien detecto la dependencia de `/dev/fd` en la consola previa
al escritorio; se sustituyo por una entrada directa que no depende de ese enlace.
La prevalidacion corregida uso una fuente en la capa RAM y declara su hash.
Se reconstruyeron ambos perfiles y se repitieron las pruebas sin esa inyeccion.

El taller QEMU se cerro durante el trabajo. Al recuperarlo, PC ya tenia su marca
de construccion completa; `pc-verificacion-recuperacion.txt` confirma todos sus
hashes antes de descargarlo. No se instaló una imagen parcial.

`instalacion-verificada.json` compara las cuatro imagenes instaladas en A: con
las probadas y verifica que datos.ext4 recupera su hash previo. Las imagenes
anteriores se respaldaron. No se formateo ni se escribieron nuevas entradas EFI.

La comprobacion posterior detecto una imagen de datos semilla. La carpeta
original ULTRA estaba en la papelera: se encontro alli datos.ext4 con el mismo
SHA256 registrado antes del trabajo. Se copio primero a un respaldo local,
se verifico y se restauro junto con los seis informes originales, conservando
las imagenes nuevas. `recuperacion-datos.json` registra la recuperacion exacta;
no se atribuye la eliminacion de la carpeta a una causa no demostrada.

El instalador ahora respalda datos existentes, verifica que no cambian durante
la copia y guarda un recibo local de su presencia. Una actualizacion posterior
se detiene si desaparecen. Tambien admite exigir un SHA256 concreto mediante
`-DatosEsperadosSHA256`; el recibo permite cambios legitimos hechos desde Linux.
`prueba-instalador-datos.txt` prueba el instalador real sobre una unidad temporal
de archivos: preservacion, respaldo, datos ausentes, hash distinto, opciones
incompatibles y reinicio intencionado con `-NuevosDatos`. No usa discos reales.

`originales-sha256.json` registra los bytes originales de los seis informes.
Estos y los registros de pruebas se preservan en `registros-originales.zip`.
Las copias de texto legibles normalizan finales de linea y espacios finales
para revisar en Git; los PNG permanecen intactos.

Queda confirmar el arreglo con la Realtek fisica en el siguiente arranque. Las
pruebas QEMU no acreditan throughput del cable ni todos los estados del silicio.
La hora UTC del informe fisico tampoco es una referencia cronologica fiable;
los intervalos se calcularon con uptime/reloj monotonico, como explica el analisis.
