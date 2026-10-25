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

/* CASM 0.2 integers: exact, CPU widths, evaluated by the compiler with checked rules. */
#define CASM_EXPR_TOKENS 256
#define CASM_EXPR_DEPTH 32
#define CASM_RPN_TEXT 4096

enum { CASM_ASCII = 0, CASM_DECIMAL = 1 };
typedef struct {
    /* '#' + integer name for decimal renderings: one byte more than a source name. */
    char name[CASM_NAME_MAX + 2];
    unsigned char bytes[CASM_TEXT_MAX];
    size_t length;
    unsigned line;
    int kind;
    size_t integer;
} CasmConstant;
typedef struct {
    char name[CASM_NAME_MAX + 1];
    unsigned line;
    int type;            /* index into casm_int_types */
    int negative;        /* exact value = negative ? -magnitude : magnitude */
    uint64_t magnitude;
    char rpn[CASM_RPN_TEXT];   /* postfix form for the independent verifier */
    size_t rpn_length;
    unsigned rpn_tokens;
    int used;
    long decimal;        /* constant index of its decimal rendering, or -1 */
} CasmInteger;
typedef struct { size_t constant; unsigned line; } CasmCall;
typedef struct {
    CasmConstant constants[CASM_ITEMS_MAX];
    CasmCall calls[CASM_ITEMS_MAX];
    CasmInteger integers[CASM_ITEMS_MAX];
    size_t constant_count, call_count, integer_count;
    unsigned return_line;
} CasmProgram;

typedef struct { const char *name; unsigned bits; int is_signed; } CasmIntType;
extern const CasmIntType casm_int_types[8];

int casm_parse(const unsigned char *source, size_t length, CasmProgram *program);
int casm_emit(const CasmProgram *program, const char *hex_path, const char *ir_path);
#endif
