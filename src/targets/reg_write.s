.global main

.section .data

hex_format:         .asciz "%#Lx"
float_format:       .asciz "%.2f"
long_float_format:  .asciz "%.2Lf"

.section .text

.macro trap
    # %rax <- kill(my_pid, SIGTRAP /* 62 */)
    movq    $62, %rax
    movq    %r12, %rdi
    movq    $5, %rsi
    syscall
.endm

main:
    push    %rbp
    movq    %rsp, %rbp

    # getpid()
    movq    $39, %rax
    syscall
    movq    %rax, %r12

    trap

    # printf(hex_format, %rsi)
    leaq    hex_format(%rip), %rdi
    movq    $0, %rax
    call    printf@plt
    movq    $0, %rdi
    call    fflush@plt

    trap
    
    # printf(hex_format, %mm0)
    movq    %mm0, %rsi
    leaq    hex_format(%rip), %rdi
    movq    $0, %rax
    call    printf@plt
    movq    $0, %rdi
    call    fflush@plt

    trap
    
    # printf(float_format, %xmm0)
    leaq    float_format(%rip), %rdi
    movq    $1, %rax
    call    printf@plt
    movq    $0, %rdi
    call    fflush@plt

    trap
    
    # printf(long_float_format, %st0)
    subq    $16, %rsp
    fstpt   (%rsp)
    leaq    long_float_format(%rip), %rdi
    movq    $0, %rax
    call    printf@plt
    movq    $0, %rdi
    call    fflush@plt
    addq    $16, %rsp

    trap

    popq    %rbp
    movq    $0, %rax
    ret
