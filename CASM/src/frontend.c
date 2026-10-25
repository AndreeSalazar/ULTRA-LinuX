#include "casm.h"
#include <stdio.h>
#include <string.h>

const CasmIntType casm_int_types[8] = {
    {"uint8_t", 8, 0}, {"uint16_t", 16, 0}, {"uint32_t", 32, 0}, {"uint64_t", 64, 0},
    {"int8_t", 8, 1}, {"int16_t", 16, 1}, {"int32_t", 32, 1}, {"int64_t", 64, 1},
};

enum { TOK_EOF = 256, TOK_NAME, TOK_STRING, TOK_NUMBER };
typedef struct {
    const unsigned char *source;
    size_t length, pos;
    unsigned line, token_line;
    int token, failed;
    char name[CASM_NAME_MAX + 1];
    unsigned char text[CASM_TEXT_MAX];
    size_t text_length;
} Parser;

static int fail(Parser *p, const char *rule, const char *reason) {
    if (!p->failed) fprintf(stderr, "RED [%s] line %u: %s\n", rule, p->token_line, reason);
    p->failed = 1;
    return 0;
}
static int letter(unsigned char c) {
    return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || c == '_';
}
static int digit(unsigned char c) { return c >= '0' && c <= '9'; }
static int append(Parser *p, unsigned char c) {
    if (p->text_length >= CASM_TEXT_MAX - 1) return fail(p, "STRING-SIZE", "String exceeds the bounded literal capacity.");
    p->text[p->text_length++] = c;
    return 1;
}
static int next(Parser *p) {
    unsigned char c;
    size_t start, count;
    if (p->failed) return 0;
    for (;;) {
        if (p->pos >= p->length) { p->token_line = p->line; p->token = TOK_EOF; return 1; }
        c = p->source[p->pos];
        p->token_line = p->line;
        if (c == ' ' || c == '\t' || c == '\r' || c == '\n') {
            if (c == '\n') p->line++;
            p->pos++; continue;
        }
        if (c == '/' && p->pos + 1 < p->length && p->source[p->pos + 1] == '/') {
            p->pos += 2;
            while (p->pos < p->length && p->source[p->pos] != '\n') p->pos++;
            continue;
        }
        if (c == '/' && p->pos + 1 < p->length && p->source[p->pos + 1] == '*') {
            p->pos += 2;
            while (p->pos + 1 < p->length && !(p->source[p->pos] == '*' && p->source[p->pos + 1] == '/')) {
                if (p->source[p->pos++] == '\n') p->line++;
            }
            if (p->pos + 1 >= p->length) return fail(p, "COMMENT-END", "Unterminated block comment.");
            p->pos += 2; continue;
        }
        break;
    }
    c = p->source[p->pos++];
    if (letter(c)) {
        start = p->pos - 1;
        while (p->pos < p->length && (letter(p->source[p->pos]) || digit(p->source[p->pos]))) p->pos++;
        count = p->pos - start;
        if (count > CASM_NAME_MAX) return fail(p, "NAME-SIZE", "Identifier exceeds 63 characters.");
        memcpy(p->name, p->source + start, count); p->name[count] = 0;
        p->token = TOK_NAME; return 1;
    }
    if (digit(c)) {
        /* The literal text is kept as written; integer_literal() reads it exactly. */
        start = p->pos - 1;
        while (p->pos < p->length && (digit(p->source[p->pos]) || letter(p->source[p->pos]))) p->pos++;
        count = p->pos - start;
        if (count >= CASM_TEXT_MAX) return fail(p, "INT-LITERAL", "Integer literal is too long.");
        memcpy(p->text, p->source + start, count); p->text[count] = 0; p->text_length = count;
        p->token = TOK_NUMBER; return 1;
    }
    if (c == '"') {
        p->text_length = 0;
        while (p->pos < p->length) {
            c = p->source[p->pos++];
            if (c == '"') {
                if (!append(p, 0)) return 0;
                p->token = TOK_STRING; return 1;
            }
            if (c == '\\') {
                if (p->pos >= p->length) return fail(p, "STRING-END", "Unterminated escape.");
                c = p->source[p->pos++];
                if (c == 'n') c = '\n';
                else if (c == 'r') c = '\r';
                else if (c != '\\' && c != '"') return fail(p, "STRING-ESCAPE", "Only \\n, \\r, \\\\ and \\\" escapes are supported.");
            } else if (c < 32 || c > 126) return fail(p, "STRING-ASCII", "Use printable ASCII and explicit CR/LF escapes.");
            if (!append(p, c)) return 0;
        }
        return fail(p, "STRING-END", "Unterminated string literal.");
    }
    if (c == 0 || c > 127) return fail(p, "SOURCE-ASCII", "NUL and non-ASCII source bytes are not supported.");
    if (strchr("(){}[]=;+-*/%", c)) { p->token = c; return 1; }
    return fail(p, "TOKEN-UNSUPPORTED", "Token is outside the implemented C subset; nothing will be emitted.");
}
static int expect(Parser *p, int token) {
    if (p->failed) return 0;
    if (p->token != token) return fail(p, "SYNTAX", "Unexpected token in the supported grammar.");
    return next(p);
}
static int is_name(Parser *p, const char *name) { return p->token == TOK_NAME && !strcmp(p->name, name); }
static int word(Parser *p, const char *name) {
    if (!is_name(p, name)) return fail(p, "C-SUBSET", "Expected a supported declaration or statement; see LANGUAGE.md.");
    return next(p);
}
static int reserved(const char *name) {
    static const char *names[] = {"auto","break","case","char","const","continue","default","do","double","else","enum","extern","float","for","goto","if","inline","int","long","register","restrict","return","short","signed","sizeof","static","struct","switch","typedef","union","unsigned","void","volatile","while","_Alignas","_Alignof","_Atomic","_Bool","_Complex","_Generic","_Imaginary","_Noreturn","_Static_assert","_Thread_local","metal_main","metal_puts","metal_put_int",
        "uint8_t","uint16_t","uint32_t","uint64_t","int8_t","int16_t","int32_t","int64_t"};
    size_t i;
    for (i = 0; i < sizeof(names)/sizeof(names[0]); i++) if (!strcmp(name, names[i])) return 1;
    return name[0] == '_';
}
/* ---------- CASM 0.2: exact integers ----------
 * Every value lives in the declared type. Each literal, reference and intermediate result
 * must be representable there; nothing wraps, nothing is promoted, nothing is silent.
 * Division truncates toward zero; x % y has the sign of x; MIN % -1 is exactly 0. */
