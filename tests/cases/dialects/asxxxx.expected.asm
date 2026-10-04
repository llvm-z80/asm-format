foo:    ld      a, b    ; c

        .asciz  /a, b/(13) (10)/c ;d/   ; e

        .ascii  ^/a, b/ ; c
        .ascii  ^(x, y( ; c

        .ascii  ((1+2), 3)/a b/

        .ascii/a, b/(10)

        .include /x, y/
        .incbin  (f, g( , 0
