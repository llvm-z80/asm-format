  ld a,b ; save

  ld (ix+SPRITE_BUFFER_OFFSET),a ; mirror


  ret ; done
