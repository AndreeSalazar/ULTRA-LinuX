/* SPDX-License-Identifier: GPL-2.0-only */
#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <poll.h>
#include <stdio.h>
#include <string.h>
#include <sys/mount.h>
#include <sys/reboot.h>
#include <sys/sysinfo.h>
#include <sys/utsname.h>
#include <unistd.h>
static int serial_fd = -1;
static void emit(const char *s) {
    size_t n = strlen(s);
    (void)write(STDOUT_FILENO, s, n);
    if (serial_fd >= 0) (void)write(serial_fd, s, n);
}
static void execute(char *line) {
    char out[256];
    if (!strcmp(line, "help")) emit("help | version | mem | pid | echo TEXT | halt\n");
    else if (!strcmp(line, "version")) {
        struct utsname u;
        if (!uname(&u)) { snprintf(out, sizeof(out), "%s %s %s\n", u.sysname, u.release, u.machine); emit(out); }
    } else if (!strcmp(line, "mem")) {
        struct sysinfo s;
        if (!sysinfo(&s)) { snprintf(out, sizeof(out), "RAM total=%llu MiB free=%llu MiB\n", (unsigned long long)s.totalram*s.mem_unit/1048576, (unsigned long long)s.freeram*s.mem_unit/1048576); emit(out); }
    } else if (!strcmp(line, "pid")) { snprintf(out,sizeof(out),"init PID=%ld\n",(long)getpid());emit(out); }
    else if (!strncmp(line, "echo ", 5)) { emit(line+5); emit("\n"); }
    else if (!strcmp(line, "halt")) { emit("Apagando prueba QEMU.\n"); sync(); reboot(RB_POWER_OFF); }
    else if (*line) emit("Comando desconocido; usa help.\n");
}
int main(void) {
    (void)mount("devtmpfs", "/dev", "devtmpfs", 0, NULL);
    int fd=open("/dev/console",O_RDWR);
    if(fd>=0){for(int i=0;i<3;i++)(void)dup2(fd,i);if(fd>2)close(fd);}
    serial_fd=open("/dev/ttyS0",O_RDWR|O_NOCTTY|O_NONBLOCK);
    (void)mount("proc","/proc","proc",0,NULL);
    (void)mount("sysfs","/sys","sysfs",0,NULL);
    emit("\nLINUX RECONSTRUIDO: INIT C READY\nKernel oficial + init propio, compilados con GCC.\n");
    execute("version"); execute("pid"); execute("help");
    struct pollfd fds[2]={{STDIN_FILENO,POLLIN,0},{serial_fd,POLLIN,0}};
    char lines[2][256]={{0}};size_t lengths[2]={0};int overflow[2]={0};
    emit("metal> ");
    for(;;){
        int rc=poll(fds,serial_fd>=0?2:1,-1);
        if(rc<0){if(errno==EINTR)continue;emit("ROJO: poll fallo; init permanece vivo.\n");for(;;)pause();}
        for(int i=0;i<(serial_fd>=0?2:1);i++)if(fds[i].revents&POLLIN){
            char buf[64];ssize_t n=read(fds[i].fd,buf,sizeof(buf));
            for(ssize_t k=0;k<n;k++){
                unsigned char c=(unsigned char)buf[k];
                if(c=='\n'||c=='\r'){
                    if(overflow[i])emit("ROJO: linea demasiado larga; comando descartado.\n");
                    else{lines[i][lengths[i]]=0;execute(lines[i]);}
                    lengths[i]=0;overflow[i]=0;emit("metal> ");
                }else if(c==8||c==127){if(lengths[i])lengths[i]--;}
                else if(c>=32&&c<127){if(lengths[i]<sizeof(lines[i])-1)lines[i][lengths[i]++]=(char)c;else overflow[i]=1;}
            }
        }
    }
}
