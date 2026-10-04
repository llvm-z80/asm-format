        .dw     _start, 0x0100, 3
        .dw     _main,  0x10,   12
        .dw     _irq,   0x2000, 7

        ld       a,    (hl)
        ld       (de), a
        inc      hl
        ld       (ix+SPRITE_BUFFER_OFFSET), a
        ld       a, b
        add      a, c
        .optsdcc -mz80
        ld       b,    (hl)
        ld       (hl), c        ; store

        ld      (ix+0x10), a
        ld      (ix+0x11), b
        ld      c, a
        ld      d, b
