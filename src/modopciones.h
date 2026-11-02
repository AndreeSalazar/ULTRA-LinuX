/* SPDX-License-Identifier: GPL-2.0-only */
/*
 * Module options gathered the way kmod's modprobe does, for PID 1's
 * finit_module path:
 *   1. "options <module> <args...>" lines from modprobe.d text, in order;
 *   2. "<module>.<arg>" tokens from the kernel command line (they win, last).
 * '-' and '_' name the same module. Only "options" is honoured: install,
 * remove, softdep and blacklist lines are ignored, so nothing is executed.
 * Arguments are plain tokens of [A-Za-z0-9_.,:/=+-]; anything else (quotes,
 * shell characters) is skipped. Pure C, no I/O: tested on the host.
 */
#ifndef ULTRA_MODOPCIONES_H
#define ULTRA_MODOPCIONES_H

#include <stdbool.h>
#include <stddef.h>
#include <string.h>

static bool ultra_mod_char(char c) {
    return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9') ||
           c == '_' || c == '-' || c == '.' || c == ',' || c == ':' || c == '/' || c == '=' || c == '+';
}

/* Same module name, treating '-' and '_' alike; a is not NUL-terminated. */
static bool ultra_mod_same(const char *a, size_t alen, const char *b) {
    size_t i = 0;
    for (; i < alen && b[i]; i++) {
        char x = (char)(a[i] == '-' ? '_' : a[i]), y = (char)(b[i] == '-' ? '_' : b[i]);
        if (x != y) return false;
    }
    return i == alen && b[i] == 0;
}

/* Append one token (len bytes) and a separating space; false if it does not fit. */
static bool ultra_mod_append(char *out, size_t cap, const char *tok, size_t len) {
    size_t used = strlen(out);
    if (used + (used ? 1 : 0) + len + 1 > cap) return false;
    if (used) out[used++] = ' ';
    memcpy(out + used, tok, len);
    out[used + len] = 0;
    return true;
}

static bool ultra_mod_valid(const char *tok, size_t len) {
    if (len == 0 || tok[0] == '=') return false;
    for (size_t i = 0; i < len; i++) if (!ultra_mod_char(tok[i])) return false;
    return true;
}

static const char *ultra_mod_skip_space(const char *p, const char *end) {
    while (p < end && (*p == ' ' || *p == '\t')) p++;
    return p;
}

static const char *ultra_mod_token(const char *p, const char *end, size_t *len) {
    const char *start = p;
    while (p < end && *p != ' ' && *p != '\t') p++;
    *len = (size_t)(p - start);
    return start;
}

/* Options for name from modprobe.d text. Returns false only on overflow. */
static bool ultra_mod_from_conf(const char *text, const char *name, char *out, size_t cap) {
    for (const char *line = text; line && *line;) {
        const char *nl = strchr(line, '\n');
        const char *end = nl ? nl : line + strlen(line);
        const char *hash = memchr(line, '#', (size_t)(end - line));
        if (hash) end = hash;
        size_t len;
        const char *p = ultra_mod_skip_space(line, end);
        const char *word = ultra_mod_token(p, end, &len);
        if (len == 7 && !memcmp(word, "options", 7)) {
            p = ultra_mod_skip_space(word + len, end);
            const char *mod = ultra_mod_token(p, end, &len);
            if (len && ultra_mod_same(mod, len, name)) {
                for (p = ultra_mod_skip_space(mod + len, end); p < end; p = ultra_mod_skip_space(p, end)) {
                    const char *tok = ultra_mod_token(p, end, &len);
                    p = tok + len;
                    if (ultra_mod_valid(tok, len) && !ultra_mod_append(out, cap, tok, len)) return false;
                }
            }
        }
        line = nl ? nl + 1 : NULL;
    }
    return true;
}

/* "<name>.<arg>" tokens from the kernel command line. False only on overflow. */
static bool ultra_mod_from_cmdline(const char *cmdline, const char *name, char *out, size_t cap) {
    const char *end = cmdline + strlen(cmdline);
    for (const char *p = cmdline; p < end;) {
        while (p < end && (*p == ' ' || *p == '\t' || *p == '\n')) p++;
        const char *tok = p;
        while (p < end && *p != ' ' && *p != '\t' && *p != '\n') p++;
        size_t len = (size_t)(p - tok);
        const char *dot = memchr(tok, '.', len);
        const char *eq = memchr(tok, '=', len);
        /* The module part ends at the first '.', which must come before any '='. */
        if (!dot || (eq && eq < dot) || dot == tok) continue;
        if (!ultra_mod_same(tok, (size_t)(dot - tok), name)) continue;
        const char *arg = dot + 1;
        size_t alen = len - (size_t)(arg - tok);
        if (ultra_mod_valid(arg, alen) && !ultra_mod_append(out, cap, arg, alen)) return false;
    }
    return true;
}

#endif
