Main::
.loop
        dec     a
        jr      nz, .loop
:       jr      :-

Main.done: ret
Main.wait
        halt
.l2:    nop

Label:  ld      a, b

SECTION "main", ROM0[$150]
DEF COUNT EQU 3
INCLUDE "hardware.inc"

        ld      a, b :: ld c,d
        ld      a, b::ld c,d

        db      1, 2,\
                3,4
        db      1,\     ; first
                2

        db      "a\"b, c", 0
        db      #"a\",     0
        db      #"""x,
y""",0

        db      "a\
  b, c",0

        db      """a,
  b""",0

        ld      a, 'A'
        cp      ','
        cp      '\''

MACRO add2
        add     a, \1
        ld      b, \<10>
.l\@
        jr      .l\@
ENDM
        MAC     a\, b, \1

        ld      a, {d:COUNT}
{PREFIX}_label: nop

        ld      a, /* x, y */ 5
  /* multi
     line */
        nop

        ld      [hl+], a
        ldh     [c],   a
        ld      a,     [$ff00+c]
        ld      hl,    sp+3
