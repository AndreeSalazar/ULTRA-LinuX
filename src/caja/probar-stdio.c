/* SPDX-License-Identifier: GPL-2.0-only */
#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <signal.h>
#include <sys/socket.h>
#include <sys/wait.h>
#include <unistd.h>
#include "../guardian/ultra_guard.h"

static void test_socket_stdio(int target) {
    int sockets[2], capture[2];
    ULTRA_CHECK(socketpair(AF_UNIX, SOCK_STREAM | SOCK_CLOEXEC, 0, sockets) == 0 &&
                pipe2(capture, O_CLOEXEC) == 0, "TEST-STDIO", "preparacion fallo");
    pid_t child = fork();
    ULTRA_CHECK(child >= 0, "TEST-STDIO", "fork fallo");
    if (child == 0) {
        int supplied = dup2(sockets[0], target);
        if (supplied < 0) _exit(20);
        int errors = dup2(capture[1], STDERR_FILENO);
        if (errors < 0) { close(supplied); _exit(20); }
        close(sockets[0]); close(sockets[1]); close(capture[0]); close(capture[1]);
        char *const args[] = {"caja", "lector", "echo", "NO-DEBE-CONTINUAR", NULL};
        execv("/bin/caja", args);
        close(supplied); close(errors);
        _exit(21);
    }
    close(sockets[0]); close(capture[1]);
    char log[4096]; size_t used = 0;
    while (used < sizeof(log) - 1) {
        ssize_t n = read(capture[0], log + used, sizeof(log) - 1 - used);
        if (n < 0 && errno == EINTR) continue;
        if (n <= 0) break;
        used += (size_t)n;
    }
    log[used] = 0;
    int st;
    ULTRA_CHECK(waitpid(child, &st, 0) == child && WIFEXITED(st) && WEXITSTATUS(st) == 4 &&
                strstr(log, "CAJA-FD") != NULL,
                "TEST-STDIO", "socket heredado no rechazado antes de exec");
    char data[64];
    ssize_t got = recv(sockets[1], data, sizeof(data), MSG_DONTWAIT);
    ULTRA_CHECK(got == 0 || (got < 0 && (errno == EAGAIN || errno == EWOULDBLOCK)),
                "TEST-STDIO", "programa pudo escribir en socket heredado");
    close(capture[0]); close(sockets[1]);
}

int main(void) {
    test_socket_stdio(0);
    test_socket_stdio(1);
    int sockets[2];
    ULTRA_CHECK(socketpair(AF_UNIX, SOCK_STREAM | SOCK_CLOEXEC, 0, sockets) == 0, "TEST-STDIO", "socketpair fallo");
    pid_t child = fork();
    ULTRA_CHECK(child >= 0, "TEST-STDIO", "fork fallo");
    if (child == 0) {
        int fd = dup2(sockets[0], STDERR_FILENO);
        if (fd < 0) _exit(20);
        close(sockets[0]); close(sockets[1]);
        char *const args[] = {"caja", "lector", "echo", "NO-DEBE-CONTINUAR", NULL};
        execv("/bin/caja", args);
        close(fd); _exit(21);
    }
    close(sockets[0]);
    int st;
    ULTRA_CHECK(waitpid(child, &st, 0) == child && WIFEXITED(st) && WEXITSTATUS(st) == 4,
                "TEST-STDIO", "socket stderr no rechazado");
    char data;
    ULTRA_CHECK(recv(sockets[1], &data, 1, 0) == 0, "TEST-STDIO", "se envio diagnostico al socket stderr");
    close(sockets[1]);
    puts("STDIO-OK: sockets en stdin/stdout/stderr rechazados antes de exec; sin datos enviados");
    return 0;
}
