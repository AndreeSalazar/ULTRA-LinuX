/* SPDX-License-Identifier: GPL-2.0-only */
/* Pieces of the ULTRA LinuX PID 1 that live in their own files. */
#ifndef ULTRA_INIT_H
#define ULTRA_INIT_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

/* init.c: one VERDE, AMARILLO or ROJO line on the console. */
void report(const char *level, const char *code, const char *detail);

/* particiones.c: ULTRA-SISTEMA-A on /sistema and ULTRA-DATOS on /datos. True if /datos is writable. */
bool mount_partitions(void);
/* Leaves /datos clean before power-off: unmounted, or at least read-only. */
void unmount_data(void);
/* BMO\ULTRA when ULTRA runs from files on the FAT32 partition: Windows reads it directly. NULL otherwise. */
const char *windows_dir(void);

/* hardware.c: loads the PC's modules, guided by /datos/PC.configuración when it exists. */
void prepare_hardware(bool data);
/* What this boot measured: ms up to the hardware stage and the previous boot of the same kernel (-1 if none). */
void boot_measure(int64_t *now_ms, int64_t *before_ms, size_t *modules, size_t *loaded);

/* init.c: text for the screen only (tty1), never the serial port of the tests. */
void screen_write(const char *s);
bool screen_available(void);

/* tarjeta.c: the boot card, after the hardware stage and before the shells. */
void show_card(void);

#endif
