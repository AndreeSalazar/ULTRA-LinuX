# Microtiron al entrar en escenarios de Warframe

El usuario precisa que ve un pico en la grafica de tiempos de cuadro y despues
el juego continua normal, sin deformaciones. No se ha reproducido el pico en
el PC desde esta investigacion y no se dispone de su duracion medida.

## Comprobado

- Los informes GPU de A: siguen siendo los de la sesion de 13:07 del 23 de
  octubre. Sus cinco muestras son posteriores al Xid 109 y no capturan el
  microtiron. No se atribuye ese Xid a Warframe.
- Se inspecciono el ext4 personal en QEMU sin red, como dispositivo de solo
  lectura y montaje `ro,noload`. Se conservaron tamano y fecha de la imagen.
- No hay CSV en `~/.cache/ultra/rendimiento/`, la carpeta prevista por ULTRA.
  Esto no afirma que no haya capturas guardadas manualmente en otra ruta.
- El perfil MangoHud tiene `log_interval=1000`, `log_duration=600` y subidas
  desactivadas. Una muestra por segundo puede omitir un cuadro lento aislado;
  no sirve para afirmar el p99 real de todos los cuadros.
- Se encontro EE.log de 873.644 bytes en el prefijo Steam de la biblioteca
  activa. Se analizaron solo categorias fijas; no se exportaron sus lineas,
  identificadores, direcciones, chats ni datos de cuenta. Contiene menciones
  de D3D11 y cache de shaders, sin menciones D3D12, errores DXGI de dispositivo
  perdido ni mensajes de disco lleno/E/S en los patrones buscados.
  Una mencion no acredita la API activa por si sola; no encontrar esos
  patrones tampoco demuestra ausencia de todos los errores posibles.
- El ext4 mantiene **3.343.835.136 bytes (3,11 GiB)** disponibles. La capa
  anterior de unos 30 GiB sigue presente; no se borro en esta revision.

El resultado sanitizado se guarda en
`evidencias/warframe-microtiron/2026-10-23/inspeccion.json`.
Se puede repetir la consulta con
`python scripts/inspeccionar_almacen_personal.py --warframe` desde Windows.
No inicia juegos ni cambia parametros de produccion.

## Hipotesis, aun no causas demostradas

1. **Compilacion grafica al presentar contenido nuevo.** DXVK traduce D3D11
   a Vulkan y permite mostrar actividad del compilador con `DXVK_HUD=compiler`.
   Su documentacion explica que cargar shaders puede provocar pausas incluso
   cuando se usan bibliotecas de pipelines. El log del motor no es un registro
   completo del compilador DXVK, por lo que su silencio no descarta esto.
   [Fuente DXVK](https://github.com/doitsujin/dxvk/blob/master/README.md#graphics-pipeline-library).
2. **Lectura/descompresion de recursos y trabajo CPU durante la entrada.**
   Faltan contadores de disco y tiempos de cuadro sincronizados en ese punto.
   El poco espacio libre es un problema confirmado, pero no prueba por si
   solo la causa del pico. Overlay tampoco demuestra por si solo un tiron.
3. **Espera de red/servidor.** Los errores NIC en cero no descartan espera
   externa. Faltan RTT y trafico durante el mismo pico. No se demuestra una
   renegociacion de Ethernet, DHCP, DNS o HTTPS durante ese cuadro.

No se borraron caches ni prefijos: la reconstruccion de caches tambien puede
alterar el comportamiento de la primera visita. No se cambiaron controladores,
banderas de optimizacion, intervalos de red o politica de energia sin evidencia.

## Prueba corta preparada, sin reconstruir ULTRA

El nuevo [modo integrado de diagnostico](DIAGNOSTICO-WARFRAME.md) prepara esta
captura desde Super+J y añade recursos, hilos y eventos del kernel. La opcion
manual siguiente sigue disponible para imagenes anteriores.

En Steam, Warframe > Propiedades > Opciones de lanzamiento, si el campo esta
vacio, se puede usar temporalmente:

```text
DXVK_HUD=compiler MANGOHUD_CONFIG=read_cfg,log_interval=0,log_duration=120,permit_upload=0 %command%
```

Si ya hay opciones, conservarlas: anteponer los dos ajustes al comando
existente, sin duplicar `%command%`. `compiler` corresponde a DXVK/D3D11;
no es un indicador de compilacion vkd3d en D3D12.

1. Iniciar el juego desde Steam con el HUD activado en ULTRA. Pulsar
   **Shift izquierdo + F2** justo antes de entrar al escenario.
2. Repetir dos veces el mismo escenario/ruta sin borrar caches, manteniendo
   resolucion, opciones graficas y version de Proton. Anotar si el indicador
   del compilador coincide con el pico y si este cambia en la segunda visita.
   Una segunda visita mas fluida tambien puede deberse a cache de archivos;
   por si sola no demuestra compilacion de shaders.
3. Pulsar la misma combinacion para detener la captura; se detiene sola a los
   120 segundos. Abrir **Super+J > FPS y HUD > Abrir registros de FPS**.
   Retirar las dos opciones temporales despues de la comparacion.

`read_cfg` mantiene el perfil y su carpeta de salida; `log_interval=0` usa el
camino de registro por cuadro de MangoHud 0.8.4. Se ha contrastado con su
[configuracion documentada](https://github.com/flightlessmango/MangoHud/blob/v0.8.4/README.md#hud-configuration)
y [codigo de registro](https://github.com/flightlessmango/MangoHud/blob/v0.8.4/src/logging.cpp).
Esta prueba aun requiere ejecutarse en el PC. El registro introduce trabajo
adicional: comparar tambien una pasada sin captura si aparecen picos nuevos.
Los sensores CPU/GPU pueden actualizarse mas lentamente que los cuadros;
una fila por cuadro no los convierte en sensores de igual resolucion temporal.

No se afirma haber mejorado FPS ni eliminado el microtiron con esta revision.
La evidencia obtenida reduce hipotesis y deja una captura repetible para poder
decidir si corregir compilacion, almacenamiento o red sin mezclar causas.
