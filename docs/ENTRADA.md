# Entrada: mouse y teclado ULTRA

Abrir con **Super+Ctrl+M**, desde SUPER > Mouse y teclado o buscando
Mouse y teclado ULTRA en el lanzador de aplicaciones.

Ventana nativa Rust/egui/wgpu, con las dependencias de Carpetas ya disponibles.
No inicia un navegador ni un servicio de sondeo. El trabajo de Hyprland corre
fuera del hilo de dibujo; la ventana solo solicita cuadros al interactuar o
mientras espera una consulta.

## Controles

- Mouse: sensibilidad -1 a 1, aceleracion del sistema/plana/adaptativa,
  boton principal derecho e inversion y multiplicador de la rueda (0,1 a 5).
- Presets Normal y Rapido: cambian solo sensibilidad y aceleracion en el formulario.
- Teclado: US, espanol o latinoamericano; combinaciones US+espanol/latinoamericano
  con Alt+Shift; repeticion entre 1 y 80 por segundo y retardo entre 100 y 1500 ms.
- Zona para probar el puntero y campo de texto que nunca se guarda.
- Aplicar y guardar, recuperar perfil, valores iniciales y consultar estado real.
  Los cambios del formulario no se aplican hasta guardar; valores iniciales
  tampoco modifica el escritorio por si solo.

El perfil se guarda como usuario en `~/.config/ultra/entrada.json`, version 1,
con permisos 0600 y sustitucion atomica. En el escritorio persistente corresponde
a `/juegos/usuarios/ultra/.config/ultra/entrada.json`. Se recupera al iniciar
el escritorio y en el evento `config.reloaded` de Hyprland. Sin perfil, no se
modifican los ajustes existentes.

El helper `ultra-entrada` consulta, valida, aplica y relee las nueve opciones.
Si falla la comprobacion o la escritura, intenta recuperar el estado anterior;
un fallo de recuperacion se informa. Rangos, tipos, campos y cadenas estan
validados; no ejecuta texto del formulario como comandos de shell. No usa root,
acceso a `/dev/input`, macros, lectura global del teclado ni cambios de firmware.

Son ajustes globales del compositor. Una configuracion explicita por dispositivo
puede prevalecer. No equivalen exactamente a los pasos de velocidad de Windows;
la entrada cruda y sensibilidad propia de un juego, el DPI y la frecuencia del
dispositivo requieren sus controles correspondientes.

## Integracion y banco

`ultra-entrada-ui` es un segundo binario de la misma caja Cargo, sin dependencias
nuevas. `default-run` conserva el arranque habitual de `ultra-carpetas`.
`scripts/instalar-carpetas.sh` construye Carpetas, Entrada y Portapapeles y
comprueba las bibliotecas de las aplicaciones.

`scripts/probar_entrada.py` cubre validacion, consultas, persistencia, recuperacion
ante fallo de Hyprland/escritura y rechazo de perfiles redirigidos o excesivos.
`scripts/probar_entrada_instalada.py` esta limitado a QEMU desechable y comprueba
las opciones reales, el guardado y su recuperacion tras recargar el compositor.
`scripts/probar_arranque.py --entrada` prueba tambien el atajo y la ventana.

Referencias: [opciones de Hyprland](https://wiki.hypr.land/configuring/core/config-options/),
[distribuciones de teclado](https://wiki.hypr.land/configuring/core/binds/keyboard-layouts/)
y [eventos Lua](https://wiki.hypr.land/configuring/core/advanced-configuration/events/).

[Entrega PC, pruebas y captura](../evidencias/entrada/2026-10-31/README.md).
