        .dw _start, 0x0100, 3
        .dw _main_loop, 0x10, 12
        .dw _irq, 0x2000, 7

        ld a, (hl)
        ld (de), a
        ld c, a

        ld a,b
        ld de, 300
