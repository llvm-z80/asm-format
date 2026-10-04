%include "x86inc.asm"
SECTION .text
global func
func:   mov eax,[esp+4] ; load
  rep movsb
  lock xadd [rdi],eax
  [bits 64]
PW_ONE times 8 dw 1
SIZE equ 16
.loop: dec ecx
  jnz .loop
%%skip: nop
  db 'a,b', "a;b", `c\`;d`, 0 ; strings
%macro FOO 2
  mov %1,%2
%endmacro
  %defstr VER 1,2 ; version
  mov eax, \
ebx