typedef struct { int64_t s; uint64_t u; } Value;

static int type_index(const char *name) {
    int i;
    for (i = 0; i < 8; i++) if (!strcmp(name, casm_int_types[i].name)) return i;
    return -1;
}
static int fits_signed(int64_t v, unsigned bits) {
    int64_t lo, hi;
    if (bits == 64) return 1;
    hi = (int64_t)((UINT64_C(1) << (bits - 1)) - 1); lo = -hi - 1;
    return v >= lo && v <= hi;
}
static int fits_unsigned(uint64_t v, unsigned bits) { return bits == 64 || v <= ((UINT64_C(1) << bits) - 1); }
static int overflow(Parser *p, const CasmIntType *t, const char *what) {
    char reason[160];
    snprintf(reason, sizeof reason, "%s does not fit in %s: CASM never wraps silently.", what, t->name);
    return fail(p, "INT-OVERFLOW", reason);
}
static int rpn(Parser *p, CasmInteger *n, const char *token) {
    size_t length = strlen(token);
    if (n->rpn_tokens >= CASM_EXPR_TOKENS || n->rpn_length + length + 2 > CASM_RPN_TEXT)
        return fail(p, "EXPR-SIZE", "Expression exceeds 256 operations.");
    if (n->rpn_length) n->rpn[n->rpn_length++] = ' ';
    memcpy(n->rpn + n->rpn_length, token, length + 1);
    n->rpn_length += length; n->rpn_tokens++;
    return 1;
}
/* Exact magnitude of the current number token: decimal without leading zeros, or 0x hex. */
static int integer_literal(Parser *p, uint64_t *out) {
    const unsigned char *t = p->text;
    size_t i = 0, base = 10;
    uint64_t v = 0;
    if (t[0] == '0' && (t[1] == 'x' || t[1] == 'X')) { base = 16; i = 2; if (!t[2]) return fail(p, "INT-LITERAL", "Hexadecimal literal without digits."); }
    else if (t[0] == '0' && t[1]) return fail(p, "INT-LITERAL", "Leading zero: CASM has no octal literals (010 is not 8).");
    for (; t[i]; i++) {
        unsigned d;
        if (digit(t[i])) d = (unsigned)(t[i] - '0');
        else if (base == 16 && t[i] >= 'a' && t[i] <= 'f') d = (unsigned)(t[i] - 'a' + 10);
        else if (base == 16 && t[i] >= 'A' && t[i] <= 'F') d = (unsigned)(t[i] - 'A' + 10);
        else return fail(p, "INT-LITERAL", "Malformed integer literal (no suffixes; digits only).");
        if (__builtin_mul_overflow(v, (uint64_t)base, &v) || __builtin_add_overflow(v, (uint64_t)d, &v))
            return fail(p, "INT-LITERAL", "Integer literal exceeds 18446744073709551615.");
    }
    *out = v;
    return 1;
}
static int decimal_text(uint64_t magnitude, int negative, char *out) {
    char digits[24];
    int n = 0, k = 0;
    do { digits[n++] = (char)('0' + magnitude % 10); magnitude /= 10; } while (magnitude);
    if (negative) out[k++] = '-';
    while (n) out[k++] = digits[--n];
    out[k] = 0;
    return k;
}
static int expression(Parser *p, CasmProgram *program, CasmInteger *n, Value *v, int depth);

