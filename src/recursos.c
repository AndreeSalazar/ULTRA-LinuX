/* SPDX-License-Identifier: GPL-2.0-only */
#define _POSIX_C_SOURCE 200809L
/* Fixed-path observer, serialized local loans; no privileged I/O or tuning. */
#include "guardian/produccion/ultra_custodia.h"
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#define SNAPSHOT_MAX 65536u
#define LINE_MAXIMUM 255u

static bool selected(const char *key, bool memory) {
    static const char *const mem[] = {"MemTotal:", "MemAvailable:", "AnonPages:",
        "Cached:", "Buffers:", "Shmem:", "Slab:", "SReclaimable:", "SUnreclaim:",
        "Dirty:", "Writeback:", "PageTables:", "KernelStack:", "Unevictable:",
        "Mlocked:", "SwapTotal:", "SwapFree:", "SwapCached:", "AnonHugePages:",
        "HugePages_Total:", "HugePages_Free:", "Hugepagesize:"};
    static const char *const vm[] = {"pgfault", "pgmajfault", "pswpin", "pswpout",
        "compact_stall", "compact_fail", "compact_success", "oom_kill",
        "thp_fault_alloc", "thp_fault_fallback"};
    static const char *const prefix[] = {"pgscan_", "pgsteal_", "allocstall", "workingset_refault_"};
    if (memory) {
        for (size_t i=0; i<sizeof(mem)/sizeof(mem[0]); ++i)
            if (!strcmp(key, mem[i])) return true;
    } else {
        for (size_t i=0; i<sizeof(vm)/sizeof(vm[0]); ++i)
            if (!strcmp(key, vm[i])) return true;
        for (size_t i=0; i<sizeof(prefix)/sizeof(prefix[0]); ++i)
            if (!strncmp(key, prefix[i], strlen(prefix[i]))) return true;
    }
    return false;
}

/* Parse only unsigned decimal counters; reject overflow and extra tokens. */
static bool counter_line(FILE *out, char *line, bool memory) {
    char *save = NULL;
    char *key = strtok_r(line, " \t\r", &save);
    char *value = strtok_r(NULL, " \t\r", &save);
    char *unit = strtok_r(NULL, " \t\r", &save);
    if (!key || !value || !selected(key, memory)) return false;
    uint64_t number = 0;
    for (const char *p=value; *p; ++p) {
        if (*p<'0' || *p>'9') return false;
        unsigned digit = (unsigned)(*p-'0');
        if (number > (UINT64_MAX-digit)/10u) return false;
        number = number*10u+digit;
    }
    if (!*value || (unit && (!memory || strcmp(unit, "kB"))) ||
        strtok_r(NULL, " \t\r", &save)) return false;
    if (memory) fprintf(out, "MEM %s %s %s\n", key, value, unit ? unit : "");
    else fprintf(out, "VM %s=%s\n", key, value);
    return true;
}

/* The capture is discarded after lending. Only bounded copied chunks escape.
 * Oversize/read/allocation failures produce NA, never a truncated snapshot. */
static bool snapshot(FILE *input, FILE *out, bool memory) {
    if (!input) return false;
    unsigned char *capture = malloc(SNAPSHOT_MAX+1u);
    if (!capture) return false;
    size_t n = fread(capture, 1, SNAPSHOT_MAX+1u, input);
    if (!n || n>SNAPSHOT_MAX || ferror(input) || memchr(capture, 0, n)) {
        free(capture); return false;
    }
    struct ultra_custodia *c = uc_create(SNAPSHOT_MAX, 1);
    if (!c) { free(capture); return false; }
    uc_token loan = 0;
    enum uc_status status = uc_lend(c, 1, capture, n, &loan);
    free(capture);
    if (status != UC_OK) {
        if (uc_destroy(c) != UC_OK) abort();
        return false;
    }
    unsigned char chunk[1024];
    char line[LINE_MAXIMUM+1u];
    size_t used=0;
    bool overflow=false, emitted=false;
    for (size_t off=0; off<n;) {
        size_t count=n-off < sizeof(chunk) ? n-off : sizeof(chunk);
        uc_token ticket=0;
        if (uc_begin(c, 1, loan, off, count, &ticket) != UC_OK) abort();
        if (uc_complete(c, 1, ticket, chunk, sizeof(chunk)) != UC_OK) abort();
        for (size_t i=0; i<count; ++i) {
            if (chunk[i]=='\n') {
                if (!overflow) { line[used]=0; emitted=counter_line(out, line, memory)||emitted; }
                used=0; overflow=false;
            } else if (used<LINE_MAXIMUM && !overflow) line[used++]=(char)chunk[i];
            else overflow=true;
        }
        off+=count;
    }
    if (used && !overflow) { line[used]=0; emitted=counter_line(out, line, memory)||emitted; }
    if (uc_revoke(c, 1, loan) != UC_OK || uc_destroy(c) != UC_OK) abort();
    return emitted;
}

#ifndef ULTRA_RECURSOS_TEST
int main(int argc, char **argv) {
    (void)argv;
    if (argc != 1) return 2;
    static const char *const paths[] = {"/proc/meminfo", "/proc/vmstat"};
    for (size_t i=0; i<2; ++i) {
        FILE *input = fopen(paths[i], "re");
        if (!snapshot(input, stdout, i==0)) puts(i==0 ? "MEM NA" : "VM NA");
        if (input) fclose(input);
    }
    return ferror(stdout) ? 1 : 0;
}
#endif
