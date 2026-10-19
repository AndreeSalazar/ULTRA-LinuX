/* SPDX-License-Identifier: GPL-2.0-only */
#ifndef CAJA_IDENTIDAD_POLITICA_H
#define CAJA_IDENTIDAD_POLITICA_H
#include <errno.h>
#include <grp.h>
#include <linux/capability.h>
#include <stdbool.h>
#include <sys/fsuid.h>
#include <sys/prctl.h>
#include <sys/syscall.h>
#include <unistd.h>

/* Refuse an unknown capability ABI rather than silently checking only a prefix. */
static bool caja_capabilities_empty(void) {
    struct __user_cap_header_struct header = {_LINUX_CAPABILITY_VERSION_3, 0};
    struct __user_cap_data_struct data[2] = {{0}};
    if (syscall(SYS_capget, &header, data) != 0) return false;
    for (size_t i = 0; i < 2; i++)
        if (data[i].effective || data[i].permitted || data[i].inheritable) return false;
    for (int cap = 0; cap <= 64; cap++) {
        errno = 0;
        long bound = prctl(PR_CAPBSET_READ, cap, 0, 0, 0);
        if (bound < 0) return errno == EINVAL && cap > 0;
        if (cap == 64 || bound != 0 || prctl(PR_CAP_AMBIENT, PR_CAP_AMBIENT_IS_SET, cap, 0, 0) != 0)
            return false;
    }
    return false;
}

/* Called before installing seccomp; these inspection syscalls are not grants. */
static bool caja_identity_valid(uid_t uid, gid_t gid) {
    uid_t r, e, s;
    gid_t gr, ge, gs;
    if (uid == 0 || gid == 0 || getresuid(&r, &e, &s) != 0 || getresgid(&gr, &ge, &gs) != 0)
        return false;
    return r == uid && e == uid && s == uid && gr == gid && ge == gid && gs == gid &&
        (uid_t)setfsuid((uid_t)-1) == uid && (gid_t)setfsgid((gid_t)-1) == gid &&
        getgroups(0, NULL) == 0 && prctl(PR_GET_NO_NEW_PRIVS, 0, 0, 0, 0) == 1 &&
        caja_capabilities_empty();
}

static bool caja_drop_identity(uid_t uid, gid_t gid) {
    if (uid == 0 || gid == 0 || prctl(PR_SET_KEEPCAPS, 0, 0, 0, 0) != 0 ||
        prctl(PR_CAP_AMBIENT, PR_CAP_AMBIENT_CLEAR_ALL, 0, 0, 0) != 0) return false;
    bool end = false;
    for (int cap = 0; cap <= 64; cap++) {
        errno = 0;
        long present = prctl(PR_CAPBSET_READ, cap, 0, 0, 0);
        if (present < 0) { end = errno == EINVAL && cap > 0; break; }
        if (cap == 64 || prctl(PR_CAPBSET_DROP, cap, 0, 0, 0) != 0) return false;
    }
    if (!end || setgroups(0, NULL) != 0 || setresgid(gid, gid, gid) != 0 ||
        setresuid(uid, uid, uid) != 0) return false;
    /* UID changes do not automatically clear the inheritable set. */
    struct __user_cap_header_struct header = {_LINUX_CAPABILITY_VERSION_3, 0};
    struct __user_cap_data_struct empty[2] = {{0}};
    if (syscall(SYS_capset, &header, empty) != 0 ||
        prctl(PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) != 0) return false;
    return caja_identity_valid(uid, gid);
}
#endif
