# Wayland y guardianes C: investigacion por categorias

2026-10-19. La integracion de la base Arch y la sesion grafica avanza por
otro frente. Este documento organiza el trabajo de guardianes y registra
una primera API aritmetica probada; no acredita Wayland funcionando ni
un compositor protegido en produccion.

## Cruce prioritario: quien controla SIGBUS

La documentacion de libwayland-server indica que el acceso protegido a SHM
instala un manejador de SIGBUS y advierte que no funciona correctamente si
el compositor instala otro. Esto permite tratar archivos compartidos cuyo
tamano cambia o no coincide con lo declarado por el cliente.
[API oficial, wl_shm_buffer_begin_access/end_access](https://wayland.freedesktop.org/docs/html/apc.html).

Nuestro `ultra_guard_install` tambien instala SIGBUS. Por inspeccion hay
un conflicto de integracion que debe probarse: no se ha reproducido todavia
con un compositor. **No agregar esa llamada indiscriminadamente a Wayland.**
La revision debe determinar quien posee el manejador y conservar el
comportamiento documentado de libwayland ante truncamiento y senales ajenas
a sus pools. El orden de instalacion no constituye una solucion demostrada.

## Mapa de categorias y estados

Los estados siguientes son del proyecto; los criterios son propuestas de
prueba. No se considera que cualquier error grafico sea UB del lenguaje C.

| ID | Categoria | Pregunta y prueba propuesta | Estado |
|---|---|---|---|
| W-01 | Aritmetica de buffers | Dimensiones, stride, offset, capacidad y desborde antes de mapear o copiar. | API y banco de limites probados; falta adopcion grafica. |
| W-02 | Archivo SHM y senales | Archivo mas pequeno o truncado despues de mapear; compositor conserva servicio. | Investigar conflicto SIGBUS antes de integrar. |
| W-03 | Vida de objetos | Referencias de pool/buffer, callbacks y destruccion sin referencias colgantes. | Pendiente de fixtures con libwayland. |
| W-04 | Estado de protocolo | No reutilizar almacenamiento mientras el compositor lo utiliza; observar liberacion. | Pendiente de cliente/compositor real. |
| W-05 | Mensajes y versiones | IDs, tamanos, opcode y version de cada interfaz; error de un cliente sin caida del servicio. | Pendiente de banco de protocolo aislado. |
| W-06 | Descriptores IPC | Propiedad, cierre, CLOEXEC y transferencia SCM_RIGHTS; sin fugas ni descriptores accidentales. | Pendiente de perfil grafico independiente. |
| W-07 | Concurrencia y eventos | Colas por hilo, callbacks, errores y desconexion sin bloqueo. | Pendiente; TSan no cubre toda sincronizacion externa. |
| W-08 | DRM/KMS y sesion | Separar acceso a pantalla y entrada del acceso de un cliente a render. | Coordinacion con integracion grafica. |
| W-09 | DMA-BUF y GPU | Planos, formatos, modificadores, propiedad y sincronizacion CPU/GPU. | Pendiente; no cubierto por la API de cuatro bytes. |
| W-10 | Cargador y bibliotecas | Build dinamico real, rutas de biblioteca, recursos y cobertura de codigo ajeno. | Pendiente de versiones instaladas y medicion. |
| W-11 | Recursos por cliente | Limites pequenos de memoria, descriptores, eventos y tiempo; desconexion de un cliente sin derribar otros. | Pendiente; no probar agotamiento del host. |
| W-12 | Observacion en produccion | Versiones, hashes, logs por categoria y fallos recuperables sin filtrar contenido de usuarios. | Diseno pendiente; el banco no es telemetria instalada. |

La especificacion separa el objeto buffer de su almacenamiento y describe
las condiciones de reutilizacion y referencias de pool. Las propiedades de
protocolo deben probarse aparte de las de memoria de C.
[Protocolo Wayland, wl_shm_pool y wl_buffer](https://wayland.freedesktop.org/docs/html/apa.html).

La API de cliente documenta el reparto de eventos entre colas/hilos;
el algoritmo concreto debe seguir la API de la version instalada.
[API de cliente](https://wayland.freedesktop.org/docs/html/apb.html).
Para DRM y memoria compartida con GPU, partir de
[DRM UAPI](https://docs.kernel.org/gpu/drm-uapi.html) y
[DMA-BUF](https://docs.kernel.org/driver-api/dma-buf.html).
Una comprobacion de tamanos no valida fences ni termina el trabajo de GPU.

## Primera regla disponible: limites antes de operar

[ultra_buffer.h](../src/guardian/produccion/ultra_buffer.h) es una API separada,
sin dependencia de Wayland, sin asignaciones y sin instalar manejadores.
No se incluye automaticamente en el codigo grafico de otro frente.

`ultra_buffer_layout` valida filas, bytes usados por fila y stride; comprueba
multiplicacion y suma con overflow; exige que offset + filas * stride quepa
en la capacidad. Incluye todo el padding de la ultima fila: es nuestro
contrato conservador, no una afirmacion de que todo formato de Wayland exija
la misma formula. Devuelve argumento invalido, desborde, capacidad insuficiente
o exito. En un rechazo no modifica la salida.

`ultra_buffer32_layout` acepta dimensiones de 32 bits con signo y las valida
antes de convertirlas a size_t. Se limita a pixeles empaquetados de cuatro
bytes. No interpreta un identificador de formato ni maneja planos separados.
Ambas funciones requieren comprobar su resultado; GCC avisa si se ignora.

La API solo prueba un contrato numerico. No sabe si la capacidad declarada
es real, si el fd o puntero es valido, si el archivo se trunca, si el acceso
esta alineado para un tipo, ni quien mantiene vivo el almacenamiento. Debe
usarse antes de reservar, mapear o copiar y combinarse con esos contratos.

El [banco](../src/guardian/laboratorio/probar-buffers.c) contrasta 16.807
combinaciones de bordes con un oraculo independiente de enteros de 128 bits
en GCC/x86-64: 357 aceptadas y 16.450 rechazadas por perfil. Agrega 11 controles
de pixeles, padding, dimensiones con signo, pool un byte menor y salida nula.
Los calculos extremos no reservan esos tamanos ni acceden a memoria grafica.
Se ejecuta en aislamiento con produccion O2 y diagnostico ASan+UBSan.
Una prueba de compilacion separada exige que ignorar el resultado sea rechazado
por el perfil estricto, con diagnostico warn_unused_result.

Evidencia: [buffers de ronda 9](../evidencias/laboratorio-c/ronda9-f7c499a80d244b3ba797c72bde3e5b5a/buffers-ronda9.txt).
La suite C anterior conserva sus 62 minas y 36 rechazos; estos controles nuevos
se cuentan aparte.

Esto no instrumenta automaticamente paquetes Arch precompilados. Para aplicar
flags a libwayland o al compositor hay que reconstruir las versiones elegidas,
revisar su build y probarlas; el perfil static-pie minimo no sustituye su build.

## Por que la caja BusyBox no es aun una caja Wayland

La politica actual esta disenada para binarios estaticos y deniega socket,
connect, sendmsg/recvmsg y las operaciones de eventos no concedidas. Cierra
descriptores desde 3 y rechaza sockets en stdio. Ademas deniega mmap/mprotect
con PROT_EXEC. Por lectura de esas reglas, reutilizarla directamente para
clientes graficos dinamicos no es compatible con sus necesidades habituales.
No se ha probado aqui una aplicacion Wayland dentro de caja.

Se necesitan declaraciones separadas para cliente, compositor y preparacion
de sesion. Un cliente no debe recibir automaticamente dispositivos de entrada
ni autoridad KMS. El render node y el nodo primario DRM tienen papeles distintos;
eso debe comprobarse en la configuracion y driver reales.
[DRM: nodos y acceso de usuario](https://docs.kernel.org/gpu/drm-uapi.html).

Medir la carga elegida sirve para proponer syscalls, rutas y fd. No basta
convertir la traza en concesiones: hay que comprobar argumentos, origen y
vida de los fd y casos de rechazo. No se han ampliado permisos de caja en
esta ronda, ni se ha concedido acceso de dispositivos a programas de prueba.

## Secuencia de investigacion para acompanar la integracion

1. Fijar compositor, versiones y backend del sistema que se esta construyendo.
2. Probar primero el banco aritmetico sin dependencias graficas, como aqui.
3. Anadir cliente minimo y pruebas de protocolo en un entorno headless o
   anidado, tras comprobar que el backend exista en esa build. Weston documenta
   varios backends: [ejecucion de Weston](https://wayland.pages.freedesktop.org/weston/toc/running-weston.html).
4. Resolver SIGBUS y probar SHM truncado en la VM con recursos limitados.
5. Preparar perfiles de permisos por papel y repetir controles validos.
6. Validar despues KMS/entrada y finalmente DMA-BUF/GPU en el hardware.

La investigacion por categorias acompana el trabajo de integracion, sin
dar por instaladas dependencias ni cambiar init, particiones o arranque.
La ronda 9 guarda su evidencia de C por ejecucion; el resultado de esa
suite no certifica que el escritorio ya funcione o este endurecido.
