    DEF   WIDTH EQU 32
    ld a,[hl+]
    DB    $00,$04,$0B
    SECTION "x",ROM0
    LONG_CALL   Foo
.loop:
    REPT    16
        add hl,hl
    ENDR
