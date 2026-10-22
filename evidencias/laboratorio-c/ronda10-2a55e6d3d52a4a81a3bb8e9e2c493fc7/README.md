# Ronda 10: cadenas e instalacion del guardian

QEMU del taller, GCC 16.2.1 20260810, Linux 7.2.11-arch1-1 x86-64. No se
conectaron discos personales ni se cambio la imagen instalada en A:.

## Resultados

- `campo-minas-ronda10.txt`: 36 rechazos de compilacion; ejecucion de 62 minas:
  53 VERDE, 1 NEUTRA, 8 AMARILLO y 0 ROJO. No aumenta la cobertura global.
- `cruces-guardianes-ronda10.txt`: cero ROJO en cinco perfiles.
- `hilos-*.txt`: control atomico e incompatibilidad comprobados; carrera AMARILLO.
- `dominios-ronda10.txt`, `buffers-ronda10.txt`, `perfiles-ronda10.txt` y
  `aislamiento-ronda10.txt`: controles existentes ejecutados en esta misma ronda.
- `cadenas-ronda10.txt` y `cadenas/`: 10368 combinaciones por cada uno de cuatro
  perfiles (1308 aceptaciones, 9060 rechazos), cinco controles de frontera
  PROT_NONE, cuatro argumentos nulos y cinco controles del parser real tarjeta.c.
  Los mismos casos repetidos en cuatro perfiles no son 41472 casos distintos.
- `instalacion-ronda10.txt` y `instalacion/`: diez escenarios por perfil, cuatro
  perfiles; pila y handlers reales consultados, ocho errores simulados detenidos
  con exit 78 y trampa intencional con salida 132.

## Atribucion

Las cadenas se comparan con un oraculo independiente de snapshot, incluyendo
alias y canarios. El contraste `legacy-*` viola deliberadamente el contrato
de la API antigua al suministrar un origen sin NUL junto a una pagina protegida:
produccion termina con 139 y ASan con 1; la API nueva lo rechaza y conserva destino.
No implica que todo uso de la API antigua sea incorrecto.

La instalacion inyecta retornos mediante wrappers de linker solo en las pruebas.
No hay inyeccion ni hooks en los binarios de produccion. Los rechazos requieren
paso/errno esperados, exit 78 y ausencia del marcador de continuacion. Un timeout
o fallo del sandbox no se cuenta como deteccion. Los casos invalidos se ejecutan
en namespaces nuevos sin red/home, con limites de CPU/tiempo y recursos.
La RAM esta limitada por la VM de 4 GiB; no se afirma un cgroup de 512 MiB.

`hashes-taller.txt` dentro de cada suite conserva hashes de fuentes y binarios
de la VM; los binarios no se incluyen en Git. `SHA256SUMS` en esta carpeta
identifica evidencia publica y fuentes locales normalizadas a LF. El origen
del manifiesto es la raiz del repositorio, no esta carpeta.

Los pasos iniciales de cadenas quedaron en build como prevalidacion. La primera
revision rechazo un snapshot no inicializado del harness con el wrapper memcpy;
se corrigio el test sin relajar el perfil. Aqui se publica la auditoria final
con las fuentes corregidas y el guardian de instalacion nuevo.

Ver [contrato de cadenas](../../../docs/CADENAS-ACOTADAS.md) e
[instalacion del guardian](../../../docs/GUARDIAN-INSTALACION.md). Pendientes:
propiedad/vida util general y construccion/pruebas de arranque antes de desplegar.
