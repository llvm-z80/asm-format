        mov     r0, #1  @ one
        @ comment line
# 1 "file.c"

        add     r0, r1,   #4
        ldr     r0, [r1,#4]!
        ldr     r0, [r1], #4
        ldr     r0, =0x1234

        push    {r4,r5,lr}
        ldm     r0!, {r1-r3}

        it      eq
        moveq   r0, #0

        .asciz  "x @ y", "z"

#define ADD3(r) \
	add r, r, #3
        ADD3(r0)        @ add

        nop ; nop

        .macro  dec_loop reg
                subs    \reg, \reg, #1
                bne     .L_\@   @ again
                mov     r0, r1
        .endm
