# Reloj de Peru: verificacion e instalacion

Fecha local: 2026-10-22, America/Lima. Base Git 527c682 mas las fuentes de esta
entrega. Se reconstruyeron generico #50 y PC #36, conservando la puerta ELF
de los cinco ejecutables propios. Los manifiestos por perfil registran los
artefactos y paquetes, incluido chrony 4.9.

## Resultado

- Generico: **124 controles verdes, cero rojos, salida 0**.
- PC: **125 controles verdes, cero rojos, salida 0**.
- En ambos: epoch 0 se muestra como 1969-12-31 19:00 -0500 en BusyBox y Arch;
  el mes se muestra como diciembre mediante LC_TIME=es_PE.UTF-8. Estas consultas
  no ajustan el reloj a 1969: solo formatean un instante fijo.
- En ambos: el auxiliar real completo el ajuste NTS y publico su marca UTC.
  Las transcripciones incluyen su resultado y el log de chronyd. Las pruebas
  de escritorio, Brave, persistencia y apagado limpio tambien pasaron.
- Los informes incluyen fecha_utc y fecha_local con offset -05:00.

configuracion-imagenes.txt comprueba directamente /etc/localtime, /etc/timezone,
el locale y la configuracion del cliente en las dos imagenes. Los hashes del
auxiliar y chrony.conf coinciden con las fuentes locales. La barra usa Lima,
locale peruano y clic derecho para abrir ultra-hora.

## Pruebas separadas

fixtures.txt y los archivos fail/success/busy prueban tres escenarios con
chronyd **simulado**, en namespaces sin red. Comprueban ausencia de marca tras
fallo, reutilizacion de un exito reciente y exclusion de dos intentos concurrentes.
El stub no ajusta ningun reloj; no se presenta como prueba de NTS real.

consulta-nts.txt es una consulta real aparte, con chronyd -Q: respondio con
salida 0 y calculo un desfase estimado de 0,024343 segundos, **sin corregir el
reloj del taller**. No es una medicion de ping ni una garantia de exactitud.
El ajuste real se comprobo despues dentro de las dos VM de arranque.

El log conserva el aviso de permisos que deshabilita el socket Unix de comandos
en el directorio publico de estado. Ese socket no se utiliza; el cliente puntual
funciona y no ofrece puertos de servicio NTP/control por red. No se oculta el
aviso ni se interpreta como un fallo del ajuste que termino correctamente.

## Promocion

[instalacion-A.json](instalacion-A.json) registra los hashes copiados a A:,
la zona y el locale. Se respaldaron antes dos kernels y dos sistemas en el
proyecto Windows, bajo build/respaldo-antes-hora-2026-10-22 (ignorado por Git).
respaldo-previo.json conserva sus hashes. datos.ext4 no cambia durante la copia.
No se reparticionan discos ni se cambia el reloj/configuracion de Windows.

Falta confirmar este nuevo arranque en el PC fisico. QEMU no valida la RTX
fisica; esta entrega no promete cambios en FPS ni resuelve el aviso SRSO de NVIDIA.
El taller y las VM de prueba quedaron apagados tras recuperar los resultados.

La sincronizacion es puntual al abrir el escritorio, en segundo plano y con
limite; no es un servicio continuo. Si falla la red o NTS, queda pendiente sin
bloquear la sesion. Se reutiliza un ajuste correcto durante una hora del mismo
arranque. No se activa rtcsync ni se invoca hwclock para escribir el RTC.

SHA256SUMS cubre esta carpeta con texto LF, salvo el propio manifiesto.
Ver [configuracion y limites](../../docs/HORA-PERU.md).
