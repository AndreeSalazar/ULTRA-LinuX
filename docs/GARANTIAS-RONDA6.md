# Garantias concretas y evidencia: ronda 6

Se investigaron los contratos de Landlock, seccomp, ioctl/fcntl, arch_prctl y PID
namespaces en fuentes primarias. Las reglas se aplicaron al perfil BusyBox de la
caja y se verificaron en el kernel Arch 7.2.11 del taller. No se ha instalado ni
acreditado un nuevo arranque de ULTRA con esta ronda.

## Contratos reforzados

| Contrato | Implementacion | Evidencia |
|---|---|---|
| Una llamada no declarada no obtiene ALLOW | Filtro de arquitectura, numero y lista de 46 declaraciones | Equivalencia simbolica del BPF real, prueba de las 386 entradas y 14 controles en kernel |
| Los comandos internos no son permisos abiertos | Listas por argumento de ioctl, fcntl, prctl y arch_prctl | Prueba simbolica de todos los bits; 65536 comandos ioctl enumerados; comandos desconocidos denegados |
| No se entrega un socket heredado al programa por stdio | Validacion de tipo de fd 0/1/2 antes del exec y cierre de fd >= 3 | Sockets UNIX en cada fd rechazados; ningun byte enviado, ni diagnostico por stderr socket |
| Leer datos no concede ejecutarlos | Landlock separa reglas read/write/execute; solo /bin tiene EXECUTE | Ejecutable valido en /tmp rechazado; BusyBox en /bin sigue funcionando |
| Una ruta permitida no concede su destino externo | Landlock comprueba el objeto resuelto | Enlace de /tmp a /datos/secreto rechazado sin revelar el marcador |
| Solo lectura incluye impedir truncado | Landlock FS_TRUNCATE; ahora se exige ABI >= 6 | Archivo de control con permisos DAC 0666 no truncado y contenido conservado |
| El guardia no retorna al fallo al ser PID 1 | Salida inmediata tras la bandera en PID 1, en vez de depender de senal por defecto | Rechazo explicito termina con 134; trampa con 132, bajo timeout de tres segundos |

## Garantia del filtro: que se demostro

`caja --dump-filter` exporta los bytes BPF construidos por el ejecutable real.
`scripts/verificar-seccomp.py` los interpreta simbolicamente con Z3 4.16.0.
El contrato independiente declara arquitectura, numeros permitidos, rechazos de
x32/i386, PROT_EXEC y las listas escalares por argumento. Los numeros de syscall
proceden del catalogo del commit fijado.

Se consulto si **existe alguna entrada de seccomp_data que haga divergir el filtro
y el contrato**. Z3 devolvio UNSAT: no existe dentro del modelo. Las 16 palabras
simbolicas de 32 bits representan los 64 bytes/512 bits del registro completo:
numero, arquitectura, direccion de instruccion y seis argumentos. Los bits altos
de argumentos permanecen libres; las reglas de comandos usan sus 32 bits bajos,
como los tipos int/unsigned int de estas interfaces en x86-64.

Esto verifica las **156 instrucciones emitidas**, no solamente ejemplos de numeros
o argumentos. La prueba incluye un control: un filtro mutado que permite todo
produce un contraejemplo y se rechaza. Un timeout o resultado unknown de Z3 no
se acepta como prueba. El reporte y el manifiesto incluyen hashes de filtro y fuentes.

Esta es una garantia delimitada de equivalencia del modelo cBPF y el contrato.
Depende de que el decodificador, Z3 y el kernel interpreten correctamente sus
instrucciones y de que el filtro instalado sea el verificado. No prueba ausencia
de UB en C ni ausencia de vulnerabilidades en el kernel, compilador o hardware.
El resultado UNSAT registrado no es un certificado revisado por un segundo prover.

## Listas de comandos

- ioctl: TCGETS, TCSETS, TCSETSW, TCSETSF, TIOCGWINSZ, TIOCGPGRP y FIONREAD.
  No se concede TIOCSTI ni la configuracion arbitraria de dispositivos.
