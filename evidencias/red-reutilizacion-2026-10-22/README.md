# Reutilizacion DHCP en ULTRA

Ambos perfiles construidos aprobaron **9/9 comprobaciones cada uno** y se
instalaron en A:. Los cuatro hashes EFI/EROFS coinciden con sus manifiestos;
`datos.ext4` conserva el hash previo. Los kernels no se recompilaron.

| Perfil | Primera espera sin cable | Reentrada con DHCP pendiente |
|---|---:|---:|
| Generico | 12,36 s | 0,219 s |
| PC | 11,77 s | 0,172 s |

Son mediciones de una ejecucion virtual por perfil. La primera llamada inicia
el supervisor; la segunda conserva el mismo PID sin reiniciarlo. No son dos
versiones de Steam ni una comparacion de su velocidad de descarga.

Resultados: `generico.json`, `pc.json` e `instalacion-A.json`. Los hashes de los
archivos publicos y fuentes verificadas estan en `SHA256SUMS`.

La segunda invocacion con DHCP pendiente reutiliza un supervisor comprobado,
sin enviarle otra orden de preparacion ni repetir la espera inicial. Sigue
devolviendo aviso de configuracion pendiente cuando corresponde. El primer
arranque mantiene su espera acotada y dhcpcd conserva las renovaciones y la
recuperacion al conectar o reconectar un cable.

El pidfile debe estar bajo /run, ser regular, pertenecer a root y no permitir
escritura de grupo/otros. Se verifica PID vivo no zombie, UID root o cuenta
dhcpcd e identidad de ejecutable mediante dispositivo/inodo. La comprobacion
acepta el daemon con separacion de privilegios dentro del chroot. Las llamadas
se serializan con una espera maxima de dos segundos; el daemon no hereda ese
bloqueo ni el bloqueo de sesion del escritorio.

`scripts/probar_enlace_tardio.py` prueba copias desechables de cada imagen con
NIC virtual privada. Sin argumento de fuente adicional se comprueban las fuentes
ya construidas, sin inyectar codigo. El modo opcional de inyeccion se reserva a
preflight y no sustituye la validacion final. Los resultados finales registran
esperas, casos aprobados y hashes de los auxiliares instalados. Se verifica
el mismo PID entre llamadas, bloqueo liberado, DHCP tras conectar tarde, DNS,
HTTPS con CA valida como UID 1000 sin capacidades, reconexion y reentrada lista.
Tambien se ejecuta el informe ampliado y se exige salida cero.

No se adjuntan discos personales a estas pruebas. Los logs completos quedan
en build, ignorado por Git. El respaldo de A: queda en el proyecto Windows.
Los recibos publicos contienen hashes de imagen y preservacion de datos,
sin sesiones o credenciales.

Esta mejora evita trabajo repetido del preparador de ULTRA. No mide FPS,
velocidad de descarga ni el tiempo del verificador de Warframe. Steam y
Warframe gestionan sus propias conexiones HTTP/TLS. No se desactiva su
integridad ni se fuerza la reutilizacion de sus sesiones. El enlace fisico
sigue limitado a los 100 Mbps negociados previamente observados. FQ-CoDel
continua siendo una propuesta de laboratorio sin activar en el kernel ULTRA.

Referencia primaria: [dhcpcd, consulta de pidfile y funcionamiento del daemon](https://man.archlinux.org/man/dhcpcd.8.en).
