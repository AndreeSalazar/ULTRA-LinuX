/* SPDX-License-Identifier: GPL-2.0-only */
/* Compiled grant list. Measurement proposes changes; only review grants them. */
#ifndef CAJA_SYSCALLS_POLITICA_H
#define CAJA_SYSCALLS_POLITICA_H
#define CAJA_SYSCALLS(X) \
    X(read, "leer descriptores; Landlock limita nuevas aperturas") \
    X(write, "salida estandar y archivos concedidos") \
    X(writev, "salida por vectores") \
    X(open, "aperturas heredadas BusyBox; Landlock comprueba rutas") \
    X(openat, "aperturas relativas; Landlock comprueba rutas") \
    X(close, "cerrar descriptores") \
    X(stat, "metadatos usados por BusyBox") \
    X(lstat, "metadatos sin seguir enlace final") \
    X(fstat, "metadatos del descriptor") \
    X(newfstatat, "metadatos relativos usados por libc") \
    X(lseek, "posicion de archivos") \
    X(mmap, "memoria de datos; PROT_EXEC prohibido por el filtro") \
    X(munmap, "liberar mapas de memoria") \
    X(mprotect, "permisos de datos; PROT_EXEC prohibido por el filtro") \
    X(brk, "heap del programa") \
    X(rt_sigaction, "instalar manejadores") \
    X(rt_sigprocmask, "mascara de senales") \
    X(rt_sigreturn, "regreso de manejadores") \
    X(ioctl, "solo siete comandos de terminal/consulta declarados") \
    X(fcntl, "solo seis comandos de flags y duplicacion declarados") \
    X(dup2, "redirecciones de la shell") \
    X(getdents64, "listar directorios permitidos") \
    X(execve, "ejecutar binarios en rutas concedidas") \
    X(arch_prctl, "solo SET_FS, GET_FS y GET_GS; sin desactivar CET") \
    X(set_tid_address, "inicializacion de libc") \
    X(prctl, "cuatro consultas y SET_NAME del propio proceso, medido en BusyBox") \
    X(exit, "terminar hilo") \
    X(exit_group, "terminar programa") \
    X(getpid, "identidad del proceso") \
    X(getppid, "identidad del padre") \
    X(getuid, "consulta de usuario") \
    X(getgid, "consulta de grupo") \
    X(geteuid, "consulta de usuario efectivo") \
    X(getegid, "consulta de grupo efectivo") \
    X(setuid, "BusyBox confirma identidad; solo UID 65534 autorizado") \
    X(setgid, "BusyBox confirma identidad; solo GID 65534 autorizado") \
    X(uname, "consulta de nombre UTS privado") \
    X(fork, "subprocesos heredando filtros") \
    X(vfork, "subprocesos heredando filtros") \
    X(wait4, "recoger subprocesos") \
    X(nanosleep, "esperas BusyBox") \
    X(clock_nanosleep, "esperas relativas o absolutas") \
    X(sendfile, "copias entre descriptores concedidos") \
    X(getcwd, "directorio actual") \
    X(chdir, "cambiar directorio; no concede lectura") \
    X(getgroups, "consulta de grupos con id")
#endif
