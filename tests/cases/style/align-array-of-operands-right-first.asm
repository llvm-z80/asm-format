    pavgb m3, [r0+r3]
    movh [r0], m0
    movh [r0+r2],m1
    movh  [r0+r2*2], m2
    movh [r0+r3], m3
    RET
    mova m0, [r1+r2*2+0x100]
    mova [r0+r1*2+0x1000], m0
