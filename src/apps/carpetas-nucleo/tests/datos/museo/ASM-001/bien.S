    .globl llama
llama:
    push %rbx
    call otra
    pop %rbx
    ret
