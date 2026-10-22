# Seguimiento de C, ASM y graficos

Fecha de revision: 2026-10-22, America/Lima.

- [Tareas](tareas-graficas.csv): 18 registros con prioridad, dependencia,
  criterio de cierre, evidencia, siguiente accion y limites. Prioridad 1 es
  trabajo previo o importante; prioridad 2 es ampliacion posterior. El estado
  describe el alcance probado, no un nivel universal de seguridad.
- [Mediciones](mediciones-graficas.csv): 12 filas de medianas por proceso,
  conservando el experimento base y esta ampliacion. Cada fila resume siete
  muestras por ruta con 2048 operaciones por muestra. Las observaciones crudas
  permanecen en el informe enlazado de cada experimento.

CSV UTF-8, separador coma y decimal punto. Los valores calculados se exportan
como numeros, sin formulas ejecutables. La columna `reduccion_tiempo_pct`
significa `100 * (1 - reutilizar_ms / recrear_ms)`: 43 significa 43 %, no 4300 %.
Un valor negativo indicaria regresion. No sumar ni promediar porcentajes de
distintas APIs o cargas para inventar una mejora global.

Los porcentajes comparan dos rutas dentro del mismo proceso y experimento;
comparar experimentos diferentes no prueba que un cambio de codigo haya
acelerado el sistema. El host, la VM y sus cargas introducen variacion. RSS es
pico acumulado del proceso, no memoria instantanea atribuible a cada ruta.
Los resultados no miden VRAM ni FPS ni rendimiento nativo de la RTX.

No hay porcentaje de proyecto completado: el alcance sigue abierto y las tareas
tienen costes y dependencias distintos. Mantener los estados especificos:
`PROBADO_LAB`, `PROBADO_SOFTWARE`, `MEDIDO_SOFTWARE`, `INTEGRADO_QEMU`,
`PENDIENTE_SOFTWARE`, `PENDIENTE_HARDWARE` y `PENDIENTE_INTEGRACION`.
Actualizar la evidencia y el criterio de cierre antes de cambiar un estado.

## Controles agregados en esta revision

G-13 crea y destruye 16 contextos/superficies EGL. Cada contexto nuevo crea una
textura, comprueba su nombre y un pixel verde del framebuffer y libera la textura
antes de cerrar el contexto. No simula perdida de contexto ni certifica ausencia
de fugas internas del driver.

V-05 repite 64 operaciones fill/copy entre dos buffers coherentes. Conserva la
barrera TRANSFER_WRITE a TRANSFER_READ antes de copiar, despues la dependencia
de escritura a lectura host, y espera una fence antes de comprobar las 1024
palabras del destino. El destino se pone a cero antes de cada envio para impedir
que datos viejos basten para pasar.

V-06 omite deliberadamente la primera barrera durante grabacion y exige
exactamente `SYNC-HAZARD-READ-AFTER-WRITE`. La capa aborta la llamada invalida;
el command buffer se descarta sin ningun envio. Esto prueba ese diagnostico
concreto, no todas las dependencias posibles de Vulkan.

[Informe final](../../evidencias/cxx/grafica-af760ff71a40457290d1d82166efd416/informe.txt):
ejecutor con salida 0; G-13, V-05 y V-06 completados en O2 y ASan/UBSan O1.
Los tres ROJO deliberados son el error tardio Vulkan y los dos rechazos del modo
de medicion instrumentado. No hubo diagnosticos ASan/UBSan/LSan inesperados.
El benchmark continua midiendo fill, no la nueva copia. Los CSV conservan esa
distincion y su alcance llvmpipe/QEMU.

El [primer intento](../../evidencias/cxx/grafica-fc6cbe2c969344f59552f32dc6fbb9aa/informe.txt)
fue rechazado por GCC al mezclar un enum y flags en una expresion condicional.
Se corrigio la construccion de los flags sin relajar advertencias ni reutilizar
resultados del intento fallido.

Siguiente trabajo independiente: inventariar tipos de memoria no coherente del
backend y ampliar tamanos/regiones de copia. La medicion RTX y la integracion en
un consumidor real permanecen pendientes. Las imagenes de A: no cambiaron.

Referencia: [dependencias de memoria Vulkan](https://docs.vulkan.org/guide/latest/synchronization_examples.html).
