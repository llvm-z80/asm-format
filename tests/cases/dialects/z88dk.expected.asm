SECTION code_clib
PUBLIC asm_memcpy
.asm_memcpy
        ld      a, b    ; size
        or      c
        ret     z
        ex      af, af'
loop:   ldi \ jp pe,loop
        ld      a, 1 : ld b,2
        ld      a, ';'  // comment
        defb    1, 2,\
                3
SIZE    equ     16
. spaced ld     a, b
