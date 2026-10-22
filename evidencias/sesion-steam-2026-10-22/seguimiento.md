# Verificacion de Warframe y siguiente prioridad

El usuario confirma que el lanzador propio de Warframe esta verificando y tarda.
No se interpreta esa espera como un cierre del juego ni se acredita una partida.

El soporte oficial explica que Verify compara archivos con la version del CDN
y puede reparar o reemplazar contenido. Por tanto puede haber lectura local y
transferencias; no es solo comprobar si existe el ejecutable.

Fuente: [Warframe Support: Update failed](https://support.warframe.com/hc/en-us/articles/360044025712-Update-failed).

Durante los intervalos medidos la interfaz completa recibio 2,316 GiB, con un
pico medio de 11,723 MiB/s por intervalo. El enlace observado es de 100 Mbps:
su limite bruto equivale a unos 11,92 MiB/s. Esto muestra intervalos cercanos
al limite del enlace; no atribuye todo el trafico a Warframe ni mide su lectura
de archivos o el progreso del verificador.

Quedan 2,8 GiB en el ext4. Si el verificador reemplaza archivos grandes, la capa
Linux puede necesitar mucho mas espacio aunque los originales sigan en C:.
Optimize tambien puede reescribir contenido: no se propone ejecutarlo como
remedio a ciegas con este margen. No se ha detectado ENOSPC en el informe.

La inspeccion complementaria [almacenamiento-sololectura.json](almacenamiento-sololectura.json)
usa una VM de mantenimiento separada, sin tarjeta de red ni clientes, con el
archivo de D: y su dispositivo virtual en solo lectura. ext4 se monta con
`ro,noload,nosuid,nodev,noexec`; el kernel informa `norecovery` como equivalente
de noload. Se conserva el tamano y mtime del archivo. No se leen contenidos de
cuentas o sesiones: se cuentan bloques asignados con du, sin seguir enlaces.

Dentro de la capa Steam, que ocupa 6,56 GiB:

| Componente | GiB asignados aproximados |
|---|---:|
| Proton Experimental y dos Steam Linux Runtime | 2,87 |
| Cambios/archivos de Warframe en la capa Linux | 2,21 |
| Shaders | 0,74 |
| Prefijos de compatibilidad | 0,74 |

El total no incluye como copias los 52,4 GiB originales de Warframe en C:.
Esto tampoco demuestra que la verificacion haya terminado. El home ocupa
otros 6,09 GiB aproximadamente; incluye el cliente y herramientas propias de
compatibilidad, por lo que sus subtotales no deben sumarse otra vez.

Lo siguiente es obtener del verificador su progreso y errores de escritura,
si aparecen, y evaluar almacenamiento. No se deduce que estos componentes
sean basura eliminable ni se propone saltarse la verificacion de integridad.

La imagen actual de D: mide 16 GiB. D: tiene aproximadamente 10,57 GiB libres:
no alcanza para agregar los 16 GiB que llevarian la imagen a 32 GiB. Cualquier
ampliacion necesita comprobar el espacio, respaldo y un procedimiento fuera
de linea que preserve UUID, datos y la configuracion de montaje. No se amplio
el archivo, no se cambiaron los originales de C: ni se borraron caches.
