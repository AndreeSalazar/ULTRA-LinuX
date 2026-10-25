#include "casm.h"
#include <stdio.h>
#include <stdlib.h>

int main(int argc, char **argv) {
    unsigned char *source;
    CasmProgram *program;
    FILE *file;
    size_t length;
    int ok;
    if (argc != 4) {
        fprintf(stderr, "Usage: casmc SOURCE OUTPUT.hex OUTPUT.ir.json\n");
        return 2;
    }
    file = fopen(argv[1], "rb");
    if (!file) { fprintf(stderr, "RED [HOST-READ] Cannot open source.\n"); return 1; }
    source = calloc(CASM_SOURCE_MAX + 1, 1);
    program = calloc(1, sizeof(*program));
    if (!source || !program) {
        fprintf(stderr, "RED [HOST-MEMORY] Allocation failed.\n");
        fclose(file); free(source); free(program); return 1;
    }
    length = fread(source, 1, CASM_SOURCE_MAX + 1, file);
    ok = !ferror(file) && length <= CASM_SOURCE_MAX;
    fclose(file);
    if (!ok) fprintf(stderr, "RED [SOURCE-SIZE] Source exceeds 65536 bytes or cannot be read.\n");
    if (ok) ok = casm_parse(source, length, program);
    if (ok) ok = casm_emit(program, argv[2], argv[3]);
    free(program); free(source);
    return ok ? 0 : 1;
}
