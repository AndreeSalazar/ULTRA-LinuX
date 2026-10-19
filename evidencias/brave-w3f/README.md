# W3f: Brave, red y sandbox

Fuente: Brave 1.97.59 ZIP amd64 oficial, SHA256 fijado y validado antes de
extraer. Metadatos y bibliotecas en los ficheros brave-origen/bibliotecas;
lista de paquetes y manifiesto SHA256 por perfil.

Los registros legibles usan LF y eliminan espacios finales. Sus bytes
originales, incluidos los intentos fallidos, estan en registros-originales.zip.
Las capturas PNG se comprobaron completas antes de guardar la entrega.

`raiz-preflight.txt`: control sin root rechazado, chroot reproduce EPERM,
raiz privada permite userns con mapa 0 -> UID inicial 1000, y el namespace
de montajes del controlador no cambia.

`brave-preflight.txt` y `inicio-previo.png`: Brave real en el taller, prueba
HTML/WebGL por software con ANGLE/SwiftShader y error GL 0. No es la RTX ni
una ventana Wayland del PC fisico.

`generico-intento-1*`: primera prueba de navegacion automatizada fallida,
conservada. La captura original ya completa se rescato del archivo temporal;
la copia prematura del runner estaba truncada. Se corrigio la espera del
PNG y la secuencia de teclado, sin modificar el navegador para la repeticion.

`generico-resultados.txt`: 95 verdes, cero fallos y salida 0.
`generico-prueba-bmo.log`: ambos arranques QEMU, credenciales, namespaces y
navegacion HTTPS dentro de Brave. No se pasaron banderas para desactivar el
sandbox. `generico-brave.png`: ventana Wayland real de esa prueba.
`qemu-generico/`: informe de memoria extraido y resumen. Kernel #37,
319 paquetes y EROFS 607 MiB; imagen generica copiada a A: con datos conservados.

QEMU dispone de 2 GiB, una CPU y NIC virtio con NAT de usuario, sin puertos
entrantes. Carece de NVIDIA; la prueba explicita de NVIDIA debe rechazarse.
Para medir aceleracion de Brave en RTX hace falta el arranque fisico PC y
su pagina de diagnostico Super+G.

Uso y limites: [BRAVE-W3F.md](../../docs/BRAVE-W3F.md).

`pc-resultados.txt`: 99 verdes, cero fallos y salida 0. El resto de ficheros
pc-* conserva manifiesto, bibliotecas, paquetes, captura y ambos arranques.
Kernel PC #25, 325 paquetes, EROFS 1009 MiB. `qemu-pc/` contiene siete muestras
durante la prueba con Brave. Son datos software, no mediciones de la RTX.
`instalacion-verificada.json`: las cuatro EFI/EROFS coinciden con las imagenes
probadas y datos.ext4 conserva su hash. Hay copia local de las imagenes previas.
