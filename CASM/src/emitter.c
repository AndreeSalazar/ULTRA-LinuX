#include "casm.h"
#include <stdio.h>
#include <string.h>

static void put32(unsigned char *out, uint32_t value) {
    size_t i; for(i=0;i<4;i++) out[i]=(unsigned char)(value>>(8*i));
}
int casm_emit(const CasmProgram *program, const char *hex_path, const char *ir_path) {
    unsigned char body[CASM_BODY_MAX];
    size_t offsets[CASM_ITEMS_MAX], size=program->call_count*11+1, i,j,at;
    FILE *hex,*ir;
    for(i=0;i<program->constant_count;i++) {
        if(program->constants[i].length>CASM_BODY_MAX-size) {
            fprintf(stderr,"RED [BODY-LIMIT] Code and data exceed 4072 bytes.\n"); return 0;
        }
        offsets[i]=size; size+=program->constants[i].length;
    }
    memset(body,0,sizeof(body));
    for(i=0;i<program->call_count;i++) {
        at=i*11;
        body[at]=0x48; body[at+1]=0x8d; body[at+2]=0x35;
        put32(body+at+3,(uint32_t)(offsets[program->calls[i].constant]-(at+7)));
        memcpy(body+at+7,"\x41\xff\x57\x10",4);
    }
    body[program->call_count*11]=0xc3;
    for(i=0;i<program->constant_count;i++) memcpy(body+offsets[i],program->constants[i].bytes,program->constants[i].length);
    /* Only analysis artifacts are emitted here. The independent verifier publishes .x86. */
    hex=fopen(hex_path,"wb");
    if(!hex) { fprintf(stderr,"RED [HOST-WRITE] Cannot create HEX output.\n"); return 0; }
    fprintf(hex,"; CASM body only: x86-64 / metal64. Verify before packaging.\n");
    for(i=0;i<size;i++) fprintf(hex,"%02X%c",body[i],(i%16==15 || i+1==size)?'\n':' ');
    if(ferror(hex)) { fclose(hex); return 0; }
    if(fclose(hex)) return 0;
    ir=fopen(ir_path,"wb");
    if(!ir) { fprintf(stderr,"RED [HOST-WRITE] Cannot create IR output.\n"); return 0; }
    fprintf(ir,"{\n\"Schema\":1,\"Target\":\"x86_64\",\"ABI\":\"metal64\",\"Constants\":[");
    for(i=0;i<program->constant_count;i++) {
        const CasmConstant *c=&program->constants[i];
        fprintf(ir,"%s{\"Name\":\"%s\",\"Line\":%u,\"Offset\":%zu,\"Hex\":\"",i?",":"",c->name,c->line,offsets[i]);
        for(j=0;j<c->length;j++) fprintf(ir,"%02X",c->bytes[j]);
        fprintf(ir,"\"}");
    }
    fprintf(ir,"],\n\"Calls\":[");
    for(i=0;i<program->call_count;i++) fprintf(ir,"%s{\"Constant\":%zu,\"Line\":%u,\"Offset\":%zu}",i?",":"",program->calls[i].constant,program->calls[i].line,i*11);
    fprintf(ir,"],\n\"Return\":{\"Line\":%u,\"Offset\":%zu},\"BodyBytes\":%zu\n}\n",program->return_line,program->call_count*11,size);
    if(ferror(ir)) { fclose(ir); return 0; }
    if(fclose(ir)) return 0;
    if(size>3500) printf("YELLOW [BODY-MARGIN] %zu of 4072 bytes occupied.\n",size);
    for(i=0;i<program->constant_count;i++) {
        int used=0;
        for(j=0;j<program->call_count;j++) if(program->calls[j].constant==i) used=1;
        if(!used) printf("YELLOW [UNUSED-CONSTANT] line %u: %s\n",program->constants[i].line,program->constants[i].name);
    }
    printf("GREEN [EMISSION] %zu body bytes; unverified HEX and IR written.\n",size);
    return 1;
}