- fcntl: F_GETFD, F_SETFD, F_GETFL, F_SETFL, F_DUPFD y F_DUPFD_CLOEXEC.
- prctl: PR_GET_NO_NEW_PRIVS, PR_GET_SECCOMP, PR_GET_DUMPABLE, PR_GET_NAME y
  PR_SET_NAME. El ultimo se observo en BusyBox al ejecutar subprogramas; cambia
  el nombre del propio proceso, no concede privilegios.
- arch_prctl: ARCH_SET_FS, ARCH_GET_FS y ARCH_GET_GS. No se concede desactivar
  ni desbloquear la pila sombra mediante otras operaciones.

Otros argumentos de esos comandos y otras syscalls concedidas aun no estan
caracterizados por completo. El filtro no desreferencia buffers del usuario.

## Red y datos: alcance real

La caja mantiene un namespace de red separado y seccomp deniega socket,
socketpair, connect y las demas interfaces no declaradas. Landlock ahora exige
ABI >= 6: se manejan bind/connect TCP sin concesiones y se activan los scopes
de senales y sockets UNIX abstractos. Se agregan derechos de UDP desde ABI 10
y resolucion UNIX desde ABI 9 cuando los headers los conocen y el kernel los ofrece.
La capa principal sin red sigue siendo seccomp mas el rechazo de sockets heredados;
los scopes no convierten Landlock en un filtro universal de protocolos.

Stdio permite archivos regulares, pipes, terminales y /dev/null: son canales de
datos explicitamente concedidos por el lanzador. **Un pipe puede llegar a un
programa externo que tenga red.** Este perfil no promete prevenir toda exfiltracion
de datos que el programa ya tiene permiso de leer y escribir a una salida concedida.
Para ello haria falta una politica de flujo de informacion y un mediador de salida.

Otros pendientes visibles: /tmp sigue siendo el directorio concedido del sistema
contenedor, sin tmpfs privado por caja; no hay cuotas agregadas con cgroups ni
limite propio de tiempo de pared. Las ocho minas C AMARILLO continuan abiertas.
No se concede una garantia absoluta sobre internet, todos los datos o toda UB.

## Resultados y repeticion

- Caja real: **17/17** con fixtures sinteticos de datos; las 13 pruebas base se
  mantienen para el sistema instalado, sin crear ni modificar datos personales.
- Kernel del taller: **14/14** controles de filtro; sockets heredados: **3/3**.
- Rechazo/trampa como PID 1: **2/2**. El ensayo externo usa unshare --kill-child
  para terminar su PID 1 al morir el lanzador; se corrigio tras una prueba que
  dejo procesos bloqueados y se cerraron unicamente esos procesos identificados.
- Regresion C: 62 minas, 53 VERDE, 1 NEUTRA, 8 AMARILLO y 0 ROJO; 36 rechazos de
  compilacion, controles validos de cruces y control atomico sin carreras.

Ejecutar `./probar-syscalls.ps1` y `./probar-guardianes.ps1` desde Windows.
Z3 se instalo desde el paquete oficial Arch `python-z3-solver` dentro del taller.
Los scripts no instalan paquetes automaticamente ni reducen reglas al fallar.

Evidencias principales: `evidencias/seccomp-formal-ronda6.txt`,
`seccomp-filtro-ronda6.bpf`, `seccomp-SHA256SUMS-ronda6.txt`,
`syscalls-ronda6.txt`, `syscalls-ronda6.csv`, `campo-minas-ronda6.txt` y los dos
reportes `guardian-pid1-*-ronda6.txt`.

Fuentes primarias:

- [Landlock: derechos, ABIs y limites](https://docs.kernel.org/userspace-api/landlock.html).
- [Seccomp: datos, acciones y restricciones](https://man7.org/linux/man-pages/man2/seccomp.2.html).
- [PID namespaces y el comportamiento especial de PID 1](https://man7.org/linux/man-pages/man7/pid_namespaces.7.html).
- [arch_prctl](https://man7.org/linux/man-pages/man2/arch_prctl.2.html) y [fcntl](https://man7.org/linux/man-pages/man2/fcntl.2.html).
- [Z3, proyecto oficial](https://github.com/Z3Prover/z3).
