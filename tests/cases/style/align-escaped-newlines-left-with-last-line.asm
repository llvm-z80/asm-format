%define REGS rax, \
  rbx,  \
     rcx_and_more_registers
  db 1, 2, \
 3, 4,\
  5
label: mov eax,   \
      ebx ; comment
