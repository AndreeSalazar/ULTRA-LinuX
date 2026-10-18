# ULTRA LinuX

Sistema propio construido sobre el kernel oficial de Linux, compilado con GCC. Toma ideas de Arch Linux (control y pacman) y de SteamOS (sesión de juego e inmutabilidad), pero no es ninguna de las dos ni usa sus marcas. Hardware objetivo: Ryzen 5 5600X, MSI A320M-A PRO MAX, NVIDIA RTX 3060 12 GB.

## Guardianes en C

El código C propio se compila con guardianes que convierten las sorpresas en errores visibles:

| Guardián | Opciones GCC | Qué evita |
|---|---|---|
| Avisos como errores | `-Wall -Wextra -Werror -Wconversion -Wsign-conversion -Wshadow -Wformat=2 -Wvla -Wnull-dereference -Wimplicit-fallthrough -Wstrict-prototypes` | Conversiones que cambian valores, variables tapadas, formatos peligrosos y `switch` que caen sin querer |
| Analizador estático | `-fanalyzer` | Fugas, doble liberación, uso tras liberar y rutas con NULL, detectados al compilar |
| Trampa de UB | `-fsanitize=undefined -fsanitize-trap=all` | Desbordamiento con signo, desplazamientos inválidos, índices fuera de rango: el programa se detiene en la instrucción culpable en lugar de continuar con un valor basura. No necesita biblioteca, así que funciona en binarios estáticos |
| Memoria | `-fstack-protector-strong -D_FORTIFY_SOURCE=3 -ftrivial-auto-var-init=zero` | Desbordes de pila y de `memcpy`/`snprintf`; variables sin inicializar quedan a cero |
| Alias | `-fno-strict-aliasing` | Las mismas optimizaciones por alias que el kernel también desactiva |

Comprobado: un `INT_MAX + 1` compilado así termina con `Illegal instruction` (código 132) en lugar de devolver un número negativo. En la Fase 1 los guardianes detuvieron la compilación por dos errores reales: un valor `unsigned` 0xCDEF0123 que cambiaba de signo al guardarse en `int`, y un `chdir` sin comprobar.

Las opciones exactas usadas en cada compilación se guardan en `artifacts/guardianes.txt` (`evidencias/fase1-guardianes.txt`).

Los guardianes no hacen correcto un programa. Solo hacen que muchos errores se vean en lugar de esconderse. El kernel upstream conserva sus propias opciones de Kbuild y no se compila con estas.

## Fase 1: shell propia (VERDE)

- `src/init.c` es el PID 1. Monta `/dev`, `/proc`, `/sys`, `/tmp` y `/run`, informa cada paso como VERDE/AMARILLO/ROJO y supervisa una shell BusyBox: la relanza si muere y frena si muere en bucle.
- Las señales se atienden de forma síncrona con `sigwaitinfo`. `poweroff`, `reboot` y `halt` de BusyBox apagan mediante el init. Ctrl-Alt-Del llega como señal en lugar de reiniciar de golpe.
- BusyBox 1.36.1 viene del repositorio oficial de Arch. La compilación lo rechaza si no es estático.
- El kernel añade ACPI y PCI. Sin ACPI, `poweroff` solo detenía la CPU y QEMU no terminaba.
- `src/rootfs/etc/` aporta `os-release` y `profile`.
- `scripts/probar_arranque.py` arranca el kernel sin pantalla, escribe comandos por la consola serie y comprueba las respuestas. Resultado: 5/5 comandos VERDE y apagado VERDE (`evidencias/fase1-prueba-shell.log`). Kernel `bzImage-tiny64` de 2,3 MB, SHA-256 `d284a8cf…7462`.

Para verlo en una ventana:

```powershell
.\construir-kernel.ps1 -Accion Ver
```

## Hoja de ruta

1. **Fase 1: shell propia.** Hecha.
2. **Fase 2: kernel para tu PC.** Perfil para el Ryzen y la A320M: NVMe/SATA, USB, red, sonido, EFI y framebuffer. La RTX 3060 (Ampere) usará el módulo `nvidia-open`, que exige módulos de kernel activados.
3. **Fase 3: sistema base.** Disco con glibc, systemd y pacman sobre tu kernel.
4. **Fase 4: gráficos y juegos.** Mesa, Vulkan con el driver NVIDIA, Wayland, KDE Plasma, gamescope y Steam.
5. **Fase 5: identidad ULTRA.** Raíz inmutable A/B al estilo SteamOS y ajustes del kernel medidos frente a la referencia.
6. **Fase 6: el PC real.** Arranque dual sin tocar Windows, con recuperación.
