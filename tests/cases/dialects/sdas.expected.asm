foo:    ld      a, b    ; c
        ret

loop:
loop2:

a: b:   nop
c:d: e: ret

very_long_label: ld a, b

___udivqi3:: xor a

00101$:
        jr      00101$
10$:    djnz    10$

loop:   dec     b

.area _CODE
.globl _f
_x = 0x1234
C$t.c$2$0_0$1 ==.

        .area   _CODE (REL,CON)
        .globl  _f
        .db     1, 2, 3
        .dw     #label, 0
        .ds     2

        ld      a,     #0x01
        ld      0(ix), a
        ld      a,     (hl)
        jp      (hl)
        ex      (sp), hl
        ld      a,    #<label
        ld      a,    #>label

        ld      a, #';  ; semicolon
        ld      a, #',
        ld      a, #'\n
        ld      a, #'\001

        ld      a, #' 

        ld      hl, #"AB
        ld      hl, #"a,

        ex      af, af' ; swap
        ex      af, af'

; column zero
        ; indented
        ; tab
foo:            ; label
        nop     ;glued

        ld      hl, #a_very_long_symbol_name_here       ; c

        .ascii  ;a, b;  ; c
        .ascii/a, b/
        .asciz  "x, y"
        .str    'it'
        .fcc    |a b|

        .title  A;B, C
        .sbttl  sub ; title

        .define kw, /a, b/
        .define kw2 /x y/
        .define kw3/p, q/
        .define kw4 ^/r, s/

        .include /a, b.inc/
        .incbin  "f.bin", 0, 16
        .msg     /a, b/

        nop

        ret

        nop
        ret

        nop
; asm-format off
table:  .db 1,2,3,  4,5,6
; asm-format on
        ret

        .ascii  /äö/    ; c

        nop
  ;  asm-format off  
  ld a,b
