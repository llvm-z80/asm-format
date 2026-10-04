format ELF executable 3
        entry   start
segment readable executable
start:
        mov     [con_handle], 1
        mov     esi, _logo
.loop:  dec     ecx
        jnz     .loop
_logo   db      'flat assembler', 0
_usage  db      0xA
        db      'usage', 0xA
macro display_string str {
        mov     esi, str
}
