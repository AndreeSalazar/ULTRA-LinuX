# Super+V: historial y pines

Super+V abre tarjetas nativas de texto e imagenes. Super+Shift+V sigue siendo
un alias. El atajo de ventana flotante pasa a Super+Ctrl+V.

- Buscar consulta las vistas previas; el historial se muestra en paginas de 30,
  sin cargar todos los contenidos en RAM. No hay un maximo fijo de entradas.
- Fijar mantiene una entrada delante de las no fijadas y la protege al limpiar
  o retirar copias antiguas por espacio. Desfijar permite eliminarla despues.
- Copiar recupera los bytes originales y cierra el panel. Ctrl+V los pega en
  la aplicacion elegida; el panel no escribe teclas ni ejecuta el texto copiado.
- Limpiar no fijados conserva los pines. Pausar detiene el registro de copias
  nuevas y se recuerda al abrir otra sesion.
- Presupuesto: 256 MiB iniciales, ajustable entre 32 y 4096 MiB. Al llenarse,
  se retiran las entradas no fijadas mas antiguas. Si solo quedan pines que
  impiden guardar otra copia, se conserva todo y se rechaza esa nueva entrada
  del historial; el portapapeles del sistema sigue disponible.

La base de datos SQLite se guarda en
`~/.local/share/ultra/portapapeles/historial.sqlite3`, con carpeta 0700 y archivo
0600. En ULTRA persistente vive en el home ext4, no en la RAM que se pierde
al apagar. Se deduplican copias identicas conservando su pin. Los identificadores
no se reutilizan, para que una lista abierta no seleccione otra copia por error.

El contador estima contenido y metadatos. SQLite tiene tambien un limite de
paginas para la base de datos. Se contabiliza sobrecarga y se recuperan paginas
libres; los diarios de una transaccion pueden ocupar espacio temporal adicional.
No es almacenamiento infinito. El presupuesto evita un historial sin control
sin imponer un numero arbitrario de entradas como el limite anterior de 200.

Por copia se admiten hasta 16 MiB; las imagenes tienen hasta 16 millones de
pixeles. PNG, JPEG, GIF, WebP y BMP se conservan con su formato original; se
generan miniaturas acotadas para el panel. Otros contenidos binarios no se
registran. No se importan los pines ni las copias de Windows.

Dos observadores `wl-paste --type text/image --watch` supervisados activan el
helper solo cuando cambia la seleccion; ambos terminan con la sesion.
Se omiten estados sensibles/vacios que informa wl-clipboard.
Esa indicacion depende de las aplicaciones; no detecta todas las contrasenas.
Pausar permite evitar el registro de otras copias privadas. El historial local
no esta cifrado y no se exporta automaticamente a informes ni a Git.

## Implementacion y validacion

Panel Rust/egui/wgpu con consultas fuera del hilo de dibujo. Reutiliza las
dependencias existentes de Carpetas. Helper Python/SQLite de UID1000: sin root,
red, shell para contenidos, macros ni ejecucion de enlaces.

`scripts/probar_portapapeles.py`: mas de 200 entradas, paginacion, pines tras
limpiar y reabrir, deduplicacion, cuota y recuperacion ante falta de espacio,
imagenes exactas, pausa, copias sensibles, concurrencia, permisos e IDs estables.
`scripts/probar_portapapeles_instalado.py` y el banco `--portapapeles` verifican
copias Wayland reales, texto/PNG exactos y la ventana de Super+V exclusivamente
en QEMU desechable.

Referencia: [protocolo de estados de wl-clipboard](https://github.com/bugaevc/wl-clipboard/blob/master/data/wl-clipboard.1).
La seleccion explicita de texto e imagen sigue tambien el
[ejemplo de multiples tipos de cliphist](https://github.com/sentriz/cliphist#faq).

[Entrega PC, pruebas y captura](../evidencias/entrada/2026-10-31/README.md).
