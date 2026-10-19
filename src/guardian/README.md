# Guardianes C: dos categorias

- [Produccion](produccion/README.md): implementacion compartida, revision y
  perfil GCC para programas C de usuario.
- [Laboratorio](laboratorio/README.md): minas, controles, perfiles de diagnostico
  y comparacion sin protecciones.

Los archivos ultra_guard.c/h de esta carpeta son entradas de compatibilidad:
incluyen la unica implementacion, que vive en produccion. minas.c conserva
la entrada del diagnostico que el arranque actual ya incluia. No hay dos
copias de los guardianes.

Los lanzadores PowerShell crean directorios unicos de transferencia, trabajo
y evidencia. Una auditoria no reemplaza los artefactos del kernel ni los
informes de otra ejecucion. El catalogo de syscalls generado queda en su
ejecucion, sin modificar automaticamente el header de produccion.
