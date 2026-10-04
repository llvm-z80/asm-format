.model flat
.code
x64 equ 1
MY_PROC macro name:req, n:req
  align 16
endm
main PROC
  mov eax,ebx ; comment
  rep movsb
@@: dec ecx
  jnz @B
  ifidni <a,b>,<c>
  endif
  COMMENT ! block
  mov eax , 1
  !
  mov eax, \
ebx
main ENDP
data db 'it''s',0
END
