# Menu del escritorio con teclado

2026-10-19. Compilados y probados los perfiles generico y PC; ambos
instalados en A:. Desde la consola de ULTRA basta escribir `e`.

Flechas arriba/abajo y Enter; 1-6 seleccionan, Enter ejecuta; Esc o Q sale.
Opciones: automatico, NVIDIA, recuperacion software, ultimo diagnostico,
estado NVIDIA y volver a la consola. El escritorio conserva el preparador
y las credenciales de W3b. Al terminar la sesion, una tecla vuelve al menu.

## Verificacion

| Prueba | Resultado |
|---|---|
| Terminal Linux PTY | Flechas, Enter, numeros, Esc, Q, modos, error y diagnosticos aprobados |
| Generico BMO | 70 VERDE, 0 ROJO, proceso con salida 0 |
| PC BMO | 74 VERDE, 0 ROJO, proceso con salida 0 |

La prueba PTY usa el menu real con backends sustituidos para verificar los
modos solicitados, un fallo NVIDIA y el retorno al menu. Se reproduce en
Linux con `python scripts/probar_menu.py src/rootfs/bin/ultra-menu`.

Las dos pruebas QEMU arrancan las imagenes compiladas, ejecutan `e` en su
consola y eligen NVIDIA con flecha y Enter. QEMU no tiene NVIDIA: se exige
el rechazo por falta del dispositivo. Luego verifican diagnostico con numero
y Enter, Q para volver a la consola y escritorio automatico con Foot visible
(97 % de pantalla), teclado, salida limpia y persistencia tras reiniciar.
Cage y Foot tienen UID/GID 1000, capacidades cero y NoNewPrivs=1.

Los registros *-pruebas.txt y *-serie.log y las capturas documentan esos
arranques. fuentes.sha256 y menus-empaquetados.sha256 permiten comparar
el menu de ambas raices con la fuente probada. Los listados de paquetes
y manifiestos corresponden a estas compilaciones. Los registros de texto
se normalizaron en finales de linea y espacios finales; originales en
build/menu-registros-originales, fuera de Git.

## Instalacion

verificacion-instalacion.txt confirma por SHA256 los dos EFI y los dos
sistemas instalados frente a los artefactos probados. datos.ext4 conserva
su hash anterior a la instalacion:
21869E552FE75AEE74298F7E60F68234C98FE120DE6F7E5EA318DDE6CC0DA5C5.

Las copias verificadas del arranque anterior estan en
build/antes-menu-0ff22fd58a5940079c7b9f6f8268a11d; el registro local es
build/instalacion-menu.json. Se actualizaron archivos en la particion BMO;
no se cambiaron las entradas de firmware ni se reemplazaron datos.

## Prueba fisica pendiente

F11, perfil PC, `e`, flecha abajo, Enter. Tras cerrar Foot con `exit`, una
tecla devuelve al menu. Elegir el diagnostico y comprobar el GL renderer
real. Las pruebas QEMU no certifican aceleracion RTX; esta interfaz permite
iniciarla y consultar registros sin escribir comandos largos.
