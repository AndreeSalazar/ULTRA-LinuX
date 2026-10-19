# Autoridad personal y programas sin privilegios implicitos

Idea del usuario, registrada el 2026-10-19: ULTRA debe reservar decisiones
sensibles al propietario, avisarle y no conceder a un programa toda su
autoridad por el hecho de ejecutarlo. Este documento es una propuesta de
arquitectura; no anuncia un nuevo modelo de permisos implementado.

## Root y los anillos son conceptos diferentes

Root es la identidad UID 0, con reglas especiales y privilegios que Linux
distribuye tambien mediante capacidades. En x86, un programa root sigue
ejecutandose normalmente en ring 3. El kernel y sus controladores ejecutan
codigo privilegiado en ring 0; una syscall entra al kernel para que este
compruebe la operacion. Root no significa que cada instruccion del programa
se ejecute en ring 0.

Cambiar el nombre o numero de la cuenta administradora no demuestra retirar
su autoridad. La propuesta requiere politicas que el propio proceso
restringido no pueda quitar ni sustituir, incluso si conserva UID 0 dentro
de su dominio. No debe existir una cuenta personal omnipotente heredada por
todos sus hijos.

Fuentes: [credenciales Linux](https://docs.kernel.org/security/credentials.html),
[capacidades](https://man7.org/linux/man-pages/man7/capabilities.7.html).

## Arquitectura propuesta

1. Sesion, compositor y aplicaciones ordinarias sin root y con datos/fd
   concedidos por finalidad. Un programa propio tambien puede tener un fallo.
2. Servicios pequenos y separados para operaciones administrativas concretas,
   como instalar una imagen firmada o preparar una sesion grafica.
3. El kernel comprueba identidades, capacidades y politica obligatoria; no
   ejecuta el dialogo grafico ni espera una respuesta humana en cualquier
   contexto del kernel.
4. Un agente de autorizacion aislado presenta una peticion concreta: solicitante,
   accion, objeto, alcance y duracion. La aplicacion solicitante no controla
   ese dialogo ni puede hacerse pasar por el propietario.
5. Una autorizacion habilita solo esa operacion o recurso; no entrega una
   shell root, acceso general al disco ni una excepcion permanente.

Los [LSM](https://docs.kernel.org/security/lsm-development.html) ofrecen puntos
de aplicacion de politicas. Landlock puede restringir procesos que cooperan
en su confinamiento; no equivale por si solo a una politica global contra
todo administrador. La eleccion e integracion de mecanismos sigue pendiente.
[Alcance de Landlock](https://docs.kernel.org/userspace-api/landlock.html).

## Avisar siempre requiere definir que se autoriza

Se propone aprobar acciones sensibles nuevas, no cada lectura, dibujo de
pantalla o syscall. Las operaciones corrientes previamente concedidas quedan
limitadas por su politica. Preguntar por todo bloquearia el uso y favoreceria
aceptaciones automaticas. Deben existir limites de solicitudes y denegacion
si no llega una respuesta, sin bloquear indefinidamente el kernel.

La aprobacion debe quedar ligada al recurso real y al proceso/contexto correcto,
no solamente a un nombre de archivo, PID reutilizable o ruta que pueda cambiar
entre comprobacion y uso. Considerar reemplazo de archivos, renombrados,
reinicio de procesos, reutilizacion y duplicacion de fd y revocacion.

Hace falta un canal de confirmacion fiable: una ventana dibujada por una
aplicacion cualquiera no demuestra consentimiento. Si el compositor esta
comprometido, puede falsificar ventanas o entradas. Diseñar como accede el
propietario al agente de confianza es parte del trabajo, no una propiedad
automatica de estar sentado frente al PC.

## Que significa confiar en el PC

La maquina no identifica por si sola intenciones humanas. Hay que definir
que autentica al propietario y que componentes se consideran fiables:
arranque, kernel, controladores, agente de autorizacion y camino de entrada.
Un driver de terceros cargado en el kernel participa de la autoridad de
ring 0. Firmar un modulo acredita su origen segun una clave confiada, no
demuestra ausencia de fallos. [Firma de modulos](https://www.kernel.org/doc/html/latest/admin-guide/module-signing.html).

Si un fallo permite controlar el kernel, la politica de ese mismo kernel
no garantiza protegerse del atacante. Las medidas de aislamiento reducen
riesgos dentro de un modelo de confianza; no crean un observador infalible.

## Aplicacion a ULTRA y pendientes conservados

El siguiente paso practico es retirar privilegios de la sesion grafica y
separar la preparacion de dispositivos de compositor y clientes. No hace
falta empezar borrando UID 0 o reescribiendo todas las comprobaciones Linux.
Desarrollar primero una operacion administrativa concreta, con autorizacion
limitada y pruebas de denegacion, duplicacion, expiracion y muerte del agente.

Se conservan los cuatro hallazgos de la
[revision W3a](REVISION-SEGURIDAD-W3A.md): sesion grafica privilegiada,
dependencia requerida ausente aceptada, omisiones perdidas entre procesos
y mensaje SRSO que afirma mas de lo comprobado. Los dos defectos del cargador
estan reproducidos, pero esta nota no los corrige ni interviene en W3a.

Las pruebas futuras deben incluir un proceso con UID 0 dentro de un dominio
confinado al que se deniegue una accion, ademas de controles del propietario
autorizado. Tener el mismo UID que el usuario no basta para representar su
consentimiento.
