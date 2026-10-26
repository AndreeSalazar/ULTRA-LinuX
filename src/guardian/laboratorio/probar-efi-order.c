/* SPDX-License-Identifier: GPL-2.0-only */
#include <assert.h>
#include <stdio.h>
#include "../../efi-order.h"

int main(void) {
    uint8_t payload[514]; uint16_t out[256]; size_t count;
    for (size_t i=0;i<sizeof(payload);i++) payload[i]=(uint8_t)i;
    for (size_t size=0;size<=sizeof(payload);size++) {
        for (size_t cap=0;cap<=256;cap++) {
            for(size_t i=0;i<256;i++) out[i]=0xA55Au;
            count=999;
            bool ok=ultra_efi_order_decode(payload,size,out,cap,&count);
            bool expected=size%2==0 && size/2<=cap;
            assert(ok==expected);
            if(!ok) { assert(count==0); for(size_t i=0;i<256;i++) assert(out[i]==0xA55Au); }
            else {
                assert(count==size/2);
                for(size_t i=0;i<count;i++) assert(out[i]==(uint16_t)((uint16_t)payload[2*i] | ((uint16_t)payload[2*i+1]<<8)));
                for(size_t i=count;i<256;i++) assert(out[i]==0xA55Au);
            }
        }
    }
    count=999;assert(ultra_efi_order_decode(NULL,0,NULL,0,&count)&&count==0);
    count=999;assert(!ultra_efi_order_decode(NULL,2,out,256,&count)&&count==0);
    count=999;assert(!ultra_efi_order_decode(payload,2,NULL,256,&count)&&count==0);
    assert(!ultra_efi_order_decode(payload,2,out,256,NULL));
    puts("EFI-ORDER: 132355 size/capacity pairs + 4 null/empty contracts passed");
    return 0;
}
