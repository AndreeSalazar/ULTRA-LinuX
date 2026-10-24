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

/* init.c: reboot cleanly (data unmounted) before the shells start. Returns only on failure. */
void restart_now(void);

/* init.c: text for the screen only (tty1), never the serial port of the tests. */
void screen_write(const char *s);
bool screen_available(void);

/* tarjeta.c: the boot card, after the hardware stage and before the shells. */
void show_card(void);

/* sistema.c: Arch's base system as a root at /arch (EROFS + a layer in memory); released before unmounting. */
void prepare_system(bool data);
void release_system(void);

/* logo.c: the ULTRA logo in the top-right corner of the screen, through /dev/fb0. */
void show_logo(void);
void show_intro(void);

/* particiones.c: the BMO partition's device (/dev/sda2) when ULTRA runs from it, else NULL. */
const char *bmo_device(void);

/* efi.c: firmware boot entries through efivarfs. Numbers are Boot####, -1 on failure. */
bool efi_ready(void);
int efi_find_entry(const char *devpath, const char *file);
int efi_create_entry(const char *devpath, const char *file, const char *title);
bool efi_delete_entry(int num);
bool efi_set_boot_next(int num);
bool efi_boot_next_pending(void);
bool efi_clear_boot_next(void);

/* perfil.c: the generic kernel prepares the PC's own version, with a way back. */
void choose_profile(bool data);
/* One short coloured line for the boot card: where the PC's own version stands. */
const char *profile_status(void);

#endif
