# Carpetas de juegos en varios discos

Super+J > **Elegir carpetas y discos** abre el gestor general. No esta limitado
a Warframe ni a una ruta fija de Windows. Admite hasta 32 carpetas elegidas,
identificadas por UUID del volumen y ruta relativa, no por letras C:/D:.

## Elegir e iniciar

1. Selecciona **Agregar carpeta de cualquier disco** y luego el disco.
2. Navega con flechas y Enter. En la carpeta deseada, elige **Elegir esta carpeta**.
   Escape cancela. Se seleccionan carpetas, no la raiz completa de un volumen.
3. Si contiene `steamapps`, se reconoce como biblioteca Steam. Una carpeta de
   juego independiente se conserva como tal, sin inventarle un AppID.
4. Cierra el escritorio y vuelve a abrirlo desde `e`. La preparacion privilegiada
   activa los nuevos montajes antes de lanzar las aplicaciones. No se cambia
   el montaje de un juego desde una aplicacion mientras esta ejecutandose.
5. **Mis carpetas y bibliotecas** muestra el estado y la ruta preparada.
   Agrega esa ruta en Steam > Ajustes > Almacenamiento, o utiliza la importacion
   de juegos de Heroic. En Steam, selecciona Proton para las instalaciones Windows
   y verifica sus archivos antes de jugar.

Para copiar la ruta sin escribirla: usa **Abrir carpeta preparada**, pulsa
Ctrl+L y Ctrl+C en el explorador, y pegala en el selector de Steam con Ctrl+V.

El gestor tambien permite abrir una carpeta preparada en el explorador y elegir
un EXE de una carpeta independiente para iniciarlo con UMU/GE-Proton. Cada carpeta
tiene su prefijo y registro separados. Una biblioteca Steam se inicia desde Steam;
el gestor no ejecuta sus EXE omitiendo el AppID. No garantiza compatibilidad de
todos los juegos, lanzadores, sistemas de DRM o antitrampas.

**Quitar seleccion** cambia la lista para el proximo inicio del escritorio. No
borra juegos, partidas ni capas persistentes. Los montajes ya activos se conservan
hasta cerrar/reiniciar el sistema; no se desmontan por la fuerza.

## Discos y escritura

Al preparar el escritorio se enumeran volumenes de datos NTFS/ext4/exFAT. Los
drivers correspondientes deben estar disponibles en el kernel. Se omiten
particiones EFI, reservadas y de recuperacion GPT, volumenes internos de ULTRA,
UUID ambiguos, sistemas desconocidos y discos sin formato. No se formatea,
reparticiona, repara, desbloquea ni fuerza un volumen hibernado.

Las vistas de seleccion estan en `/discos/<UUID>`, con solo lectura, nosuid,
nodev y noexec. C: reutiliza su montaje protegido de `/windows`. D: reutiliza
el NTFS privado mediante una vista bind de solo lectura: su montaje privado
necesita seguir escribiendo el archivo ext4 que contiene el home persistente.
Otros volumenes disponibles se protegen a nivel de bloque antes de montarse
en solo lectura; ext4 usa `noload` para evitar recuperacion del journal.
Un volumen ya en uso por otro montaje no se modifica. Para detectar un disco
conectado despues, vuelve a abrir el escritorio.

Cada carpeta seleccionada recibe una capa propia en
`/juegos/Bibliotecas/<identificador>`. Los originales se leen del disco y los
cambios van al ext4 persistente. No se copia la biblioteca completa al seleccionarla,
pero actualizar archivos puede copiarlos enteros. Las capas, prefijos, caches y
aplicaciones comparten los 16 GiB existentes: comprueba el espacio disponible.

Las bibliotecas Steam reciben directorios Linux independientes para compatdata
y shadercache. Las versiones `buildid` de los manifiestos de origen se comparan
entre montajes: si Windows actualizo un juego, se bloquea la reutilizacion de
esa capa hasta revisarla. No se borra ni se intenta fusionar automaticamente.
En carpetas independientes se comprueba identidad, no toda su version de contenido.
No es una comprobacion criptografica universal de los juegos.

Los accesos anteriores de Warframe y Cyberpunk permanecen disponibles. Evita
registrar la misma instalacion dos veces en Steam con capas diferentes.
Cada acceso anterior conserva su prefijo y partidas. Una carpeta nueva recibe
un prefijo independiente; no se migran automaticamente las partidas entre ambos.

## Limites de privilegios

El selector trabaja con UID1000. La configuracion se guarda en el home persistente
y root la recibe mediante un exportador ejecutado con ese mismo usuario; root
no abre la ruta de configuracion controlada por el escritorio. Las selecciones
se validan otra vez, sin `eval` ni comandos construidos con nombres de carpetas.
Python se ejecuta en modo aislado, sin modulos del home ni PYTHONPATH del usuario;
el exportador tampoco recibe la entrada de la consola privilegiada.

La preparacion abre cada componente sin seguir enlaces y fija un descriptor de
directorio. Los montajes utilizan alias privados de root, de modo que espacios,
comas o Unicode en el nombre no se convierten en opciones de montaje. Las capas
y destinos privilegiados tienen padres protegidos. PID1 desmonta primero las
capas, luego sus fuentes y vistas de discos, antes de liberar el almacenamiento.
Antes de montar se exige que UID1000 pueda leer y recorrer la ruta completa,
incluido `steamapps` cuando corresponde: root no convierte carpetas Unix
privadas de otro usuario en carpetas legibles mediante una capa propia.

El gestor no importa sesiones de Steam/GOG ni las publica. Steam y Heroic siguen
usando su home Linux. Los archivos originales son visibles en sus vistas de
lectura; esa visibilidad no equivale a una sesion autenticada compatible con Linux.
