# Evidencia de entrega: estado y contexto real

Los dos primeros puntos del plan quedan implementados: estados visibles en el
Museo del SUPER GRAFO e identidad por contenido del entorno usado para revisar.
No se convierte un diagnostico estatico en una vulnerabilidad confirmada.

| Estado | Significado |
|---|---|
| Revisado | Se completaron las comprobaciones seleccionadas y sus identidades coinciden en la ultima consulta |
| Caducado | Cambiaron fuentes, includes, configuracion, motor o herramientas/dependencias |
| Pendiente | No hay informe vigente completo, falta una herramienta o el contrato no se pudo comprobar |
| Diagnostico | El compilador/analizador produjo senales que requieren revision y, cuando proceda, reproduccion |

`runtime_failure_confirmed` permanece falso en este flujo. Solo una reproduccion
separada podria confirmar un fallo de ejecucion; ni un patron lexico ni una
advertencia aislada bastan. Se corrige tambien el mensaje «Nada falla» del Museo:
la ausencia de senales no demuestra ausencia de errores.

## Identidad registrada

- Fuente, cabeceras propias, perfil y motor con SHA-256.
- Configuracion de cada una de las 72 pasadas sobre las 18 unidades seleccionadas.
- Closure de includes emitida por GCC/Clang: 290 dependencias en esta entrega,
  incluidas cabeceras del sistema y recursos de los compiladores.
- Binarios de GCC/Clang, backend cc1, Python y Bubblewrap, mas las bibliotecas
  dinamicas obtenidas con ldd exclusivamente sobre esos binarios de sistema.
- Fotografia inmutable de los archivos propios para compilar, comprobacion del
  contexto antes/despues y rechazo si cambian durante la revision.

Se registran las dependencias antes de compilar y se comparan con las que emite
la pasada real. Los archivos propios se copian una sola vez por ronda, no una
vez por cada pasada. El sistema se monta en solo lectura en Bubblewrap, sin
red del anfitrion, con presupuestos de CPU, memoria, archivo y tiempo.
El origen externo del sistema no se autentica criptograficamente: se comprueba
identidad de bytes, no confianza del proveedor ni ausencia de una intruson de root.

El empaquetador revisa ademas con `/usr` y bibliotecas de la imagen destino,
en su propio namespace, como usuario del taller. Asi no se atribuye al ULTRA
instalado una comprobacion hecha contra otras cabeceras del anfitrion.
Los enlaces de /lib, /lib64 y /bin conservan su estructura, evitando diferencias
artificiales de rutas entre el namespace del banco y el arranque real.

## Usarlo

En la copia del proyecto abierta desde `ultra-revisar-proyecto abrir`, entrar al
**Museo** y desplegar **EVIDENCIA // CONTENIDO Y VIGENCIA**. El boton
**Comprobar contenido actual** revalida los hashes en otro hilo. Las filas se
ordenan para mostrar primero lo que requiere atencion; clic abre el archivo.
Se indica cuando se realizo la ultima comprobacion.

Un nuevo analisis del proyecto invalida la visualizacion anterior. No se escanean
bibliotecas en cada cuadro de renderizado: hay como maximo una comprobacion
activa por panel. Un cambio externo al proyecto requiere revalidar; la puerta
de entrega siempre comprueba contenido de nuevo antes de aprobar.

Para MCP o terminal:

```bash
ultra-carpetas --consulta revision_estado /ruta/al/proyecto '{"limite":5}'
ultra-revisar-proyecto estado
ultra-revisar-proyecto revisar
```

El GUI y el lanzador ejecutan exclusivamente el motor instalado en
`/usr/share/ultra/revision-engine/`, con Python `-I -S`. No importan ni ejecutan
el script de la carpeta examinada. Ese archivo sigue contando como contenido:
si alguien lo modifica, la evidencia caduca. Un informe viejo de esquema 1
queda pendiente hasta repetir la revision, nunca se migra dandolo por aprobado.

## Pruebas y limites

Las 17 pruebas del motor comprueban cabeceras modificadas conservando fecha y
tamano, cambios de configuracion, binarios/bibliotecas con igual version,
comprobaciones ausentes, errores de herramienta, duplicados, rutas invalidas,
archivos fuera de presupuesto y FIFO que no deben bloquear el lector.
Las 60 pruebas Rust incluyen salida acotada, rechazo de rutas externas y de
un resumen verde que contenga filas pendientes. En el sistema instalado se
comprueban tanto el comando como la consulta real de SUPER GRAFO.

Esto cubre el alcance estatico propio seleccionado. No acredita todo C/C++,
el kernel, los juegos ni todas las dependencias transitivas de cualquier
programa. Los resultados no tienen firma contra falsificacion por alguien con
control del usuario/administrador, y no eliminan todas las posibles carreras
de actualizacion de un sistema hostil. Se conserva el contexto historico y no
se promete seguridad absoluta ni mejoras de FPS por el solo hecho de revisar.

Quedan pendientes los siguientes tramos del plan: AST/vida util de C++, TSan y
MSan separados, fuzzing con cobertura, presupuestos de rendimiento y el mapa
fuente-instruccion-HEX de CASM Native. La ruta BMO-X no cambia en esta ronda.

## Entrega PC del 26 de octubre de 2026

La imagen final paso el banco FAT32/UEFI de QEMU: 175 comprobaciones verdes,
incluidos Hyprland, SUPER GRAFO instalado, cambios de identidad detectados y
persistencia tras reiniciar. Se reutiliza el kernel previamente auditado;
esta ronda modifica el motor de revision y su interfaz. La grafica del banco
es software y no acredita rendimiento de la RTX.

Las fuentes probadas, informes de taller y destino y resultados se conservan
en `evidencias/vigencia/2026-10-26/`. `fuentes-git.json` registra los SHA-256
con finales LF para contrastar el contenido de Git con lo probado en Linux.

Instalada en A: para el perfil PC, con kernel y EROFS comprobados por SHA-256.
`datos.ext4` mantiene exactamente su hash anterior; los informes previos se
respaldaron en el proyecto Windows. Identificador de revision entregada:
`0320900e52d9d3b6`. Imagen EROFS: 2058067968 bytes, compatible con FAT32.
