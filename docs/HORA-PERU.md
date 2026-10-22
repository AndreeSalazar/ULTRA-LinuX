# Hora regional de Peru

ULTRA usa America/Lima de tzdata; la referencia IANA actual indica UTC-05:00
sin cambio estacional vigente. La barra ya tenia esa zona, pero el resto del
sistema carecia de una configuracion equivalente. No requiere un plugin.

El constructor incorpora /etc/localtime de Lima al initramfs y configura la
misma zona en el sistema Arch de cada perfil. Genera el locale es_PE.UTF-8 y
aplica LC_TIME a las aplicaciones y terminales del escritorio. Se conserva el
formato numerico existente para no cambiar los analizadores de datos.
La barra mantiene el reloj de 24 horas y usa los nombres regionales de fechas.
Un clic derecho en el reloj abre **ultra-hora**, disponible tambien en terminal.

Los informes de GPU, red y graficos mantienen su marca UTC para comparaciones
y agregan fecha local con offset -05:00. Los tiempos de rendimiento siguen
usando el reloj monotonic; la zona no altera esas duraciones.

## Sincronizacion por internet

Al abrir el escritorio, un proceso privilegiado independiente intenta ajustar
el reloj con chrony y **NTS obligatorio** mediante time.cloudflare.com. No usa
una pagina HTML como reloj y no degrada silenciosamente a NTP sin autenticar.
Se ejecuta en segundo plano: abrir el escritorio no espera la respuesta.

Es un ajuste puntual, no un daemon continuo: chronyd -q -t 25 termina despues
del primer ajuste o de fallar. Un timeout exterior limita a 30 segundos con
2 segundos adicionales antes de forzar la terminacion. No ofrece puertos NTP
ni control por red. El auxiliar necesita privilegios para ajustar el reloj,
pero no entrega esas capacidades al compositor ni a las aplicaciones.

Solo una salida correcta de chronyd publica la marca de ultima sincronizacion
en /run/ultra-hora/sincronizado. Si falla, el estado queda pendiente y el log
permanece en /run/ultra-hora/ultima.log. Un lock evita trabajos simultaneos y
una sincronizacion correcta reciente se reutiliza durante una hora. Un nuevo
intento se hace al volver a abrir el escritorio; no se promete mantener el
ajuste continuo durante una sesion de dias.

No se invoca hwclock para escribir el RTC ni se activa rtcsync. No se cambia
la configuracion horaria de Windows. Sin internet, DNS o un reloj inicial
suficiente para validar los certificados NTS, puede quedar pendiente; la zona
regional sigue funcionando con el tiempo que tenga el sistema.

## Verificacion

Se prueban las fechas contra un instante fijo, no comparando dos capturas de
relojes en marcha: epoch 0 debe mostrarse como 1969-12-31 19:00 -0500 tanto
en BusyBox como en Arch; LC_TIME debe mostrar diciembre.

El auxiliar tiene tres fixtures con chronyd simulado en namespaces sin red:
fallo no publica exito, exito se reutiliza y dos solicitudes concurrentes no
inician dos ajustes. El stub nunca modifica un reloj. La consulta NTS real
se verifica aparte con chronyd -Q, que calcula el desfase sin corregirlo.
No se confunde una simulacion con una sincronizacion real.

Las pruebas de los perfiles y el recibo de instalacion se conservan en
[las evidencias de hora regional](../evidencias/hora-peru-2026-10-22/README.md).

Fuentes: [IANA, reglas de America/Lima](https://data.iana.org/time-zones/tzdb/southamerica),
[chronyd](https://chrony-project.org/doc/4.7/chronyd.html) y
[configuracion de chrony](https://chrony-project.org/doc/4.7/chrony.conf.html).
