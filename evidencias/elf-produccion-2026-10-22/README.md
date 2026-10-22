# Promocion de contratos ELF a produccion

Fecha local: 2026-10-22, America/Lima. Base de fuentes: 5bd51fb mas los cambios
de esta entrega. Los informes contienen hashes de binarios y del auditor; cada
perfil conserva su archivo fuentes.sha256 y el manifiesto de artefactos.

## Construcciones y comprobaciones

- Generico: kernel #49; PC: kernel #35, ambos 7.3.0-rc7 sobre 32c52ec87d62.
- Cinco ELF por perfil pasan la puerta obligatoria: init, caja, ultra-raiz,
  ultra-red-contadores y minas. Diez informes PASS, asociados al auditor local.
- Control valido y 14 mutaciones rechazadas por cada init nuevo; tampoco se
  permite reutilizar una salida. Ningun ELF mutado se ejecuta.
- Validador Windows: control valido y cinco rechazos por perfil, en fixtures
  dentro de build/, sin escrituras en A: durante esas pruebas.
- Arranque BMO/desktops con Brave: **119 verdes / 0 rojos en generico** y
  **120 verdes / 0 rojos en PC**, ambos con salida de proceso 0.
- Cambio automatico de perfil: **11 verdes / 0 rojos**, salida 0.
- Recuperacion con imagen PC rota en una copia virtual: **6 verdes / 0 rojos**,
  salida 0. No se altera la imagen original ni el kernel de A: para ese ensayo.

Los dos primeros intentos del genérico fallaron y se conservan como intento1 e
intento2. Una captura y los datos de foco identificaron el dialogo de creacion
de llavero de Brave como receptor de teclas. Se corrigio el harness para cerrarlo
en el perfil desechable y enfocar/verificar cada ventana con la API Lua actual.
El test final conserva libsecret, namespaces y seccomp, y no crea un llavero
sin contrasena. No se borraron las evidencias de los intentos fallidos.

Los registros arranque-*.txt son resultados resumidos; transcripcion-*.txt
conservan las respuestas de las maquinas virtuales. Los PNG son capturas QEMU;
ninguno es una foto del PC fisico. GPU software en ambos
ensayos: estas pruebas no demuestran rendimiento o estabilidad de la RTX 3060.

## Instalacion en A:

[Recibo de instalacion](instalacion-A.json): ambos kernels y ambos sistemas se
copiaron y compararon por SHA-256 con los artefactos nuevos. Se respaldaron
antes los cuatro archivos anteriores en el proyecto Windows, bajo build/
(ignorado por Git); sus hashes estan en respaldo-previo.json. Los scripts
conservaron ademas datos e informes segun su politica de respaldo habitual.

datos.ext4 mantiene antes y despues:
9d15db669eee41d495aa514697ea324ca7dc3bd1357a0cbf8f3ae94c5ffb9a74.
No se modificaron particiones ni entradas de firmware. Los discos personales
no se adjuntaron a los ensayos QEMU. Falta el nuevo arranque nativo por el usuario.

Las imagenes incluyen las mejoras C de cadenas en tarjeta y comprobacion de
errores al instalar el guardian que antes solo estaban probadas en fuentes.
El pool de handles sigue sin consumidor de produccion; el ASM sintetico no se
enlaza al PID 1. La puerta ELF es una politica de build/instalacion.

NVIDIA conserva el aviso SRSO registrado en nvidia-guardian.txt: 11674 lineas de
objtool, no funciones unicas. No se anuncia cobertura resuelta en hardware.
No hay medicion nueva de FPS, latencia o consumo en juego.

El taller y los QEMU de prueba quedaron apagados. SHA256SUMS cubre esta carpeta
salvo el propio manifiesto, con texto normalizado a LF. Los hashes originales
del taller y de los artefactos estan separados por perfil.

Ver [contratos, herramientas y limites](../../docs/ELF-PRODUCCION.md).
