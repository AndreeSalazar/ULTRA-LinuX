/* SPDX-License-Identifier: GPL-2.0-only */
/* All EFI paths are redirected into this isolated test's working directory. */
#define _GNU_SOURCE
#include <assert.h>
#define EFIVARS "./efivars-prueba"
#include "../../efi.c"

void report(const char *level, const char *code, const char *detail) {
    (void)level; (void)code; (void)detail;
}
static void fixture(const char *path, size_t payload) {
    FILE *f=fopen(path,"wb"); assert(f);
    uint8_t attr[4]={7,0,0,0};assert(fwrite(attr,1,4,f)==4);
    for(size_t i=0;i<payload;i++) assert(fputc((int)(i%256),f)!=EOF);
    assert(fclose(f)==0);
}
int main(void) {
    assert(mkdir(EFIVARS,0700)==0);
    const char *path=EFIVARS "/BootOrder-" GLOBAL;
    uint16_t order[256];size_t count=999;
    assert(order_read(order,256,&count)&&count==0); /* ENOENT only */
    fixture(path,4);assert(order_read(order,256,&count)&&count==2);
    assert(order[0]==0x0100&&order[1]==0x0302);
    assert(!order_read(order,1,&count)&&count==0);
    fixture(path,3);assert(!order_read(order,256,&count)&&count==0);
    fixture(path,4097);assert(!order_read(order,256,&count)&&count==0);
    assert(unlink(path)==0);assert(mkdir(path,0700)==0);
    assert(!order_read(order,256,&count)&&count==0); /* EISDIR must not look empty. */
    const char *entry=EFIVARS "/Boot0010-" GLOBAL;fixture(entry,2);
    assert(!efi_delete_entry(16)&&access(entry,F_OK)==0); /* No deletion after bad order. */
    assert(unlink(entry)==0);assert(rmdir(path)==0);assert(rmdir(EFIVARS)==0);
    puts("EFI-IO: missing, valid, capacity, odd, oversized, read failure and deletion fail-closed passed");
    return 0;
}
