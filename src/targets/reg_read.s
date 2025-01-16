.global main

.section .data

my_double: .double 135.79
.equ MY_INT, 0x00c0ff331deadb01

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

    # set r13
    movq    $MY_INT, %r13
    movq    $44, %rax
    trap

    # set r13b
    movb    $42, %r13b
    movq    $99, %rax
    trap

    # set ah
    movq    $0, %rbx
    movb    $21, %bh
    trap

    # set mm0
    movq    $0xba5eba11, %r13
    movq    %r13, %mm0
    trap

    # set xmm0
    movsd   my_double(%rip), %xmm0
    trap

    # set st0
    emms
    fldl    my_double(%rip)
    trap

    popq    %rbp
    movq    $0, %rax
    ret
