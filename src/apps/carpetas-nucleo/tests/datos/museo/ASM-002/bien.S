    .globl suma
suma:
    push %rbx
    mov %rdi, %rbx
    add %rsi, %rbx
    mov %rbx, %rax
    pop %rbx
    ret
