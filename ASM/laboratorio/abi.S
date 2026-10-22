/* SPDX-License-Identifier: GPL-2.0-only */
.text
.globl ultra_abi_probe
.type ultra_abi_probe,@function
ultra_abi_probe:
    push %rbp
    push %rbx
    push %r12
    push %r13
    push %r14
    push %r15
    sub $8, %rsp
    mov %rdi, %rax
    mov $101, %rbx
    mov $102, %r12
    mov $103, %r13
    mov $104, %r14
    mov $105, %r15
    mov $106, %rbp
    call *%rax
    /* Leaf returns entry rsp modulo 16. Ordinary SysV entry must be 8. */
    xor %edx, %edx
    cmp $8, %eax
    je 1f
    or $64, %edx
1:  cmp $101, %rbx
    je 2f
    or $1, %edx
2:  cmp $102, %r12
    je 3f
    or $2, %edx
3:  cmp $103, %r13
    je 4f
    or $4, %edx
4:  cmp $104, %r14
    je 5f
    or $8, %edx
5:  cmp $105, %r15
    je 6f
    or $16, %edx
6:  cmp $106, %rbp
    je 7f
    or $32, %edx
7:  mov %edx, %eax
    add $8, %rsp
    pop %r15
    pop %r14
    pop %r13
    pop %r12
    pop %rbx
    pop %rbp
    ret
.size ultra_abi_probe,.-ultra_abi_probe
.globl ultra_abi_good
.type ultra_abi_good,@function
ultra_abi_good:
    mov %rsp, %rax
    and $15, %eax
    ret
.size ultra_abi_good,.-ultra_abi_good
.globl ultra_abi_bad
.type ultra_abi_bad,@function
ultra_abi_bad:
    xor %ebx, %ebx
    xor %r12d, %r12d
    xor %r13d, %r13d
    xor %r14d, %r14d
    xor %r15d, %r15d
    xor %ebp, %ebp
    jmp ultra_abi_good
.size ultra_abi_bad,.-ultra_abi_bad
/* Intentionally misaligned call to an observer leaf ONLY; no libc/SIMD. */
.globl ultra_abi_bad_alignment
.type ultra_abi_bad_alignment,@function
ultra_abi_bad_alignment:
    call ultra_abi_good
    ret
.size ultra_abi_bad_alignment,.-ultra_abi_bad_alignment
.section .note.GNU-stack,"",@progbits
