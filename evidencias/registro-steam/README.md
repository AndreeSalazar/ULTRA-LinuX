# Registro automatico de bibliotecas Steam

Cambio: al abrir Steam desde Super+J, el integrador registra las bibliotecas
Steam activas antes de iniciar el cliente. Se actualizan las copias de
`config/libraryfolders.vdf` y `steamapps/libraryfolders.vdf`. Si una biblioteca
ya estaba registrada con un indice vacio, se agregan los AppID que tienen un
manifiesto de instalacion completa y su directorio presente. No se inventan
manifiestos, no se cambian los de Windows ni se importan cuentas.

Las bibliotecas y campos anteriores se conservan. Las escrituras usan archivos
temporales y reemplazo atomico por archivo, con un respaldo local de cada
registro modificado. No se modifica el registro si se detecta Steam abierto.
Esto es integracion del cliente; no requiere cambiar el kernel.

`componentes.txt` comprueba conservacion de campos, respaldo, las dos copias,
idempotencia, reparacion de indice vacio, rechazo de formatos ambiguos y
proteccion con Steam abierto. Los resultados por perfil corresponden a
arranques UEFI en QEMU, con ayudantes nativos y discos ficticios.

Resultados finales: **33 comprobaciones en generico y 36 en PC**. En PC se abre
el cliente Steam real hasta su pantalla de acceso, se cierra normalmente y se
comprueba que conserva la biblioteca preparada en su registro. No se inicia
sesion ni se demuestra que una cuenta personal muestre el boton Jugar.

Una prueba inicial intento crear un juego nuevo bajo `steamapps/common` del
banco, que pertenece a root. Esa operacion fue rechazada. Las pruebas finales
registran una instalacion ficticia existente, sin alterar los permisos para
eludir esa restriccion. No acreditan instalacion de juegos nuevos en una
biblioteca cuyos directorios originales no permiten escritura al usuario.

La presencia de un manifiesto no demuestra integridad de todos los archivos,
compatibilidad de Proton, derecho de acceso a una cuenta ni espacio suficiente.
No se ejecutan juegos reales ni se expone el disco de Windows o el almacen
personal de D: a estas pruebas. La confirmacion del boton Jugar de la cuenta
personal queda para el arranque fisico.
