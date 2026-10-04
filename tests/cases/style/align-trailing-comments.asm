loop: ld a,(hl) ; read
  inc hl
  ld (ix+SPRITE_BUFFER_OFFSET),a ; mirror
  djnz loop ; next

  ld a,b ; save
  or c ; zero?
  ret z ; done

  ld a,b ; save
  call print_unsigned_decimal_with_padding_sign_and_thousands_separator ; long
  ret z ; done

  ld b,#2 ; 2 cycles
          ; continues
  ; a new part
  ld c,d ; c
lbl: ; label
  nop

COUNT = 3 ; kept at column 0
          ; continues
  ; a new part
