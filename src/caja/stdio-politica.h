/* SPDX-License-Identifier: GPL-2.0-only */
#ifndef CAJA_STDIO_POLITICA_H
#define CAJA_STDIO_POLITICA_H
#include <stdbool.h>
#include <sys/stat.h>
#include <sys/sysmacros.h>
#include <unistd.h>
/* Stdio is an explicit data grant, never an inherited network/device grant. */
static bool caja_stdio_fd_valid(int fd) {
    struct stat st;
    if (fstat(fd, &st) != 0) return false;
    if (S_ISREG(st.st_mode) || S_ISFIFO(st.st_mode)) return true;
    return S_ISCHR(st.st_mode) && (st.st_rdev == makedev(1, 3) || isatty(fd));
}
static bool caja_stdio_valid(void) {
    for (int fd = 0; fd < 3; fd++) if (!caja_stdio_fd_valid(fd)) return false;
    return true;
}
#endif
