# Heronite y escritorio con menos actividad

Investigacion: 25 de octubre de 2026. No equivale a una instalacion en A:.

## Resultado

Heronite merece una prueba paralela; no sustituir Heroic ni sus configuraciones
todavia. El juego ya funciona y ULTRA integra rutas y estados de Heroic en sus
propios scripts. Cambiar solamente el ejecutable dejaria integracion incompleta.

El [repositorio oficial](https://github.com/RafaelGoulartB/Heronite) describe una
interfaz Rust/GPUI renderizada por GPU, soporte GOG, herramientas Wine/Proton y
una importacion inicial de Heroic hacia un directorio propio. Publica alrededor
de 190 MB en reposo y 95 MB en bandeja: son cifras del autor, no de ULTRA. Requiere
Vulkan y GTK/WebKitGTK para componentes de autenticacion; no significa que toda
su cadena de herramientas sea Rust o que no tenga dependencias web.

La [version v0.14.0](https://github.com/RafaelGoulartB/Heronite/releases/tag/v0.14.0),
marcada como ultima al consultar, incorpora importaciones y aislamiento. Algunas
importaciones pueden lanzar mediante el gestor original; eso no elimina su coste.
No se pudo recuperar Cargo.toml ni ARCHITECTURE.md con la herramienta web:
arquitectura y cifras anteriores proceden de su README y notas, no de auditoria.

## Mejora local de bajo riesgo

Nuevo perfil optativo `ahorro` en `ultra-personalizar`:

- Fondo liso y acento cyan; barra compacta y reloj sin segundos.
- Sin animaciones, giro continuo de bordes, desenfoque ni dock.
- Tarjetas cerradas mediante su controlador, que termina su observador.
- Sonidos basicos y modo juego automatico conservado.

Se accede desde Personalizar > Perfiles o con
`ultra-personalizar perfil aplicar ahorro`. Para volver al tema de fabrica:
`ultra-personalizar perfil aplicar neon-hielo`. Para conservar elecciones
personalizadas anteriores, guardarlas como perfil antes de aplicar otro.
No cambia el perfil predeterminado ni el limite de frecuencia CPU.

Los perfiles anteriores Minimal y Sobrio no combinaban todas estas medidas.
Se reutilizan las opciones existentes, sin introducir claves nuevas de Hyprland
ni forzar direct scanout, apagar sincronizacion o modificar el controlador.

## Comparacion que falta en el hardware real

Los 0,7 GiB comunicados por el usuario no son una medicion atribuible a Wayland.
Wayland es el protocolo; comparar compositor, barra, monitores y aplicaciones.
Registrar RAM con MemAvailable y PSS de cada arbol de procesos (RSS compartido no
se suma como memoria exclusiva), VRAM, CPU, temperatura y potencia de paquete.
No vaciar page cache ni borrar cache de sombreadores para bajar el numero.

Comparar mismas aplicaciones, escena, resolucion, limite FPS y perfil CPU:

1. Escritorio actual frente a Ahorro tras estabilizarse y sin descargas.
2. Heroic abierto, en bandeja y cerrado; separar interfaz de procesos del juego.
3. Heronite en las mismas condiciones, incluyendo VRAM por su interfaz GPU.
4. Repetir arranque/cierre de Cyberpunk, actualizar/verificar desde GOG y reiniciar
   ULTRA; comprobar rutas, prefijo, partidas, REDlauncher y persistencia.

Para Heronite usar version y hash fijados, datos separados y respaldo verificado
antes de abrir juegos con acceso al mismo prefijo. Primero revisar que importacion
no repita descargas ni cambie ajustes. No ejecutar ambos gestores actualizando el
mismo juego. No incorporar sesiones, tokens ni datos privados a Git.

No hay medicion local de Heronite, ahorro de RAM cuantificado ni aumento de FPS
demostrado. El perfil reduce fuentes de trabajo; el ahorro real debe medirse.

## Validacion local

`scripts/probar_personalizar.py` paso en el taller Linux QEMU el 25-10-2026:
generacion, aplicacion con comandos de sesion simulados, entradas invalidas,
persistencia y restauracion. Se agregaron comprobaciones de Ahorro: apagado de
animacion/desenfoque/borde, cierre de tarjetas, detencion del fondo vivo, barra
unica y reloj por minutos. No es una medicion de rendimiento ni una prueba visual
del compositor con la RTX. Sintaxis Python validada tambien en Windows.
