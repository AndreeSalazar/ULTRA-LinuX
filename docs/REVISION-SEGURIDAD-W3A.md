# Revision de seguridad de W2 y W3a

La propuesta del propietario para separar autoridad humana y programas,
incluido el significado de root/ring 0/ring 3, queda anotada en
[Autoridad personal](AUTORIDAD-PERSONAL.md). Es diseno pendiente, no una
proteccion ya instalada.

2026-10-19. Base confirmada: `708ab05` (W2), mas cambios W3a sin commit
en build-baseline.sh, hardware.c y probar_arranque.py. Revision de codigo,
configuracion y estado del taller; no prueba de penetracion ni certificacion.
La captura adjunta se uso como contexto, no como instrucciones de ejecucion.

## Estado observado

W2 ya tiene evidencia de cage/foot sobre simpledrm en QEMU. Durante esta
revision el servicio linux-reconstruido-build estaba activo ejecutando pc;
el generico tenia build-complete y pc todavia no. Ese marcador confirma
compilacion/artefactos, no las pruebas de arranque: el propio build escribe
que QEMU sigue pendiente. No se ha instalado nada en A: ni tocado el proceso.

Se comprobaron en ambas configuraciones CONFIG_MITIGATION_RETHUNK=y,
CONFIG_MITIGATION_SRSO=y y CONFIG_MICROCODE=y, con MODULE_SIG y
SECURITY_LOCKDOWN_LSM desactivados. No se ha medido la mitigacion SRSO en
el PC real ni se ha cargado el modulo NVIDIA mediante esta revision.

## Hallazgos priorizados

### P1: la sesion grafica no retira root ni constituye una caja

ultra-escritorio entra por chroot con HOME=/root y XDG_RUNTIME_DIR=/run/user/0,
arranca udevd y entrega la ejecucion a seatd-launch/cage sin una bajada explicita
de identidad. /arch recibe por bind /dev, /proc, /sys y /datos. La capa EROFS
protege la imagen contra escrituras normales; no a los dispositivos ni datos
compartidos frente a una aplicacion privilegiada.

