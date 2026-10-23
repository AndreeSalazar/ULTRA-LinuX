# Centro BMO-X por categorias

2026-10-23, America/Lima.

**Super+Espacio** abre siete categorias. Flechas y Enter seleccionan; escribir
filtra la lista. Escape dentro de una categoria vuelve a la principal; Escape
en la principal cierra el centro. La opcion **Volver a categorias** hace lo mismo.
Al abrir de nuevo el centro queda seleccionada la ultima categoria utilizada
durante esa sesion, sin ejecutar su ultima accion automaticamente.

| Categoria | Acento | Opciones |
| --- | --- | --- |
| Aplicaciones | Cyan | Buscador, Brave, archivos, terminal, editor |
| Discos y bibliotecas | Turquesa | Panel de preparacion, selecciones guardadas, Windows y discos en lectura |
| Juegos | Verde | Steam, Heroic, Warframe, Cyberpunk, preparacion y espacio |
| Pantalla y graficos | Azul | Resolucion/Hz/escala, informe, cubo Vulkan y validacion |
| Audio | Ambar | Salidas, volumen, silencio, nivel normal y amplificacion opcional |
| Ambiente | Violeta | Efectos vivos/reducidos y notificaciones |
| Sistema y ayuda | Rojo | Atajos, RAM/GPU/espacio, audio y registros |

El menu principal mantiene un acento cyan. Cada submenu cambia borde, titulo y
coincidencias a su propio acento, con fondo negro azulado y texto claro. Fuzzel
no colorea individualmente todas las filas de la principal. Los nombres y
numeros permiten distinguir las categorias sin depender solo del color.
Los colores identifican areas del centro; no representan un resultado de pruebas.

**Discos > Panel de discos y preparacion** se abre a pantalla completa; Escape
lo cierra. Super+J usa tambien pantalla completa para ese panel. La revision de
un minuto y las selecciones persistentes mantienen el comportamiento documentado
en [Discos y bibliotecas](DISCOS-PANEL.md).

Las acciones son rutas fijas. Las etiquetas no se ejecutan como comandos; el
selector solo acepta sus entradas y los scripts siguen requiriendo UID1000.
El menu no agrega privilegios ni altera los montajes o flags de los juegos.

## Verificacion

Once controles con selectores simulados: siete rutas/acentos, Escape, Volver,
entrada libre rechazada y usuario root rechazado. El arranque QEMU abre los
siete submenus con teclado, comprueba el acento en la captura y vuelve a la
principal. Incluye tambien la apertura del panel y su revision de un minuto.

El generico pasa 150 comprobaciones y PC 151, sin fallos. Las capturas y los inventarios
se guardan en `evidencias/menu/6f8229215c8140a7bc4686d3574c1e2b/`.
Ejemplo: [categoria Juegos en QEMU](../evidencias/menu/6f8229215c8140a7bc4686d3574c1e2b/generico-categoria-3.png).

Ambos perfiles se reempaquetaron y copiaron a A:, conservando los kernels y el
hash de `datos.ext4`. El recibo `entrega.json` conserva los hashes de destino.
Los respaldos de la entrega anterior permanecen en `build/`, excluidos de Git.
La prueba del equipo fisico sigue correspondiendo al siguiente arranque del propietario.
