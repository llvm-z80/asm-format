#include "textflag.h"

// func add(a, b int) int
TEXT ·add(SB),NOSPLIT,$0-24
	MOVQ	a+0(FP),AX	// a
	ADDQ	b+8(FP),AX
	MOVQ	AX,ret+16(FP)
	RET
loop:
	DECQ CX; JNZ loop
#define ROUND(a, b) \
	MOVQ a,b; \
	ADDQ b,a
DATA ·msg+0(SB)/8,$"hi, you\n"
GLOBL ·msg(SB),RODATA,$8
