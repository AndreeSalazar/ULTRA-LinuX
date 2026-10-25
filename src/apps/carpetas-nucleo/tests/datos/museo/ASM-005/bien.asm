global divide
divide:
    mov rax, rdi
    xor edx, edx
    div rsi
    ret
