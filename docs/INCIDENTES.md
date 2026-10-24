# Incidentes: qué se cruzó (Super+Shift+I)

2026-10-24, America/Lima.

Cuando algo falla sin tumbar el sistema, por ejemplo una verificación de Steam
que se cruza con otra tarea, **Super+Shift+I** (o **Super+Espacio > Sistema >
Guardar incidente**) guarda una foto del momento en:

```text
~/.cache/ultra/incidentes/incidente-<fecha UTC>/
```

| Archivo | Contenido |
| --- | --- |
| `resumen.json` | Eventos del kernel (Xid de NVIDIA, OOM, errores de almacenamiento, denegaciones de seccomp con su número de syscall y proceso), memoria, presión del sistema (avg10), espacio libre, montajes de `/juegos`, `/windows` y `/discos` con su modo, y los 12 procesos propios con más memoria |
| `registros/*.txt` | Últimas 200 líneas de los registros de lanzadores de ULTRA y de `content_log.txt` y `stderr.txt` de Steam |

Límites y privacidad:

- Solo lectura y sin root. Los eventos vienen del lector privilegiado que ya
  publica `/run/ultra-kernel/eventos.json`.
- No se copian el registro de conexiones de Steam, cuentas, variables de entorno
  ni líneas de comando. Los correos y los valores con forma de token se tapan.
- Solo se leen archivos regulares propios: un enlace simbólico se rechaza.
- Hasta 8 incidentes guardados. Retira los viejos desde **Sistema > Abrir
  incidentes**.
- Es una foto: muestra qué coincidió, no demuestra la causa por sí sola.
  Revisa los registros antes de compartirlos.

`scripts/probar_incidente.py` lo prueba en un HOME temporal: filtra eventos,
tapa datos, rechaza enlaces, recorta registros enormes y respeta el límite.
