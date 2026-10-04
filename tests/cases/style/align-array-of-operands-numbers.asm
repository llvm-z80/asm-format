        palette 4, 4, 2
        palette 28, 28, 10
        .dw _RAMBANK, _SCRN0 + 5 * SCRN_VX_B + 0, 18
        .dw _RAMBANK, _VRAM, 64

        ld a, #0x38
        ld l, #0xA7
        lb bc, 1, 7

        .db 1, 2
        .db 100, 200, 3
