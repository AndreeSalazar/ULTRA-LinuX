    .globl llama
llama:
    push %rbx
    push %r12
    call otra
    pop %r12
    pop %rbx
    ret
