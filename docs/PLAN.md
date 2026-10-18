# Plan inicial y criterios para avanzar

1. **Origen fijado.** Descargar el árbol oficial, registrar commit y licencias. Un espejo superficial no equivale al historial completo. El hash identifica contenido; no sustituye la verificación de una firma de publicación.
2. **Entorno Linux.** Disponer de GCC, binutils y dependencias de la versión elegida. Registrar versiones. No cambiar automáticamente características de Windows ni reiniciar.
3. **Referencia sin cambios.** Compilar fuera de fuentes con `O=`, guardar configuración, commit y registro. Arrancar en QEMU con un initramfs mínimo y demostrar consola. La imagen de usuario y su init son una tarea pendiente.
4. **Guardianes.** Separar controles de construcción, pruebas y mecanismos de ejecución. Definir para cada uno: recurso protegido, condición, acción, motivo, prueba positiva/negativa y coste. Una revisión textual no prueba seguridad de memoria.
5. **Experimentos pequeños.** Ramas propias, un objetivo por cambio y comparación contra la referencia. Medir tiempo de arranque, memoria y rendimiento con condiciones repetibles. Conservar una configuración recuperable.
6. **Sistema de usuario.** Elegir bibliotecas, init, consola, paquetes y actualizaciones. Inspiración Arch no implica reutilizar sus paquetes sin respetar sus dependencias y ABI.
7. **Gráficos y Chromium.** Integrar almacenamiento, USB, red, gráficos, entorno de pantalla, bibliotecas, fuentes y aislamiento. Mantener las funciones necesarias para el sandbox. POSIX por sí solo no garantiza compatibilidad con binarios Linux ni con Chromium.
8. **Prueba física.** Solo tras superar QEMU y definir recuperación. Conservar Windows y las entradas actuales. La GPU y su controlador se identificarán antes de afirmar aceleración gráfica.

## Primera meta cerrable

Kernel oficial sin cambios compilado con GCC, commit/configuración/herramientas registrados y consola propia visible en QEMU. No incluir Chromium ni una reescritura de POSIX en esta primera meta.

## Registro por experimento

Pregunta; commit base; cambio; configuración; herramienta; prueba; resultado; comparación; clasificación ROJO/AMARILLO/VERDE; motivo; limitaciones; acción siguiente. Ningún resultado verde debe extrapolarse a todo el sistema.