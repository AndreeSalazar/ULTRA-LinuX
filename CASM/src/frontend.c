#include "casm.h"
#include <stdio.h>
#include <string.h>

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
        while (p->pos < p->length && digit(p->source[p->pos])) p->pos++;
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
    if (strchr("(){}[]=;", c)) { p->token = c; return 1; }
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
    static const char *names[] = {"auto","break","case","char","const","continue","default","do","double","else","enum","extern","float","for","goto","if","inline","int","long","register","restrict","return","short","signed","sizeof","static","struct","switch","typedef","union","unsigned","void","volatile","while","_Alignas","_Alignof","_Atomic","_Bool","_Complex","_Generic","_Imaginary","_Noreturn","_Static_assert","_Thread_local","metal_main","metal_puts"};
    size_t i;
    for (i = 0; i < sizeof(names)/sizeof(names[0]); i++) if (!strcmp(name, names[i])) return 1;
    return name[0] == '_';
}
static int declaration(Parser *p, CasmProgram *program) {
    CasmConstant *constant;
    size_t i;
    unsigned line = p->token_line;
    if (program->constant_count >= CASM_ITEMS_MAX) return fail(p, "CONSTANT-LIMIT", "Too many constants.");
    if (!word(p,"const") || !word(p,"char")) return 0;
    if (p->token != TOK_NAME) return fail(p,"DECL-NAME","Expected an array identifier.");
    if (reserved(p->name)) return fail(p,"NAME-RESERVED","C keywords, underscore names and runtime names cannot be declared.");
    for (i=0; i<program->constant_count; i++) if (!strcmp(program->constants[i].name,p->name)) return fail(p,"NAME-DUPLICATE","Identifier already declared in this scope.");
    constant = &program->constants[program->constant_count];
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
    for (i=0; i<program->constant_count; i++) if (!strcmp(p->name,program->constants[i].name)) break;
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
        else if (is_name(&p,"return")) {
            program->return_line=p.token_line;
            if (!next(&p) || !expect(&p,';')) return 0;
        } else return fail(&p,"C-SUBSET","Unsupported C construct; no fallback or ignored statements.");
    }
    if (!program->return_line) return fail(&p,"RETURN-REQUIRED","The entry must end with an explicit return;.");
    if (!expect(&p,'}')) return 0;
    if (p.token != TOK_EOF) return fail(&p,"TRAILING-SOURCE","Only one entry definition is supported.");
    printf("GREEN [SEMANTIC] Immutable strings and service calls validated.\n");
    return 1;
}
