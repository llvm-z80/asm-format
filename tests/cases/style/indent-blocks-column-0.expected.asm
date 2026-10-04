        .if     GUARD
GUARD = 1
        .macro  m
                nop
        .endm
        .if     X
                nop
        .endif
        .endif
