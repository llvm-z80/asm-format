        .macro  copy n
                .if     n
                        ld      a, (hl) ; load
                        ; one byte
                .else
                        .rept   2
                                ldi
                        .endm
                .endif
        .endm
.macro top
        nop
.endm
        .irp    r, b, c
loop:           ld      a, r
        .endm
        .endif
        ret
