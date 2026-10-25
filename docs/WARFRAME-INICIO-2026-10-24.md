# Warframe: espera anterior al lanzador

El usuario precisó que la pausa del escritorio ocurre **antes de aparecer el
lanzador**, no durante la verificación visible de caché. Esto desplaza la
investigación a la preparación de bibliotecas, Steam/Proton y la creación de
la ventana. El registro de hashes del lanzador no demuestra la causa de esa
pausa anterior.

## Espera encontrada y corrección

`ultra-juegos` solicita registrar las bibliotecas antes de abrir Steam o su
URI de Warframe. `register_steam()` adquiría un `flock` exclusivo bloqueante
antes de comprobar si Steam estaba abierto. Una revisión concurrente podía
retener esa llamada sin plazo explícito.

Ahora se comprueba primero si Steam ya está abierto. En ese caso no se espera
al bloqueo ni se escriben sus bibliotecas. Con Steam cerrado, el intento de
adquirir el bloqueo es no bloqueante. Tras adquirirlo se vuelve a consultar
Steam para cubrir el arranque que pueda producirse entre ambas operaciones.

Si otra tarea mantiene el registro ocupado, el helper responde con código
75 (`EX_TEMPFAIL`). El acceso muestra el motivo y termina: **no inicia Steam
mientras esa otra tarea puede estar escribiendo sus VDF**. Se puede reintentar
cuando termine la revisión. No se salta el bloqueo ni se permiten escrituras
concurrentes para ocultar una demora.

Esto elimina una espera indefinida posible del acceso. No demuestra que fuese
el origen del tirón comunicado, ni elimina la preparación legítima de Proton,
las lecturas del juego o la verificación del lanzador.

## Captura desde el principio, sin escribir comandos

Elegir **Super+J → Warframe / diagnóstico** con Steam cerrado solicita la
captura de recursos antes de registrar bibliotecas y abrir Steam. Hasta ahora
ese modo preparaba MangoHud, pero la captura de recursos se iniciaba después
con Super+F11 y podía perder la pausa inicial.

La captura conserva los límites existentes: dos minutos, tres sesiones
guardadas como máximo y tamaño acotado. Super+Shift+F11 marca el momento de
la pausa; los registros están en Warframe / capturas guardadas. Si ya hay
tres capturas, el programa avisa en lugar de acumular más. No se activa un
observador adicional en el arranque normal de Warframe.

Se mide CPU global, presión, E/S, GPU y los procesos/hilos que se identifican
como parte del juego. Antes de que exista el proceso, sus medidas particulares
pueden ser nulas o no disponibles. La resolución temporal del observador no
equivale a una traza de cada cuadro del escritorio.

## Validación y entrega

Durante el empaquetado entró en `main` un merge con nuevas aplicaciones y tema
de escritorio. Se conserva ese trabajo en el repositorio. Esta entrega acotada
usa el escritorio ya instalado del commit `f14d12d` y sustituye únicamente los
helpers `ultra-bibliotecas.py` y `ultra-juegos`. No acredita integrar el nuevo
escritorio completo. La procedencia y huella de las nueve fuentes empaquetadas
quedan registradas, y el banco de arranque corresponde a esa base instalada.

Las pruebas de Linux usan un bloqueo `flock` real en un home desechable:
Steam abierto sale sin adquirirlo, una revisión ocupada responde sin esperar
y los VDF permanecen intactos. También se simula Steam apareciendo entre las
dos consultas. Siete pruebas del acceso verifican el orden de captura, el
rechazo de un escritor ocupado, la ausencia de observador extra en el modo
normal y la conservación de los recorridos de Cyberpunk.

No se modifican juegos, partidas, prefijos, cachés, frecuencia de CPU ni el
kernel. La comprobación física de la pausa sigue pendiente. La medición de
vatios continúa como fase posterior, según
[el plan de puntero y energía](PUNTERO-Y-ENERGIA-2026-10-24.md).

La revisión automática rechazó borrar dos imágenes anteriores del taller por
su posible uso como recuperación. Se conservaron ambas y se añadió un disco
virtual temporal nuevo de 16 GiB para preparar la imagen, sin conectar discos
físicos al taller.

La imagen pasó 152 comprobaciones de arranque/escritorio, con dos arranques y
persistencia, sin fallos. Se copió al perfil PC de A: y se comprobó por SHA-256
(`a01aadbe174e15e2…`); el kernel permanece idéntico y `datos.ext4` conserva
exactamente los bytes leídos inmediatamente antes de la instalación. El disco
virtual temporal ya se retiró; las imágenes anteriores del taller permanecen.

[Evidencias sanitizadas](../evidencias/warframe-inicio/2026-10-24).
