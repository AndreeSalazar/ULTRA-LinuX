# Sesión y lanzadores

## Sesión

El panel superior y `Super + Espacio` ofrecen **Sesión / encendido**:

- Apagar;
- Reiniciar;
- Suspender, solo si el kernel anuncia `mem` en `/sys/power/state`;
- volver a la consola sin apagar.

La elección del usuario no ejecuta una orden libre ni usa `sudo`. El usuario 1000 escribe una petición privada, con identificador aleatorio y caducidad de 30 segundos. Un supervisor ya iniciado por el escritorio acepta únicamente una de las tres acciones fijas. Para apagar o reiniciar, primero cierra Hyprland y deja que `init` desmonte los volúmenes; para suspender sincroniza y entrega `mem` al kernel.

La suspensión está condicionada al hardware real. El contrato y el rechazo de acciones inválidas se prueban en el banco; la primera reanudación con la RTX y los periféricos conectados debe verificarse desde el equipo físico.

## Juegos

`Super + J` mantiene dos rutas separadas:

- **Heronite** integra el catálogo y conserva un estado seguro del último inicio en `~/.cache/ultra/heronite-inicio.json`.
- **Steam / cliente directo** abre Steam sin pasar por la detección de Heronite. Es la ruta de recuperación cuando el catálogo no reconoce todavía una biblioteca.
- **Estado del lanzador** muestra fase, resultado y avisos del último intento, sin copiar sesiones, tokens o rutas privadas a los informes.

El informe de GPU incorpora una sección `HERONITE` con ese estado cuando existe. Así, si una actualización o el arranque del lanzador falla, queda una causa concreta para corregir en vez de asumir que el juego o el almacenamiento fallaron.
