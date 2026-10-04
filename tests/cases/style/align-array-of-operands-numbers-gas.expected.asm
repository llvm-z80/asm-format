        movl    %eax,  %ebx
        movb    %al,   %bl
        movl    $func, %eax
        movl    $0x10, %eax
        movl    $10,   %ecx

        .byte     1,   2,   3
        .byte   100, 200, 300
