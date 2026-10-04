        ld      a, b ; save
        or      c ; zero?
        ld      (ix+SPRITE_BUFFER_OFFSET), a ; mirror
