    .globl suma
suma:
    mov %rdi, %rbx
    add %rsi, %rbx
    mov %rbx, %rax
    ret
    .globl cola
cola:
    xor %r12d, %r12d
    jmp suma
