#ifndef CASM_H
#define CASM_H
#include <stddef.h>
#include <stdint.h>

/* Host compiler limits, distinct from the target payload limit. */
#define CASM_SOURCE_MAX 65536
#define CASM_ITEMS_MAX 128
#define CASM_NAME_MAX 63
#define CASM_TEXT_MAX 4096
#define CASM_BODY_MAX 4072

typedef struct {
    char name[CASM_NAME_MAX + 1];
    unsigned char bytes[CASM_TEXT_MAX];
    size_t length;
    unsigned line;
} CasmConstant;
typedef struct { size_t constant; unsigned line; } CasmCall;
typedef struct {
    CasmConstant constants[CASM_ITEMS_MAX];
    CasmCall calls[CASM_ITEMS_MAX];
    size_t constant_count, call_count;
    unsigned return_line;
} CasmProgram;

int casm_parse(const unsigned char *source, size_t length, CasmProgram *program);
int casm_emit(const CasmProgram *program, const char *hex_path, const char *ir_path);
#endif
