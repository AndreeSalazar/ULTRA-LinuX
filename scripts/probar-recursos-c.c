/* SPDX-License-Identifier: GPL-2.0-only */
#define ULTRA_RECURSOS_TEST
#include "../src/recursos.c"
#include <assert.h>
static unsigned fail_at, allocations;
void *__real_malloc(size_t size);
void *__real_calloc(size_t count, size_t size);
void *__wrap_malloc(size_t size) {
    if (fail_at && ++allocations==fail_at) return NULL;
    return __real_malloc(size);
}
void *__wrap_calloc(size_t count, size_t size) {
    if (fail_at && ++allocations==fail_at) return NULL;
    return __real_calloc(count, size);
}

static void check(const char *data, size_t size, bool memory, const char *expected) {
    FILE *input=tmpfile(), *out=tmpfile();
    assert(input && out);
    assert(fwrite(data, 1, size, input)==size);
    rewind(input);
    bool ok=snapshot(input, out, memory);
    assert(ok==(*expected!=0));
    rewind(out);
    char result[1024];
    size_t n=fread(result, 1, sizeof(result)-1, out);
    result[n]=0;
    assert(!strcmp(result, expected));
    fclose(input); fclose(out);
}
int main(void) {
    const char *valid="MemTotal: 8192 kB\nSwapTotal: 0 kB\nHugePages_Total: 0\n";
    check(valid, strlen(valid), true, "MEM MemTotal: 8192 kB\nMEM SwapTotal: 0 kB\nMEM HugePages_Total: 0 \n");
    valid="pgfault 18446744073709551615\npgmajfault 0";
    check(valid, strlen(valid), false, "VM pgfault=18446744073709551615\nVM pgmajfault=0\n");
    const char *invalid="pgfault 18446744073709551616\npgfault -1\npgfault 3a\npgfault 1 extra\nother 3\n";
    check(invalid, strlen(invalid), false, "");
    invalid="MemTotal: 1 MB\nMemTotal: 1 kB extra\nMemTotal: -2 kB\n";
    check(invalid, strlen(invalid), true, "");
    check("pgfault 1\0ignored", 17, false, "");
    check("", 0, false, "");
    FILE *out=tmpfile(); assert(out);
    assert(!snapshot(NULL, out, false)); fclose(out);
    char *large=malloc(SNAPSHOT_MAX+1u); assert(large);
    /* Exercise every placement across a chunk boundary, including long lines. */
    for (size_t offset=0; offset<2048; ++offset) {
        memset(large, 'x', offset); large[offset]='\n';
        const char tail[]="pgfault 7\npgmajfault 9";
        memcpy(large+offset+1, tail, sizeof(tail)-1);
        check(large, offset+sizeof(tail), false, "VM pgfault=7\nVM pgmajfault=9\n");
    }
    memset(large, 'x', SNAPSHOT_MAX+1u);
    memcpy(large, "pgfault 7\n", 10);
    check(large, SNAPSHOT_MAX+1u, false, "");
    check(large, SNAPSHOT_MAX, false, "VM pgfault=7\n");
    free(large);
    for (unsigned failure=1; failure<=5; ++failure) {
        FILE *input=tmpfile(); out=tmpfile(); assert(input && out);
        assert(fputs("pgfault 7\n", input)>=0); rewind(input);
        allocations=0; fail_at=failure;
        bool ok=snapshot(input, out, false);
        fail_at=0;
        assert(!ok); assert(ftell(out)==0);
        fclose(input); fclose(out);
    }
    puts("RECURSOS-C-OK: 2048 fronteras, cinco fallos de asignacion, cero, UINT64_MAX, overflow, NUL, limites y ausencia");
    return 0;
}
