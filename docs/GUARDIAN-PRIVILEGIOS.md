# Guardianes: entender el fallo y reducir privilegios

Ronda 7. Las pruebas se ejecutaron en el taller Arch Linux, kernel
7.2.11-arch1-1, GCC 16.2.1. No acreditan un arranque nuevo de ULTRA ni que
el sistema completo haya dejado de usar root.

## UB: una biblioteca tolerante no cambia la regla de C

`ctype` recibe un `int`, pero solo admite EOF o valores representables como
`unsigned char`. Que una libc tenga una tabla ampliada para negativos y una
prueba parezca funcionar no convierte ese uso en C definido. Tampoco UBSan
promete comprobar cada precondicion de biblioteca.

Las nuevas APIs `ULTRA_TOLOWER`, `ULTRA_TOUPPER` y `ULTRA_ISALPHA` rechazan el
entero fuera de dominio antes de llamar a la biblioteca. Cada argumento se
evalua una vez. Para un byte almacenado en `char`, el llamador puede convertir
a `unsigned char`; para el resultado de `fgetc`, primero debe conservar y
comprobar EOF. La API no adivina cual era la intencion.

[Prueba dedicada](../src/guardian/probar-ctype.c): en cada uno de los perfiles
produccion y diagnostico se comparan los 256 bytes y EOF con la biblioteca;
se prueba evaluacion unica y se rechazan nueve entradas invalidas
(tres por operacion) con codigo 134 y bandera `UB-CTYPE`. Cada invocacion
corre en el aislamiento habitual, sin ejecutar la llamada ctype invalida.

[Evidencia de dominios](../evidencias/dominios-ronda7.txt): 18/18 rechazos,
dos controles completos de entradas validas. No es una prueba exhaustiva de
todos los enteros, locales o funciones ctype. Las llamadas directas a la
biblioteca siguen necesitando revision.

El campo anterior conserva sus 62 minas: 53 VERDE, una NEUTRA, ocho AMARILLO
y cero ROJO; los 36 fixtures de compilacion fueron rechazados. La nueva suite
se cuenta aparte, para no inflar el numero de minas del campo.

Fuentes primarias: [SEI CERT STR37-C](https://cmu-sei.github.io/secure-coding-standards/sei-cert-c-coding-standard/rules/characters-and-strings-str/str37-c/),
[GCC: instrumentacion y alcance](https://gcc.gnu.org/onlinedocs/gcc/Instrumentation-Options.html).

## Root: quitar autoridad exige mas que cambiar el usuario

Linux representa identidades numericas y reparte privilegios en capacidades.
Eliminar una entrada llamada root de `/etc/passwd` no elimina UID 0 ni sus
reglas. Un UID distinto de cero tambien puede tener capacidades. Por eso
la propiedad comprobada aqui es sobre credenciales y conjuntos de capacidades.

La caja exige antes de instalar seccomp y ejecutar el programa:

- UID y GID reales, efectivos, guardados y de acceso al sistema de archivos: 65534.
- Ningun grupo suplementario.
- Capacidades efectivas, permitidas, heredables, ambientales y limite: vacias.
- `no_new_privs=1`, que se hereda y no puede desactivarse.

La implementacion desactiva keepcaps, limpia capacidades ambientales,
retira cada capacidad limite, cambia todas las identidades, vacia tambien
el conjunto heredable mediante capset y vuelve a comprobar el estado real.
Un fallo impide ejecutar el programa. Un ABI con mas de 64 capacidades
se rechaza: no se omite silenciosamente la parte desconocida.

Por que importa: al cambiar UID, el kernel no vacia automaticamente el
conjunto heredable. Vaciar solamente el conjunto limite tampoco demuestra
que los conjuntos efectivos y permitidos esten vacios.

[Prueba de identidad](../src/caja/probar-identidad.c): un hijo desechable del
taller prepara deliberadamente CAP_CHOWN heredable y ambiental y keepcaps,
aplica la misma retirada que caja, comprueba los cinco conjuntos y verifica
que setuid(0) y setgid(0) reciben EPERM. No modifica las credenciales del padre.
Se comprueba ademas que el estado root inicial no cumple la identidad exigida.

## Syscalls: comprobar decisiones y el rechazo efectivo

El filtro conserva 46 concesiones de 386 syscalls nativas y deniega 340 por
defecto. No necesita conceder capget, capset ni getresuid para inspeccionar
credenciales: esa inspeccion ocurre antes de instalarlo.

Los controles reales del filtro pasan de 14 a 20. Los seis nuevos comprueban
setuid(0), setgid(0), capset, unshare, setns y mount. Cuando no hay trazador,
RET_TRACE se observa como ENOSYS; esto distingue un rechazo del filtro de un
EPERM que podria haber producido solo la comprobacion de privilegios.
Las sondas de mount/setns/capset usan argumentos invalidos y unshare usa cero;
no son intentos de montar recursos ni de entrar en otros namespaces.

En la caja completa, RET_TRACE detiene el proceso para su supervisor: este
destruye la caja sin reanudar la llamada prohibida. Los 17 controles de la
caja y los controles de sockets heredados siguen pasando. La equivalencia
del filtro de 156 instrucciones con su contrato sigue resultando UNSAT en
Z3, bajo el modelo documentado; no demuestra toda la semantica del kernel.

[Informe de syscalls e identidad](../evidencias/syscalls-ronda7.txt),
[verificacion del filtro](../evidencias/seccomp-formal-ronda7.txt).

## Donde sigue root y que significa retirarlo del uso diario

| Componente | Estado y necesidad actual |
|---|---|
| Programa dentro de caja | Identidad 65534 comprobada antes de exec; sin capacidades y sin adquirir privilegios por exec. |
| Preparador/supervisor de caja | Sigue privilegiado: crea namespaces, prepara aislamiento y vigila mediante ptrace. Esta ronda no reduce sus privilegios. |
| Init de ULTRA | Sigue preparando montajes y hardware con privilegios. |
| Consola de ULTRA | El codigo actual hereda las credenciales de init al ejecutar la shell; todavia permite una sesion root. |

La direccion propuesta es una sesion cotidiana sin privilegios y operaciones
administrativas reducidas a interfaces concretas. Para convertirla en el valor
predeterminado hay que adaptar el lanzamiento de caja y la administracion de
particiones/hardware: la caja actual no es un lanzador rootless ni setuid.
Cambiar solo la shell a 65534 dejaria sin acceso a esas funciones. No se ha
alterado el arranque ni se han eliminado cuentas en esta ronda.

Fuentes primarias: [capabilities(7), conjuntos y cambios de UID](https://man7.org/linux/man-pages/man7/capabilities.7.html),
[no_new_privs del kernel](https://www.kernel.org/doc/html/latest/userspace-api/no_new_privs.html),
[seccomp(2), RET_TRACE](https://man7.org/linux/man-pages/man2/seccomp.2.html).

Persisten los limites documentados de caja: kernel compartido, pipes que
pueden comunicar con procesos externos autorizados, directorio temporal
sin separacion por caja y ausencia de cuotas agregadas por cgroup. Quitar
root de los programas no resuelve esas propiedades ni demuestra ausencia de UB.

Para reproducir en el taller existente:

```powershell
./probar-guardianes.ps1 -Ronda 7
./probar-syscalls.ps1 -Ronda 7
python scripts/museo-ub.py
python scripts/probar-museo-ub.py
python scripts/museo-ub.py --check
```
