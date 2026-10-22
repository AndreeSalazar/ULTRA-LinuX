# Contratos HEX/ELF en la construccion de produccion

Fecha: 2026-10-22. La idea de verificar el artefacto emitido, estudiada en
[TALK-CPU](HEX-TALK-CPU.md), se integra ahora en la construccion normal de ULTRA.
El inspector de siete bytes del banco ASM se conserva como ensayo especifico;
no se exige que el cuerpo completo de cada funcion GCC conserve bytes identicos.

## Cobertura de esta entrega

[build-baseline.sh](../scripts/build-baseline.sh) exige aprobar
[auditar-elf-produccion.py](../scripts/auditar-elf-produccion.py) para **init,
caja, ultra-raiz, ultra-red-contadores y minas**, antes de construir la imagen.
La misma puerta se aplica a los perfiles produccion, vigia, generico y pc; esta
entrega reconstruye y comprueba generico y pc. El laboratorio de minas conserva
sus casos deliberadamente incorrectos; aprobar su formato no aprueba esos casos.

El auditor analiza las cabeceras binarias y exige:

- ELF64 little-endian, maquina x86-64 y tipo DYN para el perfil static PIE.
- Cabeceras y segmentos dentro del archivo, alineaciones coherentes y entrada
  dentro de bytes de un segmento ejecutable.
- Ningun segmento LOAD con permisos simultaneos de escritura y ejecucion.
- Una pila GNU_STACK RW sin permiso X.
- Una region GNU_RELRO declarada y contenida en un segmento escribible.
- Sin interprete dinamico ni DT_NEEDED; tabla dinamica terminada, NOW y sin
  declaraciones de text relocations.
- Propiedades CET IBT/SHSTK publicadas y los bytes **F3 0F 1E FA (ENDBR64)** al
  inicio de main, localizados por simbolo con objdump.

Se conserva el desensamblado de main, cabeceras, segmentos, reglas, versiones
de readelf/objdump y hashes del ejecutable y del auditor. Se inspeccionan los
ejecutables sin arrancarlos. El coste del control ocurre durante el build y
la instalacion, no se agrega una inspeccion HEX a cada acceso de memoria.

Estos son contratos estructurales para binarios propios emitidos por nuestro
toolchain. No son una validacion completa de ELF hostil, una prueba de semantica,
ausencia de UB ni una prueba de que CET esta activo en la CPU. El contenido de
GNU_RELRO no demuestra por si solo la cobertura de cada objeto de relocacion.
Las pruebas de arranque consultan las protecciones reales por separado.

## De la compilacion a la instalacion

Cada build usa una carpeta nueva para sus informes, bloquea ante cualquier fallo
y empaqueta elf-produccion.tar.gz. El manifiesto SHA256SUMS registra ese archivo,
los cinco binarios y los artefactos de imagen. No se reutiliza un informe anterior
como sustituto de uno que fallo. El archivo fuentes.sha256 contiene las fuentes
enumeradas por el constructor; no es un manifiesto completo de dependencias.

construir-kernel.ps1 transfiere el auditor y descarga el archivo de informes
comprobando su hash. Antes de escribir A:, instalar-en-A.ps1 invoca
[validar-elf-produccion.ps1](../scripts/validar-elf-produccion.ps1): exige los cinco
informes PASS, esquema/nombres/reglas y hashes coincidentes con el manifiesto.
Una construccion antigua sin ese archivo se rechaza y necesita reconstruirse.

La cadena confia en el proyecto local, toolchain y manifiesto. Los SHA-256 dan
integridad y trazabilidad respecto a esa referencia; no son una firma de un
editor externo ni impiden modificar deliberadamente codigo, politica y manifiesto.

## Pruebas

[probar-elf-produccion.py](../scripts/probar-elf-produccion.py) usa un init real
como control y 14 mutaciones de copias: pila X, LOAD WX, RELRO ausente, interprete,
arquitectura, entrada, tabla fuera del archivo, segmento truncado, archivo corto,
NOW ausente, DT_NEEDED, text relocations, perdida de CET y primer byte de main
alterado. Todas deben rechazarse con la causa prevista sin publicar informe PASS.
Tambien rechaza reutilizar una carpeta de resultados. No ejecuta los ELF mutados.

[probar-validacion-elf.ps1](../scripts/probar-validacion-elf.ps1) prueba el control
valido y cinco rechazos de promocion: archivo alterado, hash de init distinto,
hash ambiguo, estado FAIL y regla obligatoria ausente. Usa fixtures en build/;
no escribe A: ni ejecuta los binarios Linux.

Los resultados de reconstruccion, arranque y copia se registran en
[las evidencias de esta entrega](../evidencias/elf-produccion-2026-10-22/README.md).
Una prueba de escritorio QEMU no valida la GPU NVIDIA fisica ni una partida.

El primer intento detecto un dialogo legitimo de creacion de llavero en el
perfil desechable de Brave que recibia las teclas de la prueba y cambiaba su
foco. Se conservaron los intentos fallidos y una captura. El harness ahora
cierra ese dialogo mediante el gestor de ventanas en la VM y comprueba el
foco real antes de enviar teclas, con la API Lua de Hyprland 0.56.2. No se
desactiva libsecret ni el sandbox, no se guarda una contrasena vacia y no se
prueba persistencia de credenciales. Referencia de la API usada:
[implementacion oficial del dispatcher](https://github.com/hyprwm/Hyprland/blob/v0.56.2/src/config/lua/bindings/LuaBindingsDispatchers.cpp).

## Lo que sigue

El pool de [handles con propietario](VIDA-UTIL-Y-ASM.md) sigue siendo candidato
sin consumidor de produccion. Esta entrega no reemplaza masivamente malloc/free
ni enlaza sondas ASM de fallo al PID 1. Migrar requiere seleccionar un consumidor,
preservar su contrato de E/S, medir coste y probar sus caminos de error.

Quedan identidad entre pools, concurrencia, desenrollado/CET del ASM propio,
reglas especificas para C++, y validacion de recursos OpenGL/Vulkan. Los binarios
de Arch, NVIDIA, Hyprland y Proton requieren politicas propias: no se les impone
el contrato static PIE de nuestras cinco herramientas.

Referencia de herramientas: [GNU readelf](https://sourceware.org/binutils/docs/binutils/readelf.html).
