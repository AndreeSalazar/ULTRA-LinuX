# Revision de cargas de juegos despues de la limpieza

El usuario informa que Cyberpunk 2077 supera la creacion del personaje y se
congela en la siguiente carga de partida. Warframe tuvo un pequeno tiron que
el usuario asocia con actividad de Internet. No se ha reproducido ninguno
de los dos incidentes en el PC desde esta revision.

## Evidencia disponible

Se reviso el informe nativo `gpu-7.3.0-rc7-pc-g32c52ec87d62-nvidia-hyprland.txt`,
fecha 2026-10-23 13:07:29 America/Lima. El otro archivo GPU es una copia del
mismo informe, no una segunda prueba. El resumen privado esta en
`build/analisis-cargas-20261023.json`. Los archivos originales permanecen en A:.

### Evento grafico confirmado

NVIDIA 615.78.08 registra a los 1378,639698 segundos de arranque:

```
Xid 109, pid=11056, name=GameThread, CTX SWITCH TIMEOUT
```

NVIDIA clasifica [Xid 109 como un tiempo de espera de cambio de contexto](https://docs.nvidia.com/deploy/xid-errors/analyzing-xid-catalog.html).
Es una pista prioritaria para un congelamiento grafico. El codigo por si solo
no distingue fallo de aplicacion, traduccion grafica, controlador o hardware:
[alcance de los Xid segun NVIDIA](https://docs.nvidia.com/deploy/xid-errors/introduction.html).
`GameThread` no identifica de forma unica un juego. No se acredita que este
evento sea del intento de carga descrito de Cyberpunk.

Las cinco muestras corresponden aproximadamente a los segundos 1987-2012
de arranque, unos diez minutos despues del evento. Registran 6089-6452 MiB
de RAM estimada y 4946-5081 MiB de VRAM, GPU al 0 %, 56-57 C y unos 20 W.
No son mediciones de la carga que fallo. No permiten afirmar que la GPU
funcionara correctamente durante el incidente ni estimar sus FPS.
No hay OOM registrado ni intercambio swap en esos contadores; los major faults
no aumentan durante esta ventana. Los page faults no equivalen a UB de C.

### Red: no se acredita la causa del tiron

El enlace r8169 es 100 Mbps full duplex; la tarjeta anuncia 1000baseT pero
el otro extremo anuncia solo 10/100. Esto no demuestra por si solo si la
limitacion esta en el puerto, configuracion o condiciones fisicas.
Los contadores NIC registran cero errores y descartes RX/TX. Eso no equivale
a ausencia de congestion, perdidas o esperas en el router o Internet.

TCP registra 705 segmentos retransmitidos y 64 timeouts acumulados desde
arranque. No son contadores exclusivos de Warframe ni una medida de perdida
del juego. Los contadores Ethernet apenas cambian en las muestras tardias;
no hay RTT del servidor, tiempos DNS/TLS ni tiempos de fotograma sincronizados
con el tiron. No se atribuye el sintoma a negociacion Ethernet o HTTPS.

### El espacio volvio a agotarse casi por completo

Una nueva inspeccion QEMU sin NIC, con el archivo ext4 de C: conectado como
dispositivo de solo lectura y montado `ro,noload`, confirma:

| Elemento | Bytes | GiB aproximados |
|---|---:|---:|
| Disponible para usuario | 3.343.835.136 | 3,11 |
| Capa Steam activa | 57.984.999.424 | 54,00 |
| Warframe en la capa activa | 53.160.632.320 | 49,51 |
| Nueva capa anterior | 32.196.136.960 | 29,98 |
| Cache general del usuario | 208.658.432 | 0,19 |

No sumar subtotales de la tabla. El informe nativo ya mostraba 97 % de uso.
El estado inspeccionado coincide con la reapertura de una nueva capa y el
crecimiento de los archivos privados del juego. Es espacio realmente ocupado,
no crecimiento inexplicable de la cache ni prueba de corrupcion ext4.
El informe privado completo esta en `build/inspeccion-despues-jugar-20261023.txt`.
Se conservaron tamano y fecha del archivo durante la inspeccion.

La nueva carpeta `.anterior-1792758815349198139` es distinta de la retirada
durante la limpieza anterior. No se borro en esta revision: puede contener
partidas y prefijos anteriores. La politica actual conserva la capa completa
cuando cambia la firma de versiones de la biblioteca original Windows.
Las actualizaciones Linux tambien escriben archivos completos en su upper.
Ambos mecanismos explican la reocupacion del espacio recuperado.

## Orden de correccion y verificacion

1. Separar una biblioteca Steam nativa en ext4 de las vistas de bibliotecas
   Windows. Migrar/verificar juegos y conservar partidas y prefijos antes
   de retirar capas. No basta con registrar la misma carpeta otra vez.
2. Revisar la politica de retencion de capas anteriores para que sea visible
   y acotada. No convertirla en un borrado automatico de partidas.
3. Reiniciar ULTRA y reproducir solo Cyberpunk una vez, registrando identidad
   de proceso, version real de Proton/GE y eventos GPU desde antes de la carga.
   Comparar un segundo intento sin HUD usando su opcion del menu; cambiar
   una sola variable por intento. No se ha ejecutado aun esta comparacion.
4. Para Warframe, correlacionar tiempos de fotograma con una carga repetible,
   trafico y RTT durante el mismo intervalo. Mantener separados congelamiento
   local, carga de archivos y espera de servidor. Los datos actuales no
   justifican forzar velocidad Ethernet ni modificar MTU, TLS o timeouts GPU.

QEMU sirvio aqui para inspeccionar almacenamiento. No puede reproducir este
fallo de la RTX 3060 fisica sin acceso a ella. Esta revision aporta evidencia
y prioridades; no acredita una correccion del controlador ni del juego.