static int primary(Parser *p, CasmProgram *program, CasmInteger *n, Value *v, int depth) {
    const CasmIntType *t = &casm_int_types[n->type];
    size_t i;
    if (p->token == '(') {
        if (depth >= CASM_EXPR_DEPTH) return fail(p, "EXPR-DEPTH", "Expression nests deeper than 32 levels.");
        return next(p) && expression(p, program, n, v, depth + 1) && expect(p, ')');
    }
    if (p->token == TOK_NUMBER) {
        uint64_t m;
        char text[24];
        if (!integer_literal(p, &m)) return 0;
        decimal_text(m, 0, text);
        if (t->is_signed ? (m > (uint64_t)INT64_MAX || !fits_signed((int64_t)m, t->bits)) : !fits_unsigned(m, t->bits)) return overflow(p, t, "The literal");
        if (t->is_signed) v->s = (int64_t)m; else v->u = m;
        return rpn(p, n, text) && next(p);
    }
    if (p->token == TOK_NAME) {
        CasmInteger *r = 0;
        char token[CASM_NAME_MAX + 2];
        for (i = 0; i < program->integer_count; i++) if (!strcmp(program->integers[i].name, p->name)) r = &program->integers[i];
        for (i = 0; i < program->constant_count; i++) if (!strcmp(program->constants[i].name, p->name)) return fail(p, "TYPE-INT", "A string cannot take part in integer arithmetic.");
        if (!r) return fail(p, "NAME-UNDEFINED", "Integer must be declared before use.");
        r->used = 1;
        /* The referenced exact value must be representable in this declaration's type. */
        if (t->is_signed) {
            if (r->negative ? r->magnitude > (uint64_t)INT64_MAX + 1 : r->magnitude > (uint64_t)INT64_MAX) return overflow(p, t, r->name);
            v->s = r->negative ? (int64_t)(0 - r->magnitude) : (int64_t)r->magnitude;
            if (!fits_signed(v->s, t->bits)) return overflow(p, t, r->name);
        } else {
            if (r->negative && r->magnitude) return overflow(p, t, r->name);
            v->u = r->magnitude;
            if (!fits_unsigned(v->u, t->bits)) return overflow(p, t, r->name);
        }
        token[0] = '@'; memcpy(token + 1, r->name, strlen(r->name) + 1);
        return rpn(p, n, token) && next(p);
    }
    return fail(p, "SYNTAX", "Expected an integer literal, a declared integer or (.");
}
static int unary(Parser *p, CasmProgram *program, CasmInteger *n, Value *v, int depth) {
    const CasmIntType *t = &casm_int_types[n->type];
    if (p->token != '-') return primary(p, program, n, v, depth);
    if (depth >= CASM_EXPR_DEPTH) return fail(p, "EXPR-DEPTH", "Expression nests deeper than 32 levels.");
    if (!next(p)) return 0;
    if (p->token == TOK_NUMBER) {
        /* -literal is one exact value, so int64_t can hold -9223372036854775808. */
        uint64_t m;
        char text[24];
        if (!integer_literal(p, &m)) return 0;
        if (t->is_signed) {
            if (m > (uint64_t)INT64_MAX + 1) return overflow(p, t, "The literal");
            v->s = m == (uint64_t)INT64_MAX + 1 ? INT64_MIN : -(int64_t)m;
            if (!fits_signed(v->s, t->bits)) return overflow(p, t, "The literal");
        } else {
            if (m) return overflow(p, t, "A negative literal");
            v->u = 0;
        }
        decimal_text(m, m != 0, text);
        return rpn(p, n, text) && next(p);
    }
    if (!unary(p, program, n, v, depth + 1)) return 0;
    if (t->is_signed) { if (v->s == INT64_MIN || !fits_signed(-v->s, t->bits)) return overflow(p, t, "The negation"); v->s = -v->s; }
    else { if (v->u) return overflow(p, t, "The negation"); }
    return rpn(p, n, "neg");
}
static int term(Parser *p, CasmProgram *program, CasmInteger *n, Value *v, int depth) {
    const CasmIntType *t = &casm_int_types[n->type];
    if (!unary(p, program, n, v, depth)) return 0;
    while (p->token == '*' || p->token == '/' || p->token == '%') {
        int op = p->token;
        Value b;
        if (!next(p) || !unary(p, program, n, &b, depth)) return 0;
        if (op != '*' && (t->is_signed ? b.s == 0 : b.u == 0)) return fail(p, "INT-DIV-ZERO", "Division or remainder by zero.");
        if (t->is_signed) {
            if (op == '*') { if (__builtin_mul_overflow(v->s, b.s, &v->s) || !fits_signed(v->s, t->bits)) return overflow(p, t, "The product"); }
            else if (op == '/') { if (v->s == INT64_MIN && b.s == -1) return overflow(p, t, "The quotient"); v->s /= b.s; if (!fits_signed(v->s, t->bits)) return overflow(p, t, "The quotient"); }
            else v->s = b.s == -1 ? 0 : v->s % b.s;
        } else {
            if (op == '*') { if (__builtin_mul_overflow(v->u, b.u, &v->u) || !fits_unsigned(v->u, t->bits)) return overflow(p, t, "The product"); }
            else if (op == '/') v->u /= b.u;
            else v->u %= b.u;
        }
        if (!rpn(p, n, op == '*' ? "*" : op == '/' ? "/" : "%")) return 0;
    }
    return 1;
}
static int expression(Parser *p, CasmProgram *program, CasmInteger *n, Value *v, int depth) {
    const CasmIntType *t = &casm_int_types[n->type];
    if (!term(p, program, n, v, depth)) return 0;
    while (p->token == '+' || p->token == '-') {
        int op = p->token;
        Value b;
        if (!next(p) || !term(p, program, n, &b, depth)) return 0;
        if (t->is_signed) {
            if ((op == '+' ? __builtin_add_overflow(v->s, b.s, &v->s) : __builtin_sub_overflow(v->s, b.s, &v->s)) || !fits_signed(v->s, t->bits))
                return overflow(p, t, op == '+' ? "The sum" : "The difference");
        } else {
            if ((op == '+' ? __builtin_add_overflow(v->u, b.u, &v->u) : __builtin_sub_overflow(v->u, b.u, &v->u)) || !fits_unsigned(v->u, t->bits))
                return overflow(p, t, op == '+' ? "The sum" : "The difference");
        }
        if (!rpn(p, n, op == '+' ? "+" : "-")) return 0;
    }
    return 1;
}
static int name_taken(const CasmProgram *program, const char *name) {
    size_t i;
    for (i = 0; i < program->constant_count; i++) if (!strcmp(program->constants[i].name, name)) return 1;
    for (i = 0; i < program->integer_count; i++) if (!strcmp(program->integers[i].name, name)) return 1;
    return 0;
}
/* const <int type> name = expression; */
static int integer_declaration(Parser *p, CasmProgram *program, int type, unsigned line) {
    CasmInteger *n;
    Value v;
    if (program->integer_count >= CASM_ITEMS_MAX) return fail(p, "INTEGER-LIMIT", "Too many integer constants.");
    if (!next(p)) return 0;
    if (p->token != TOK_NAME) return fail(p, "DECL-NAME", "Expected an integer identifier.");
    if (reserved(p->name)) return fail(p, "NAME-RESERVED", "C keywords, underscore names and runtime names cannot be declared.");
    if (name_taken(program, p->name)) return fail(p, "NAME-DUPLICATE", "Identifier already declared in this scope.");
    n = &program->integers[program->integer_count];
    memset(n, 0, sizeof(*n));
    memcpy(n->name, p->name, strlen(p->name) + 1);
    n->line = line; n->type = type; n->decimal = -1;
    if (!next(p) || !expect(p, '=')) return 0;
    if (p->token == TOK_STRING) return fail(p, "TYPE-INT", "An integer requires an integer expression, not a string.");
    memset(&v, 0, sizeof v);
    if (!expression(p, program, n, &v, 0)) return 0;
    if (casm_int_types[type].is_signed) { n->negative = v.s < 0; n->magnitude = v.s < 0 ? 0 - (uint64_t)v.s : (uint64_t)v.s; }
    else { n->negative = 0; n->magnitude = v.u; }
    program->integer_count++;
    return expect(p, ';');
}
/* metal_put_int(name); prints the exact decimal value, like COBOL's DISPLAY. */
static int put_int(Parser *p, CasmProgram *program) {
    size_t i;
    unsigned line = p->token_line;
    CasmInteger *n = 0;
    if (program->call_count >= CASM_ITEMS_MAX) return fail(p, "CALL-LIMIT", "Too many service calls.");
    if (!next(p) || !expect(p, '(')) return 0;
    if (p->token != TOK_NAME) return fail(p, "TYPE-INT", "metal_put_int requires a declared integer.");
    for (i = 0; i < program->integer_count; i++) if (!strcmp(program->integers[i].name, p->name)) n = &program->integers[i];
    for (i = 0; i < program->constant_count; i++) if (program->constants[i].kind == CASM_ASCII && !strcmp(program->constants[i].name, p->name)) return fail(p, "TYPE-INT", "metal_put_int prints integers; use metal_puts for strings.");
    if (!n) return fail(p, "NAME-UNDEFINED", "Integer must be declared before use.");
    n->used = 1;
    if (n->decimal < 0) {
        /* Its decimal rendering is data, emitted once and shared by every call. */
        CasmConstant *c;
        char text[24];
        int length;
        if (program->constant_count >= CASM_ITEMS_MAX) return fail(p, "CONSTANT-LIMIT", "Too many constants.");
        c = &program->constants[program->constant_count];
        memset(c, 0, sizeof(*c));
        c->name[0] = '#'; memcpy(c->name + 1, n->name, strlen(n->name) + 1);
        length = decimal_text(n->magnitude, n->negative, text);
        memcpy(c->bytes, text, (size_t)length + 1); c->length = (size_t)length + 1;
        c->line = n->line; c->kind = CASM_DECIMAL; c->integer = (size_t)(n - program->integers);
        n->decimal = (long)program->constant_count++;
    }
    program->calls[program->call_count].constant = (size_t)n->decimal;
    program->calls[program->call_count++].line = line;
    return next(p) && expect(p, ')') && expect(p, ';');
}

