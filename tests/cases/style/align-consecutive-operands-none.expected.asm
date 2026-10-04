        .GG_STATE = 0x00
        .GGSTATE_STT = 0b10000000
        .GGSTATE_NJAP = 0b01000000
        ld      a, b
        .optsdcc -mz80
        ld      c, d
        .SIOCTL_TXFL = 0b00000001

        ld      e, h
        safe_svcmode_maskall r0
        .SOUNDPAN_TN1R = 0b00000001