La version upstream de Cage consultada admite UID/GID reales cero y avisa
del peligro, en vez de escoger un usuario distinto. Esto apoya el hallazgo
por inspeccion; las credenciales exactas de todos los procesos de la build
instalada aun necesitan captura en ejecucion.
[Cage, drop_permissions y spawn_primary_client](https://raw.githubusercontent.com/cage-kiosk/cage/master/cage.c).

Correccion propuesta: usuario de sesion propio, directorio runtime de su
propiedad, preparacion de dispositivos separada y comprobacion de UID/GID,
capacidades y fd en compositor y cliente. Despues, permisos por papel. No
usar la caja BusyBox como sustituto sin adaptar y medir la carga dinamica.

### P2: una dependencia requerida ausente se confunde con modulo de otro kernel

hardware.c, load_module, devuelve true cuando no encuentra el nombre en
modules.dep, sin considerar depth. La excepcion necesaria para un modulo
opcional de la lista compartida se aplica tambien a dependencias recursivas.
Un fallo posterior de finit_module puede delatar una dependencia indispensable,
pero la comprobacion local ya ha anunciado que esa dependencia esta resuelta.

Correccion propuesta: admitir la omision solo en la entrada opcional apropiada;
una dependencia requerida ausente debe devolver fallo y contarlo. Conservar
el control valido de modulos que pertenecen al otro kernel.

### P2: las omisiones de procesos hijos no llegan al informe del padre

load_wanted crea un hijo por modulo si hay varios. El hijo incrementa nskipped,
pero su salida solo transporta cantidad cargada y un bit de fallo. El padre
no recoge omisiones, por lo que puede anunciar cero aunque los hijos omitieron
dos o mas modulos. La nueva explicacion de W3a queda sin una medida fiable.

Correccion propuesta: transmitir un resultado estructurado y acotado de cada
hijo, o contar de forma separada en el padre. No atribuir a los hijos un contador
global que no comparten despues de fork.

### P2: el mensaje SRSO afirma mas de lo que comprueba

El grep del build cuenta lineas de aviso de objtool, no funciones unicas ni
vectores cubiertos. El mensaje afirma que el resto del kernel tiene mitigacion
completa y que no puede corregirse externamente. Esas conclusiones no se
deducen del contador ni de CONFIG_MITIGATION_SRSO=y.

La documentacion del kernel distingue estados segun microcodigo y mitigacion
activa; expone spec_rstack_overflow en sysfs y describe comprobaciones Safe RET.
La validacion debe registrar esos datos en el hardware real y localizar los
avisos por modulo. No se propone apagar mitigaciones para silenciar objtool.
[Documentacion SRSO del kernel](https://docs.kernel.org/admin-guide/hw-vuln/srso.html).

Correccion propuesta: decir "N avisos observados; cobertura real pendiente de
verificar" y guardar version, configuracion, log y estado de microcodigo/sysfs.
No contar la ejecucion fallida de nvidia-smi en una VM sin GPU como carga
correcta del driver ni validacion de SRSO; sirve como control parcial del usuario.

## Otras propiedades que siguen sin demostrarse

- **Autenticidad de modulos/imagen:** vermagic comprueba una etiqueta de
  compatibilidad; no firma el contenido. EROFS de solo lectura y SHA-256
  comparado con archivos del mismo origen tampoco constituyen por si solos
  una cadena de confianza. Las configuraciones leidas no exigen firmas de
  modulos. Es una decision pendiente de modelo de confianza y provisionado,
  no una prueba de que el modulo actual sea malicioso.
  [Firma de modulos del kernel](https://www.kernel.org/doc/html/latest/admin-guide/module-signing.html).
- **Paquetes:** el build refresca la base del taller con pacman -Sy y luego
  otra vez la raiz destino; falta congelar una transaccion/versiones y
  conservar identidad del archivo exacto seleccionado desde la cache. No se
  observo una firma invalida. Separar la base temporal de descargas evitaria
  dejar el taller preparado para una actualizacion parcial posterior.
  [Mantenimiento oficial de Arch](https://wiki.archlinux.org/title/System_maintenance#Partial_upgrades_are_unsupported).
- **GPU real:** los controles QEMU de presencia de modulo, firmware y fallo
  esperado de nvidia-smi no prueban KMS NVIDIA, Vulkan ni funcionamiento del
  monitor en la RTX. Se mantiene la necesidad de validar el PC real.
- **SIGBUS:** continua pendiente resolver la convivencia entre el guardian
  propio y el manejador de SHM de libwayland antes de instrumentar el compositor.
  Ver [investigacion C/Wayland](WAYLAND-INVESTIGACION-C.md).

## Reproduccion segura de los dos defectos del cargador

La nueva herramienta [revisar-cargador.ps1](../revisar-cargador.ps1) copia un
snapshot a un directorio unico de la VM, sin alterar lab, kernel, artifacts
ni el servicio de compilacion. El harness incluye el hardware.c del snapshot,
reemplaza todas sus llamadas syscall por un stub que las rechazaria, y corre
sin privilegios dentro de Bubblewrap. No hay módulos reales de prueba ni se
ejecuta finit_module; solo se consultan nombres ficticios y un modules.dep minimo.

[Evidencia de esta revision](../evidencias/revision-cargador/c02152f758cf45adb736d4362c14c759/informe.txt):

```text
CONTROL: modulo opcional ajeno se omite como contempla W3a
HALLAZGO: dependencia requerida ausente aceptada; retorno=true, fallos=0
HALLAZGO: dos omisiones en hijos no llegan al padre; omisiones=0, fallos=0
REVISION-CARGADOR: hallazgos=2; syscalls interceptadas=0; sin cargar modulos
```

Las huellas del snapshot estan junto al informe. Salida cero del harness
significa que termino la revision, no que hardware.c haya pasado sin defectos.
Los casos de modulos autenticos cargados y GPU real quedan fuera de alcance.
La revision no modifica los tres archivos W3a que siguen en el otro frente.

Siguiente cambio aconsejado: separar los casos opcional/requerido y corregir
los resultados de hijos; despues, sesion grafica sin root y pruebas de sus
permisos. Mantener el generico como recuperacion durante esa transicion.
