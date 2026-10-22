# Auditoria y laboratorio de red

La NIC local anuncia 1000 Mbps, el otro extremo 100 Mbps. El cable va directo
al router segun el usuario. Linux registra full duplex y autonegociacion; Windows
tambien informa 100 Mbps. El modelo del router no se obtuvo de su pagina publica
anonima: no se inicio sesion ni se cambiaron sus opciones.

- `ethernet-linux.json`: hechos extraidos del informe fisico, configuracion
  del kernel y pico de 98,34 Mbit/s. No contiene direcciones IP o MAC.
- `windows-enlace.json`: nombre, controlador y velocidad del mismo adaptador.
- `colas-taller.json`: las cuatro pruebas aisladas FIFO/FQ-CoDel, con estadisticas
  de tc y conteo de bytes recibidos por el extremo TCP.
- `colas-resumen.txt`: resultados cortos de las cuatro ejecuciones.

El banco corre en Linux 7.2.11 del taller, no en ULTRA 7.3 ni en la NIC Realtek.
TBF modela 10 Mbit/s en salida. FIFO limita 1000 paquetes; FQ-CoDel usa sus
valores por defecto. TCP y sondas UDP comparten la cola. La mejora de p95
observada no implica una ganancia equivalente de descarga ni de ping en Internet.
El orden simetrico reduce un sesgo de orden, pero cuatro ejecuciones no forman
una evaluacion estadistica de hardware. No se expuso una interfaz fisica a los
namespaces y no se generaron conexiones hacia Internet.

`scripts/probar_auditoria_red.py` verifica anuncios multilinea, limites de seccion
y ausencia de datos sin inventar ceros. El resumidor previo sigue pasando sus
casos de tiempos, tasas, resets y recreacion de interfaces. El informe ampliado
paso bash -n y termino con RC 0 en el taller; su observador C no esta instalado
en ese entorno y aparece NA. No equivale a un arranque del informe nuevo en ULTRA.

El cambio de diagnostico y el fragmento de configuracion son fuentes preparadas
para una futura construccion. No se recompilo ni se instalo otro kernel en A:,
no se cambiaron MTU, offloads, politica TCP, autonegociacion, IRQ o RPS del PC.

Ver [investigacion y orden de aplicacion](../../docs/RED-RENDIMIENTO.md).