static int declaration(Parser *p, CasmProgram *program) {
    CasmConstant *constant;
    unsigned line = p->token_line;
    if (!word(p,"const")) return 0;
    if (p->token == TOK_NAME && type_index(p->name) >= 0) return integer_declaration(p, program, type_index(p->name), line);
    if (program->constant_count >= CASM_ITEMS_MAX) return fail(p, "CONSTANT-LIMIT", "Too many constants.");
    if (!word(p,"char")) return 0;
    if (p->token == '*') return fail(p,"TOKEN-UNSUPPORTED","Pointers are not part of CASM: declare const char name[] = \"...\";");
    if (p->token != TOK_NAME) return fail(p,"DECL-NAME","Expected an array identifier.");
    if (reserved(p->name)) return fail(p,"NAME-RESERVED","C keywords, underscore names and runtime names cannot be declared.");
    if (name_taken(program, p->name)) return fail(p,"NAME-DUPLICATE","Identifier already declared in this scope.");
    constant = &program->constants[program->constant_count];
    memset(constant, 0, sizeof(*constant));
    memcpy(constant->name, p->name, strlen(p->name)+1); constant->line = line;
    if (!next(p) || !expect(p,'[') || !expect(p,']') || !expect(p,'=')) return 0;
    if (p->token != TOK_STRING) return fail(p,"TYPE-STRING","A const char array requires a string literal initializer.");
    memcpy(constant->bytes, p->text, p->text_length); constant->length=p->text_length;
    program->constant_count++;
    return next(p) && expect(p,';');
}
static int call(Parser *p, CasmProgram *program) {
    size_t i;
    unsigned line=p->token_line;
    if (program->call_count >= CASM_ITEMS_MAX) return fail(p,"CALL-LIMIT","Too many service calls.");
    if (!word(p,"metal_puts") || !expect(p,'(')) return 0;
    if (p->token != TOK_NAME) return fail(p,"TYPE-STRING","metal_puts requires a declared immutable char array.");
    for (i=0; i<program->integer_count; i++) if (!strcmp(p->name,program->integers[i].name)) return fail(p,"TYPE-STRING","metal_puts prints strings; use metal_put_int for integers.");
    for (i=0; i<program->constant_count; i++) if (program->constants[i].kind == CASM_ASCII && !strcmp(p->name,program->constants[i].name)) break;
    if (i == program->constant_count) return fail(p,"NAME-UNDEFINED","String must be declared before use.");
    program->calls[program->call_count].constant=i;
    program->calls[program->call_count++].line=line;
    return next(p) && expect(p,')') && expect(p,';');
}
int casm_parse(const unsigned char *source, size_t length, CasmProgram *program) {
    Parser p;
    size_t i;
    memset(&p,0,sizeof(p)); p.source=source; p.length=length; p.line=1; p.token_line=1;
    /* Even comments must not silently conceal unsupported byte encodings. */
    for (i=0;i<length;i++) if (!source[i] || source[i]>127) return fail(&p,"SOURCE-ASCII","Source must be ASCII without NUL bytes or a UTF-8 BOM.");
    if (!next(&p) || !word(&p,"void") || !word(&p,"metal_main") || !expect(&p,'(') || !word(&p,"void") || !expect(&p,')') || !expect(&p,'{')) return 0;
    while (!p.failed && p.token != '}' && p.token != TOK_EOF) {
        if (program->return_line) return fail(&p,"AFTER-RETURN","Statements after return are not permitted.");
        if (is_name(&p,"const")) { if (!declaration(&p,program)) return 0; }
        else if (is_name(&p,"metal_puts")) { if (!call(&p,program)) return 0; }
        else if (is_name(&p,"metal_put_int")) { if (!put_int(&p,program)) return 0; }
        else if (is_name(&p,"return")) {
            program->return_line=p.token_line;
            if (!next(&p) || !expect(&p,';')) return 0;
        } else return fail(&p,"C-SUBSET","Unsupported C construct; no fallback or ignored statements.");
    }
    if (!program->return_line) return fail(&p,"RETURN-REQUIRED","The entry must end with an explicit return;.");
    if (!expect(&p,'}')) return 0;
    if (p.token != TOK_EOF) return fail(&p,"TRAILING-SOURCE","Only one entry definition is supported.");
    printf("GREEN [SEMANTIC] Immutable strings, exact integers and service calls validated.\n");
    return 1;
}
