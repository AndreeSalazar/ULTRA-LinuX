/* SPDX-License-Identifier: GPL-2.0-only */
/* Pieces of the ULTRA LinuX PID 1 that live in their own files. */
#ifndef ULTRA_INIT_H
#define ULTRA_INIT_H

#include <stdbool.h>

/* init.c: one VERDE, AMARILLO or ROJO line on the console. */
void report(const char *level, const char *code, const char *detail);

/* particiones.c: ULTRA-SISTEMA-A on /sistema and ULTRA-DATOS on /datos. True if /datos is writable. */
bool mount_partitions(void);
/* Leaves /datos clean before power-off: unmounted, or at least read-only. */
void unmount_data(void);

/* hardware.c: loads the PC's modules, guided by /datos/PC.configuración when it exists. */
void prepare_hardware(bool data);

#endif
