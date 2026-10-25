# CASM 0.1 language and C adoption policy

Status: executable foundation, deliberately bounded. C11 is a reference, not a
conformance claim. The public [N1570 draft](https://www.open-std.org/jtc1/sc22/wg14/www/docs/n1570.pdf)
separates environment, language and library; all need explicit decisions before
claiming complete C support.

## Implemented grammar

```text
translation-unit := "void" "metal_main" "(" "void" ")" "{" statement* "return" ";" "}"
statement        := declaration | service-call
declaration      := "const" "char" identifier "[" "]" "=" string ";"
service-call     := "metal_puts" "(" identifier ")" ";"
```

The complete source is parsed before emission. Whitespace and `//` / `/* */`
comments are accepted. Identifiers are ASCII letters/underscore followed by
letters/underscore/digits, up to 63 bytes; declarations reject all C11 keywords,
runtime names and names beginning with underscore. Comments are not nested.

Strings use printable ASCII plus `\n`, `\r`, `\\` and `\"` escapes; the compiler
adds the final zero. Embedded zero, adjacent concatenation, octal/hex/Unicode
escapes and raw newlines are not implemented. Source must be ASCII without NUL
or a UTF-8 BOM, at most 65536 bytes.

One function, one scope, at most 128 string constants and 128 calls are supported.
Names must be unique and declared before use. Only a declared immutable array
can be passed to `metal_puts`; literal call arguments are not supported yet.
An explicit final `return;` is required, with no later statements or definitions.

Arrays are lowered to the immutable data region. No addresses escape and arrays
cannot be modified or compared, so this restricted interface cannot observe a
storage difference during its one invocation. This is not general C automatic
storage lowering. `metal_puts` has an implicit built-in signature corresponding
to `void metal_puts(const char *text)`; declarations and headers are not parsed.
It outputs bytes until NUL and does not append a newline.

The body must be 1..4072 bytes (maximum packet 4096). Code comes first: 11 bytes
per call and one RET, followed by every string including unused ones. Unused
strings warn but are retained. No optimizations are implemented.

## C coverage and policy

Every expansion needs semantic rules, rejection diagnostics, encoding rules and
tests. Reviewing all C means making decisions explicit; unsupported features do
not become implemented by appearing in this table.

| C area / N1570 section | CASM 0.1 | Future decisions |
|---|---|---|
| Environment / translation phases, §5 | Bounded ASCII source | Preprocessing phases, encodings, translation units, freestanding environment |
| Scope, linkage, lifetime, §6.2 | One scope, immutable strings | Blocks, storage durations, lifetime, linkage, aliases |
| Types, representations, §6.2.5–6.2.8 | const char arrays | Integer widths, alignment, qualifiers, floats/complex types |
| Conversions, §6.3 | No arithmetic conversions | Promotions, narrowing, signedness, checked conversion policy |
| Lexical elements, §6.4 | Names, restricted strings, comments | Constants, full escapes, tokens, translation-phase rules |
| Expressions, §6.5–6.6 | One built-in service call | Arithmetic, sequencing, overflow, comparisons, addresses, constant evaluation |
| Declarations, §6.7 | One array form | Scalars, structs/unions, enum, typedef, full declarators/initialization |
| Statements, §6.8 | Calls and final return | Blocks, if/switch, loops, break/continue, goto, reachability |
| Functions/definitions, §6.9 | One void entry | Parameters, recursion, stack limits, ABI, external calls, separate compilation |
| Preprocessor, §6.10 | Rejected | Includes, macros, conditionals, diagnostic/source mapping |
| Standard library, §7 | Not implemented; metal_puts is project-specific | Host library versus bare-metal services; no implied libc |
| Rust-inspired ownership | Source pointers unavailable | Regions, borrowing, escape/lifetime rules, explicit unsafe boundary |
| Undefined/unspecified behavior, Annex J | Most relevant operations unavailable | Preserve C behavior or document each deliberate deviation |
| Threads, atomics, volatile, MMIO | Rejected | Memory model, interrupts, hardware access, privileged operations |

CASM cannot preserve acceptance of every unrestricted C program and also reject
all unsafe C operations. Restricted profiles must name their restrictions. No
alternate C-compatible or unsafe mode exists in 0.1. Do not silently redefine
overflow or pointer semantics.

## Next implementation boundary

The next coherent slice is integer types, scalar locals and checked arithmetic,
with definite-initialization tracking and precise overflow rules. Both frontend
and emission verifier need extending. Borrow checking, arbitrary libraries,
device drivers and a graphical OS are not supplied by this foundation.
