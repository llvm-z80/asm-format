        nop                     # c
        movl    %eax, %ebx      #glued
/ slash comment
        nop

foo: /* c */ nop
        movl    $1 /* a, b */, %eax
        nop     /* a
  b */  nop
        ret
  /* a
     b */
        ret

#include "x.h"
#define X(a) \
  movl a,%eax ;\
  ret
        #if FOO
        nop
        #endif

.L1:
1:      jmp     1b
"a b":  nop
foo: bar: nop

        movb    $'a, %al
        movb    $';, %al
        .byte   'a', ';', '\n', ','

        .ascii  "a, b # c", "d"
        .string "x;y"

        movl    %eax, %ebx ; addl $1,%eax
        rep; movsb
        nop;nop

        lock xaddl %eax, (%rbx)
        rep movsb
        {vex} vpaddd %ymm0, %ymm1, %ymm2
        data16 nop

        movl    4(%rax,%rbx,4), %eax
        leaq    -8(%rbp), %rdi
        jmp     *%rax
        movl    %fs:0x28, %eax

        mov     eax, DWORD PTR [ebx+4]
        lea     rdi, [rip+msg]

        .section .text.foo, "ax", @progbits
        .type    f, @function
        .size    f, .-f

        .byte   1,(2

        .macro  foo a, b
                movl    \a, \b
        .endm
        foo     %eax, %ebx
