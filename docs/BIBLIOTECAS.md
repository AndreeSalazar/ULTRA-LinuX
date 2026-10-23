# Carpetas de juegos en varios discos

Super+J > **Elegir carpetas y discos** abre el gestor general. No esta limitado
a Warframe ni a una ruta fija de Windows. Admite hasta 32 carpetas elegidas,
identificadas por UUID del volumen y ruta relativa, no por letras C:/D:.

El [panel de discos](DISCOS-PANEL.md), tambien disponible desde Super+Espacio > Discos y bibliotecas > Panel,
muestra estado y espacio en tiempo real y una revision automatica de un minuto.
Pulsa 1 para abrir el selector de carpetas. Recuerda la ultima ruta visitada.

## Elegir e iniciar

1. Selecciona **Agregar carpeta de cualquier disco** y luego el disco.
2. Navega con flechas y Enter. En la carpeta deseada, elige **Elegir esta carpeta**.
   Escape cancela. Se seleccionan carpetas, no la raiz completa de un volumen.
3. Si contiene `steamapps`, se reconoce como biblioteca Steam. Una carpeta de
   juego independiente se conserva como tal, sin inventarle un AppID.
4. Cierra el escritorio y vuelve a abrirlo desde `e`. La preparacion privilegiada
   activa los nuevos montajes antes de lanzar las aplicaciones. No se cambia
   el montaje de un juego desde una aplicacion mientras esta ejecutandose.
5. Abre **Steam desde Super+J**. Con Steam cerrado, ULTRA registra automaticamente
   las bibliotecas Steam activas en su configuracion Linux. No necesitas copiar
   identificadores ni volver a elegir un disco en Almacenamiento. Si Steam estaba
   abierto, cierralo completamente y vuelve a abrirlo desde ese menu.
6. En Steam, selecciona Proton para las instalaciones Windows y verifica sus
   archivos antes de jugar. El registro no obliga al cliente a declarar un juego
   compatible ni completo: puede necesitar actualizaciones, runtimes o espacio.

**Registros de juegos** muestra las bibliotecas y juegos encontrados. El informe
GPU tambien incluye ese resumen, sin sesiones ni cuentas. ULTRA conserva los
campos existentes de `config/libraryfolders.vdf` y `steamapps/libraryfolders.vdf`,
incluido el indice de juegos instalados, y guarda un respaldo local anterior
cuando agrega bibliotecas; no modifica ese archivo si detecta Steam abierto o
un formato desconocido. Los manifiestos originales no se reescriben.
Las carpetas independientes siguen usando Heroic o UMU, sin registrarse como Steam.

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
