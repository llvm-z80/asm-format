  addi a0,a0,3 # add
  # line

  lui a0,%hi(msg)
  addi a0,a0,%lo(msg)

  lw a1,4(a2)
  sw ra,12(sp)

  lw a1,4(a2) ; lw a3,8(a2)

  li a0,1
  call f
  ret

#define LOAD(r) \
	lw r, 0(sp)
  LOAD(a0)
