        .export         _memcpy
        .import         popax
WNDLFT  :=      $20             ; left
WNDTOP = $22
_memcpy:
        jsr     popax
@loop:  lda     (ptr1),y        ; copy
        sta     (ptr2),y
:       iny
        bne     :-
        lda     #'a'
        lda     #';'
        .byte   "a;b", 0
        .proc   foo
        rts
        .endproc
