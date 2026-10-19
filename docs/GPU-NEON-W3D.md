# W3d: nodos NVIDIA, informe GPU y tema neon

## Lo que mostraron las fotos

En el arranque fisico de W3c, el kernel carga nvidia, nvidia-modeset y
nvidia-drm 615.78.08. El preparador rechaza despues la sesion por nodos
ausentes, antes de lanzar Cage o Hyprland. No es evidencia de bajo FPS ni de
un fallo del renderer Hyprland. El aviso snd_hda_intel en 2b:00.4 corresponde
al audio integrado, separado de la GPU 29:00.0.

El mensaje anterior no identificaba cual nodo faltaba. La inspeccion del
codigo de NVIDIA revela una omision concreta: -c solicita cada minor por
separado; 0 crea nvidia0, 255 crea nvidiactl y -m agrega nvidia-modeset (254).
La orden anterior -c 0 -m no solicitaba el control. Ahora se usa:

```sh
nvidia-modprobe -c 0 -c 255 -m
```

La fuente oficial describe esa asignacion en
[cabecera de dispositivos](https://github.com/NVIDIA/nvidia-modprobe/blob/main/modprobe-utils/nvidia-modprobe-utils.h)
y [creacion de nodos](https://github.com/NVIDIA/nvidia-modprobe/blob/main/modprobe-utils/nvidia-modprobe-utils.c).
No se crean nodos manualmente ni se ejecuta nvidia-smi como preparacion.
El helper root valida cada dispositivo de caracteres y muestra su nombre
exacto cuando falta. El arranque fisico debe confirmar la correccion.

## Correccion de la prueba

El stub de W3c creaba nvidiactl aunque no se pidiera el minor 255. Aquella
prueba era insuficiente y oculto el error; sus logs historicos se conservan.
El nuevo modelo crea solo los minors solicitados y reproduce la rama UVM.
Se prueban exito, nodo omitido, fallo del helper y ambas ordenes anteriores:
-c 0 -m y -c0 -u. Ambas deben rechazarse por ausencia del control.
Esto verifica el flujo de preparacion con dispositivos simulados; no se
presenta como prueba del driver real ni de permisos/major/minor en la RTX.

## Informe que vuelve a Windows

El preparador genera /arch/run/ultra-escritorio/informe-gpu.txt al salir,
incluso si no llega a crear un cliente. Registra fecha UTC, kernel, modo,
compositor, codigo de salida, nodos y major/minor, version del driver,
informacion de GPU en procfs, mensajes del kernel y un extracto del cliente.
Limpia logs del cliente y broker al comenzar para no atribuir una sesion vieja
al intento nuevo. No ejecuta nvidia-smi al generar el informe.

La envoltura privilegiada exterior copia el resultado a BMO cuando termina
el comando. Al usar `e`, salir del menu con Q para completar esa copia:

```
A:\ULTRA\informe\gpu-<kernel>.txt
A:\ULTRA\informe\gpu-fallo-<kernel>.txt
```

El segundo conserva el ultimo intento fallido de la sesion, aunque despues
se use la recuperacion. Si no hay fallos nuevos, el archivo de fallo ya
existente puede ser de otro arranque: comprobar su fecha UTC. No se monta
BMO dentro del escritorio ni se concede
un acceso nuevo a las aplicaciones. Los informes se copian como archivos;
datos.ext4 se conserva. Los fallos siguen devolviendo un codigo distinto de
cero aunque se haya podido guardar el informe.

## Estilo aplicado

Negro #050a12, azul oscuro #09111d, cyan #00e5ff, verde #48ffb4 y rojo
metalico #dc546b. Bordes activos cyan/verde; barra con degradado oscuro y
salida rojo metal; terminal y lanzador con la misma paleta. Las apps GTK3
usan Adwaita-dark con colores propios. El home inicial de Hyprland es ahora
el directorio de ultra, en lugar de /.

El fondo SVG es una composicion propia con rejilla tenue, trazas cyan,
detalle rojo metalico y marca ULTRA. Swaybg muestra una imagen estatica;
no se agregan animaciones, blur ni sombras al compositor. Los cambios son
de apariencia, sin una afirmacion de mejora medida de rendimiento.

## Prueba siguiente en el PC

Desde arranque limpio: F11, perfil PC, `e`, 2, Enter. No usar antes la opcion
de nvidia-smi, para comprobar que la preparacion funciona por si sola.
Si falla, volver al menu y salir con Q; apagar de forma normal. Leer despues
los dos informes GPU en Windows. Si abre, comprobar apps, teclado, raton y
salida Win+Shift+M. El home sigue siendo temporal y pierde documentos al apagar.

Ver [evidencia de W3d](../evidencias/neon-gpu-w3d/README.md).
