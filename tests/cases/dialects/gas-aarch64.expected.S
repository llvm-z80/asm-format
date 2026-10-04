        add     w0, w0, #3      // add
        // line
#include "x.h"

        ldp     x29, x30, [sp,#-16]!
        ldp     x29, x30, [sp], #16
        ldr     x0,  [x1,x2,lsl #3]

        add     x0, x1, x2, lsl #3

.Lfoo:
1:      b       1b

        .asciz  "a // b", "c"

#define RET \
	ret
        RET
