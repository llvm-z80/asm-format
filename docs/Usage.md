# Usage

asm-format formats assembly source files. The syntax of the source is described
by a dialect, as explained in [Dialects.md](Dialects.md). The layout is
described by a style file, as explained in this document.

## Changes

asm-format changes only spaces, tabs and line terminators:

- the indentation of statements, comment lines and continued lines;
- the whitespace between the label, mnemonic, operand and comment fields;
- the whitespace after commas in the operand field;
- the whitespace before line continuations;
- trailing whitespace;
- the number of consecutive empty lines, and the empty lines at the start and
  end of the file;
- line terminators.

Files in any character encoding can be formatted. A line ends with LF, CR
followed by LF, or CR not followed by LF. By default, each line keeps its line
terminator.

## Command line

```
asm-format [options] [<file> ...]
```

Without file arguments, asm-format formats standard input and writes the
result to standard output. With file arguments, it formats each file and
writes the result to standard output, or back to the file if `-i` is given.
The file name `-` stands for standard input.

| Option | Description |
|---|---|
| `-i` | Edit the files in place. A file is written only if its contents change. |
| `-n`, `--dry-run` | Do not change any file. A warning is reported for each file that would change. |
| `--Werror` | Report formatting warnings as errors. |
| `--style=<style>` | The style to use. `file` (the default) searches for a style file, `file:<path>` uses the given file, and a string that begins with `{` is read as a YAML mapping. |
| `--assume-filename=<path>` | The file name of standard input, used to find the style file and to match `Overrides`. |
| `--dump-config` | Print the options in effect and exit. |
| `--help` | Print the available options. |
| `--version` | Print the version. |

```sh
# Format files in place.
asm-format -i src/*.asm

# Check files without changing them, for example in CI.
asm-format --dry-run --Werror src/*.asm

# Format standard input with an inline style.
asm-format --style="{Dialect: rgbds}" < main.asm
```

## Style files

A style file names the dialect of the source files and sets the options
described in [Options](#options).

```yaml
Dialect: sdas
IndentWidth: 4
OperandColumn: 12
CommentColumn: 28
```

### File names

A style file has one of these names:

- `.asm-format`
- `asm-format`
- `.asm-format.yaml`
- `asm-format.yaml`

It is an error for a directory to contain more than one of them.

### Lookup

For each input file, asm-format uses the style file in the closest directory
that contains the input file: the directory of the file, then its parent, and
so on. Other style files are read only through `BasedOnStyle`. For standard
input, the search starts from the directory of `--assume-filename`, or from the
current directory if it is not given. A `--style` that gives a path or an
inline style replaces the search.

If no style file is found, the default values are used. `Dialect` has no
default value, so it must then be given with `--style`.

A style file that sets `BasedOnStyle: InheritParentConfig` is applied on top of
the style file of the closest parent directory, as described for
[`BasedOnStyle`](#basedonstyle).

### Format

A style file is a YAML document that contains one mapping. Keys are
case-sensitive, and unknown keys are errors.

### Default values

The style file below sets every option to its default value. `Dialect` has no
default value; `sdas` is only an example. `DirectiveOperandColumn` follows
`OperandColumn` unless it is set. `BasedOnStyle` and `Overrides` are not set by
default. `--dump-config` prints the options in effect in the same format.

```yaml
Dialect: sdas
IndentWidth: 8
OperandColumn: 16
DirectiveOperandColumn: 16
AlignConsecutiveMnemonics:
  Kind: Left
  AcrossEmptyLines: false
  AcrossComments: false
  MaxPadding: 0
AlignConsecutiveOperands:
  Kind: Left
  AcrossEmptyLines: false
  AcrossComments: false
  MaxPadding: 4
AlignArrayOfOperands:
  Kind: Left
  MaxPadding: 4
  Numbers: Left
CommentColumn: 0
SpacesBeforeTrailingComments: 1
AlignTrailingComments:
  Kind: Left
  OverEmptyLines: 0
  AlignToTab: true
  MaxColumn: 80
SpaceAfterComma: true
IndentComments: true
IndentBlocks: true
BlockIndentWidth: 8
ContinuationIndentWidth: 8
AlignEscapedNewlines: Left
UseTab: Never
TabWidth: 8
MaxEmptyLinesToKeep: 1
KeepEmptyLines:
  AtStartOfFile: true
  AtEndOfFile: false
LineEnding: Keep
InsertNewlineAtEOF: true
DisableFormat: false
```

## Layout

### Columns

Columns are counted from 0 at the start of a line. A tab advances to the next
multiple of `TabWidth`. A UTF-8 character counts as one column.

```asm
loop:   ld      a, (hl) ; comment
^       ^       ^       ^
0       8       16      24
```

### Lines

asm-format does not change:

- the lines from an `asm-format off` comment to an `asm-format on` comment;
- lines that begin inside a multi-line comment or string;
- lines continued from the previous line, if the dialect continues lines by a
  column.

The other lines are formatted as follows:

- Lines continued from the previous line are described in
  [Continued lines](#continued-lines).
- Lines that begin at column 0 without a label, such as directives, macro
  definitions and preprocessor lines, keep their text. Only their trailing
  whitespace is removed and the whitespace before a line continuation is set.
  If `IndentWidth` is 0, these lines are formatted like other statements.
- Lines that contain only whitespace become empty. The number of consecutive
  empty lines is limited by `MaxEmptyLinesToKeep`.
- Lines that contain only a comment are described in
  [Comment lines](#comment-lines).
- All other lines are described in [Statement lines](#statement-lines).

Whitespace means the bytes space and tab. Trailing whitespace is whitespace at
the end of a line that is not part of a comment or string. It is kept if
removing it would change how the line is read, for example where a statement
separator at the end of a line would otherwise become a line continuation. An
empty line keeps one space if its line terminator is LF and the previous line
ends with CR, because the two would otherwise form one line terminator.

### Statement lines

The dialect divides a statement into labels, a mnemonic, operands and a
trailing comment. Only the first statement of a line is formatted. The text
from the first statement separator to the end of the line is kept, together
with the whitespace before it.

The fields are placed as follows. A field begins at the column given below only
if the preceding text ends before that column. Otherwise, it follows the
preceding text after one space.

1. Labels begin at column 0. Whitespace between two labels is replaced by one
   space. Where there is none, none is inserted.
2. The mnemonic begins at column `IndentWidth`, moved right inside blocks as
   described for [`IndentBlocks`](#indentblocks). See also
   [`AlignConsecutiveMnemonics`](#alignconsecutivemnemonics).
3. The operands begin at column `OperandColumn`, or `DirectiveOperandColumn`
   for a directive, moved right inside blocks by the same number of columns as
   the mnemonic. If the column is 0, the operands follow the mnemonic after one
   space. If the input has no whitespace between the mnemonic and the
   operands, none is inserted. See also
   [`AlignConsecutiveOperands`](#alignconsecutiveoperands) and
   [`AlignArrayOfOperands`](#alignarrayofoperands).
4. The trailing comment is placed as described for
   [`AlignTrailingComments`](#aligntrailingcomments).

In the first line below, the label ends after column `IndentWidth`, so the
mnemonic follows it after one space, and the operands follow the mnemonic in
the same way.

```asm
very_long_label: ld a, b
loop:   inc     hl
```

Whitespace inside a field is not changed, except after commas as described for
[`SpaceAfterComma`](#spaceaftercomma).

If a comment that is not the trailing comment begins where the mnemonic would
begin, the text from that comment to the end of the line is kept, together
with the whitespace before it.

If the dialect continues lines by a column, the part of each line before that
column is formatted, and the rest keeps its columns. If the formatted part
reaches that column, or a comment or string crosses it, the line is kept
unchanged.

### Comment lines

A comment line contains only a comment. If `IndentComments` is `true` and the
comment ends on the same line, the comment is placed as follows. Otherwise,
its leading whitespace is kept.

- If the previous line ends with a trailing comment that began at the same
  column as this comment in the input, this comment continues that trailing
  comment and begins at its column.
- Otherwise, the comment begins at the column of the mnemonic of a statement
  on that line, which is `IndentWidth` outside blocks.

```asm
        ld      b, #2   ; 2 cycles
                        ; 24 cycles in total
        ; next part
        ret
```

### Continued lines

A line that ends with a line continuation of the dialect, followed only by
whitespace and comments, continues on the next line. Continued lines are
formatted as follows:

- The leading whitespace is replaced so that the line begins
  `ContinuationIndentWidth` columns after the column of the mnemonic of the
  first line, or after column 0 if the first line has no mnemonic. A continued
  line that does not begin with whitespace is not indented.
- The whitespace before each line continuation is set by
  `AlignEscapedNewlines`. A line continuation with no whitespace before it is
  not moved.
- Trailing whitespace is removed. The rest of the line is not changed.

```asm
; NASM
%define REGS rax, \
        rbx,      \
        rcx_and_more_registers
        db      1, 2, \
                3, 4
```

## Options

Most options have an example that formats the same input with different
values. The comment above each result names the values used. The examples use
the `sdas` dialect and the default values of the other options, unless the
comment says otherwise.

The options that align text share their values. `None` turns the alignment
off, and `Left` and `Right` give the side to which the text is aligned. Each of
them can be given as a mapping with the key `Kind`, or as a value of `Kind`
alone, which sets only `Kind`.

### `BasedOnStyle`

Type: `InheritParentConfig`. No default.

If set to `InheritParentConfig`, the style file of the closest parent
directory is read first, and the options of this style file are applied on
top of it. That style file can itself inherit from its parent. If no such
style file is found, the default values are used as the base. For an inline
style given with `--style`, the search starts from the directory of the input
file. This option is not allowed in `Overrides`.

```yaml
# sm83/.asm-format
BasedOnStyle: InheritParentConfig
Dialect: rgbds
```

### `Dialect`

Type: string. No default.

The dialect of the source files. asm-format reads the dialect file
`<name>.toml` from the directory of the style file that sets `Dialect`. If
that file does not exist, the built-in dialect with that name is used. For an
inline style given with `--style`, the current directory is used instead of
the directory of the style file.

Built-in dialects:

| Name | Assembler |
|---|---|
| `asxxxx` | ASxxxx assemblers |
| `sdas` | Assemblers of SDCC, such as sdasz80 and sdasgb |
| `rgbds` | RGBDS rgbasm |
| `gas-x86` | GNU as for x86 |
| `gas-arm` | GNU as for ARM |
| `gas-aarch64` | GNU as for AArch64 |
| `gas-riscv` | GNU as for RISC-V |
| `nasm` | NASM and YASM |
| `masm` | Microsoft MASM and compatible assemblers |
| `fasm` | flat assembler |
| `go` | The Go assembler |
| `ca65` | ca65 of cc65 |
| `z88dk` | z80asm of z88dk |

### `IndentWidth`

Type: unsigned integer. Default: `8`.

The column at which the mnemonic begins. If `0`, lines that begin at column 0
are formatted like other statements.

```asm
; IndentWidth: 8
___udivqi3:
        ld      d, a
        ld      e, #8

; IndentWidth: 4, OperandColumn: 12
___udivqi3:
    ld      d, a
    ld      e, #8
```

### `OperandColumn`

Type: unsigned integer. Default: `16`.

The column at which the operands begin. If `0`, the operands follow the
mnemonic after one space. If the mnemonic reaches this column, the operands
follow it after one space, or at the next tab stop if `UseTab` is `Always`.

```asm
; OperandColumn: 16
        ld      a, b

        .optsdcc -mz80

; OperandColumn: 0
        ld a, b

        .optsdcc -mz80
```

### `DirectiveOperandColumn`

Type: unsigned integer. Default: the value of `OperandColumn`.

The column at which the operands of a directive begin. The dialect lists its
directives. The value has the same meaning as for `OperandColumn`. If this
option is set, `AlignConsecutiveOperands` aligns directives and other lines
separately.

```asm
; RGBDS, IndentWidth: 4, OperandColumn: 12
    DEF     WIDTH EQU 32
    inc     hl
    DB      $00, $04, $0B

; RGBDS, IndentWidth: 4, OperandColumn: 12, DirectiveOperandColumn: 0
    DEF WIDTH EQU 32
    inc     hl
    DB $00, $04, $0B
```

### `AlignConsecutiveMnemonics`

Type: `None`, `Left` or mapping. Default: `{Kind: Left, AcrossEmptyLines:
false, AcrossComments: false, MaxPadding: 0}`.

Aligns the mnemonics of consecutive lines whose label reaches `IndentWidth`. A
line whose label ends before `IndentWidth` keeps its mnemonic at `IndentWidth`
and does not end a run.

| Key | Type | Meaning |
|---|---|---|
| `Kind` | `None` or `Left` | `None` does not align. `Left` makes the mnemonics of the lines of a run begin at the same column. |
| `AcrossEmptyLines` | boolean | If `true`, empty lines do not end a run. |
| `AcrossComments` | boolean | If `true`, lines that contain only a comment do not end a run. |
| `MaxPadding` | unsigned integer | The largest difference between the columns that the lines of a run need. A line that would make it larger begins a new run. `0` means no limit. |

A run ends at a line that has no label or no mnemonic. A value of `None` or
`Left` sets `Kind`.

```asm
; NASM, AlignConsecutiveMnemonics: Left
FSP_HEADER_IMGBASE_OFFSET EQU 1Ch
STACK_SAVED_RAX_OFFSET    EQU 8*7
SHORT   EQU     1

; NASM, AlignConsecutiveMnemonics: None
FSP_HEADER_IMGBASE_OFFSET EQU 1Ch
STACK_SAVED_RAX_OFFSET EQU 8*7
SHORT   EQU     1
```

### `AlignConsecutiveOperands`

Type: `None`, `Left` or mapping. Default: `{Kind: Left, AcrossEmptyLines:
false, AcrossComments: false, MaxPadding: 4}`.

Aligns the column at which the operands begin on consecutive lines. A line
whose mnemonic reaches `OperandColumn`, such as a symbol assignment with a long
name, moves its operands right, and the other lines of its run follow it. A
run of lines whose mnemonics all end before `OperandColumn` is not changed.
The operands within the operand field are aligned by
[`AlignArrayOfOperands`](#alignarrayofoperands). The keys are the same as for
[`AlignConsecutiveMnemonics`](#alignconsecutivemnemonics).

A run ends at a line without operands or without whitespace before them, and
at a line whose mnemonic begins at a different column. `MaxPadding` limits how
far a line is moved, so a much longer mnemonic does not move the lines around
it. If `OperandColumn` is `0`, every mnemonic reaches it, so the operands of
all consecutive lines are aligned.

In the example below, `ld a, b` after the assignments would have to move six
columns, more than the default `MaxPadding` of 4, so it stays.

```asm
; AlignConsecutiveOperands: Left
        .GG_STATE     = 0x00
        .GGSTATE_STT  = 0b10000000
        .GGSTATE_NJAP = 0b01000000
        ld      a, b

        ld       a, b
        .optsdcc -mz80
        ld       c, d

; AlignConsecutiveOperands: None
        .GG_STATE = 0x00
        .GGSTATE_STT = 0b10000000
        .GGSTATE_NJAP = 0b01000000
        ld      a, b

        ld      a, b
        .optsdcc -mz80
        ld      c, d
```

```asm
; IndentWidth: 4, OperandColumn: 0, AlignConsecutiveOperands: Left
    ld   a, b
    push de
    jr   nz, .loop

; IndentWidth: 4, OperandColumn: 0, AlignConsecutiveOperands: None
    ld a, b
    push de
    jr nz, .loop
```

### `AlignArrayOfOperands`

Type: `None`, `Left`, `Right`, `RightFirst` or mapping. Default: `{Kind: Left,
MaxPadding: 4, Numbers: Left}`.

Aligns the operands of consecutive lines as the columns of a table. The
operands are divided at the commas that `SpaceAfterComma` changes, and the
space after each such comma is widened so that each operand begins at the same
column as in the other lines.

| Key | Type | Meaning |
|---|---|---|
| `Kind` | `None`, `Left`, `Right` or `RightFirst` | How the columns are aligned. See the table below. |
| `MaxPadding` | unsigned integer | With `Left` and `Right`, the largest difference between the widths of the first operands in a group. With `RightFirst`, the largest number of columns by which the commas move right to make room for a long first operand. With every kind, the other columns are aligned up to the first column that would need more padding than this. `0` means no limit. |
| `Numbers` | `Left` or `Right` | With `Right`, a column of a group whose operands are all numbers is aligned to the right. This has no effect if `Kind` is `Right`. |

| `Kind` | Meaning |
|---|---|
| `None` | The operands are not aligned. |
| `Left` | Every column is aligned to the left. |
| `Right` | Every column is aligned to the right. |
| `RightFirst` | The first operands are aligned to the right, so that the commas after them line up, and the other columns to the left. |

A value of `None`, `Left`, `Right` or `RightFirst` sets `Kind`. This option has
no effect if `SpaceAfterComma` is `false`.

A group is a run of consecutive lines that have two or more operands, whose
operands begin at the same column, and whose first operands differ in width by
at most `MaxPadding` columns. A line that does not fit begins a new group.

A column aligned to the right ends as far left as the operands before it
allow. The first operands are aligned to the right only if every line of the
group has whitespace between the mnemonic and the operands.

With `RightFirst`, the operands of a group need not begin at the same column.
The commas line up where the shortest first operand ends when it begins at the
operand column. A longer first operand begins left of that column, but at
least one space after its mnemonic. If it does not fit, the commas of the
group move right. A line without whitespace between the mnemonic and the
operands, or with a tab in its first operand, is not part of a group.

A number is a decimal digit followed by letters, digits and underscores, `$`
followed by such a number or by hexadecimal digits, or `%` followed by binary
digits. Each form can be preceded by `#` and `-`.

```asm
; AlignArrayOfOperands: None
        .dw     _start, 0x0100, 3
        .dw     _main_loop, 0x10, 12
        .dw     _timer, 0x2000, 7

        ld      a, (hl)
        ld      (de), a
        ld      c, a

; AlignArrayOfOperands: Left
        .dw     _start,     0x0100, 3
        .dw     _main_loop, 0x10,   12
        .dw     _timer,     0x2000, 7

        ld      a,    (hl)
        ld      (de), a
        ld      c,    a

; AlignArrayOfOperands: Right
        .dw         _start, 0x0100,  3
        .dw     _main_loop,   0x10, 12
        .dw         _timer, 0x2000,  7

        ld         a, (hl)
        ld      (de),    a
        ld         c,    a

; AlignArrayOfOperands: RightFirst
        .dw     _start, 0x0100, 3
        .dw _main_loop, 0x10,   12
        .dw     _timer, 0x2000, 7

        ld      a, (hl)
        ld   (de), a
        ld      c, a
```

```asm
; AlignArrayOfOperands: {MaxPadding: 4}
        .dw     _start,     0x0100, 3
        .dw     _main_loop, 0x10,   12
        .dw     _irq, 0x2000, 7

; AlignArrayOfOperands: {MaxPadding: 8}
        .dw     _start,     0x0100, 3
        .dw     _main_loop, 0x10,   12
        .dw     _irq,       0x2000, 7
```

```asm
; AlignArrayOfOperands: {Numbers: Left}
        palette 4,  4,   2
        palette 28, 28,  10
        palette 31, RED, 0

; AlignArrayOfOperands: {Numbers: Right}
        palette  4, 4,   2
        palette 28, 28, 10
        palette 31, RED, 0
```

```asm
; NASM, IndentWidth: 4, OperandColumn: 14, AlignArrayOfOperands: RightFirst
    pavgb       m3, [r0+r3]
    movh      [r0], m0
    movh   [r0+r2], m1
    movh [r0+r2*2], m2
```

### `CommentColumn`

Type: unsigned integer. Default: `0`.

The column before which a trailing comment does not begin.

```asm
; AlignTrailingComments: None, CommentColumn: 0
        ld      d, a ; D = dividend
        jr      c, ___udivqi3_skip ; skip

; AlignTrailingComments: None, CommentColumn: 32
        ld      d, a            ; D = dividend
        jr      c, ___udivqi3_skip ; skip
```

### `SpacesBeforeTrailingComments`

Type: unsigned integer. Default: `1`.

The minimum number of spaces between the preceding text and a trailing
comment.

```asm
; AlignTrailingComments: None, CommentColumn: 32,
; SpacesBeforeTrailingComments: 1
        ld      d, a            ; D = dividend
        jr      c, ___udivqi3_skip ; skip

; AlignTrailingComments: None, CommentColumn: 32,
; SpacesBeforeTrailingComments: 2
        ld      d, a            ; D = dividend
        jr      c, ___udivqi3_skip  ; skip
```

### `AlignTrailingComments`

Type: `None`, `Left` or mapping. Default: `{Kind: Left, OverEmptyLines: 0,
AlignToTab: true, MaxColumn: 80}`.

Controls the placement of trailing comments. A trailing comment begins at
least `SpacesBeforeTrailingComments` columns after the preceding text, not
before column `CommentColumn`, and, on a line with a mnemonic, after column
`IndentWidth`.

| Key | Type | Meaning |
|---|---|---|
| `Kind` | `None` or `Left` | `None` places each trailing comment by itself. `Left` makes the trailing comments of a block of lines begin at the same column. |
| `OverEmptyLines` | unsigned integer | The number of consecutive empty lines that do not end a block. |
| `AlignToTab` | boolean | If `true`, the column of a block is rounded up to a multiple of `TabWidth`. |
| `MaxColumn` | unsigned integer | A line whose trailing comment would begin after this column is not aligned, and its comment begins `SpacesBeforeTrailingComments` columns after the preceding text. Inside blocks, the column moves right with the indentation. `0` means no limit. |

A block of lines ends at an empty line, at a line that contains only a comment
and does not continue a trailing comment, and at a line that asm-format keeps
or that begins at column 0 without a label. Lines without a trailing comment
do not end a block. The trailing comments of a block begin at the smallest
column that is allowed for all of them. A value of `None` or `Left` sets
`Kind`.

```asm
; AlignTrailingComments: Left
loop:   ld      a, (hl)                         ; read
        ld      (ix+SPRITE_BUFFER_OFFSET), a    ; mirror
        djnz    loop                            ; next

        ld      a, b    ; save
        ret     z       ; done

; AlignTrailingComments: {AlignToTab: false}
loop:   ld      a, (hl)                      ; read
        ld      (ix+SPRITE_BUFFER_OFFSET), a ; mirror
        djnz    loop                         ; next

        ld      a, b ; save
        ret     z    ; done

; AlignTrailingComments: None
loop:   ld      a, (hl) ; read
        ld      (ix+SPRITE_BUFFER_OFFSET), a ; mirror
        djnz    loop ; next

        ld      a, b ; save
        ret     z ; done
```

With `MaxColumn: 80`, a long line does not move the comments of the other
lines:

```asm
        ld      a, b    ; save
        call    print_unsigned_decimal_with_padding_sign_and_thousands_separator ; long
        ret     z       ; done
```

A comment after labels on a line without a mnemonic is aligned with the other
trailing comments of its block. If the block has none and the labels end
before the column of the mnemonic, the comment begins at that column, as if it
were a statement.

```asm
1:      ; shift by modulo-32 bits
        and     a, #0x1f

loop:                   ; assert Y = 0
        ld      a, (hl) ; get high byte
```

### `SpaceAfterComma`

Type: boolean. Default: `true`.

If `true`, one space is placed after each comma in the operands that is not
inside parentheses, brackets or braces, or more if `AlignArrayOfOperands`
aligns the operands. Commas inside strings and comments are not changed, and
neither are a comma at the end of the operands and a comma before a comment or
line continuation. If the brackets of the operands are not balanced, no comma
is changed. This option has no effect in a positional dialect.

```asm
; GNU as for x86, SpaceAfterComma: false
        movl    4(%rax,%rbx,4),%eax
        .byte   1,2,3

; GNU as for x86, SpaceAfterComma: true
        movl    4(%rax,%rbx,4), %eax
        .byte   1, 2, 3
```

### `IndentComments`

Type: boolean. Default: `true`.

If `true`, lines that contain only a comment are indented to `IndentWidth`, or
aligned with the trailing comment that they continue. Comment lines that begin
at column 0 and comments that continue on the following lines are not changed.

```asm
; IndentComments: true
; A comment at column 0
        ; An indented comment
        ret

; IndentComments: false
; A comment at column 0
   ; An indented comment
        ret
```

### `IndentBlocks`

Type: boolean. Default: `true`.

If `true`, the lines inside blocks are indented. A block is a conditional, a
macro or a repetition that the dialect lists, such as `.if` to `.endif` or
`.macro` to `.endm`. The mnemonic and the operands of each line inside a block
move right by `BlockIndentWidth` for each block that encloses the line, and so
do comment lines. The directives that open, divide and close a block stay at
the column of the enclosing level. Labels stay at column 0.

A block opened on a line that begins at column 0 in the result does not indent
its lines, so that macros and conditionals written at the top level keep their
contents at `IndentWidth`. This includes lines that begin with a label, unless
`IndentWidth` is 0. A block that contains a statement kept at column 0, such
as an include guard around definitions at column 0, does not indent its lines
either.

```asm
; IndentBlocks: true
        .irp    idx, .TIM_HANDLER0, .TIM_HANDLER1
                ld      hl, (idx)
                .if     CHECK
                        ld      a, h
                        or      l
                        jp      z, 1$
                .endif
                CALL_HL
        .endm
1$:
        ret

; IndentBlocks: false
        .irp    idx, .TIM_HANDLER0, .TIM_HANDLER1
        ld      hl, (idx)
        .if     CHECK
        ld      a, h
        or      l
        jp      z, 1$
        .endif
        CALL_HL
        .endm
1$:
        ret
```

### `BlockIndentWidth`

Type: unsigned integer. Default: `8`.

The number of columns by which each block indents its lines.

```asm
; BlockIndentWidth: 4
        .irp    idx, .TIM_HANDLER0, .TIM_HANDLER1
            ld      hl, (idx)
            .if     CHECK
                ld      a, h
                or      l
                jp      z, 1$
            .endif
            CALL_HL
        .endm
1$:
        ret
```

### `ContinuationIndentWidth`

Type: unsigned integer. Default: `8`.

The indentation of continued lines, counted from the column of the mnemonic
of the first line. See [Continued lines](#continued-lines).

```asm
; NASM, ContinuationIndentWidth: 8
%define REGS rax, \
        rbx,      \
        rcx_and_more_registers
        db      1, 2, \
                3, 4

; NASM, ContinuationIndentWidth: 4
%define REGS rax, \
    rbx,          \
    rcx_and_more_registers
        db      1, 2, \
            3, 4
```

### `AlignEscapedNewlines`

Type: `None`, `Left` or `LeftWithLastLine`. Default: `Left`.

How the line continuations of continued lines are aligned.

| Value | Meaning |
|---|---|
| `None` | Line continuations are not aligned. One space is placed before each. |
| `Left` | Line continuations are aligned as far left as possible. |
| `LeftWithLastLine` | Line continuations are aligned as far left as possible, also taking the last line into account. |

```asm
; NASM, AlignEscapedNewlines: None
%define REGS rax, \
        rbx, \
        rcx_and_more_registers

; NASM, AlignEscapedNewlines: Left
%define REGS rax, \
        rbx,      \
        rcx_and_more_registers

; NASM, AlignEscapedNewlines: LeftWithLastLine
%define REGS rax,              \
        rbx,                   \
        rcx_and_more_registers
```

### `UseTab`

Type: `Never`, `ForIndentation` or `Always`. Default: `Never`.

Where tab characters are used in the result.

| Value | Meaning |
|---|---|
| `Never` | Tabs are never used. |
| `ForIndentation` | Tabs are used only in the whitespace before the mnemonic, before a comment line and at the start of a continued line. |
| `Always` | Tabs are used in all whitespace that asm-format places, except the padding between operands. |

Tabs are used as long as the next tab stop does not pass the target column.
Whitespace of one column is always a space. With `TabWidth: 8` and the default
columns, `Always` produces the common layout of one tab before the mnemonic
and one tab before the operands.

### `TabWidth`

Type: unsigned integer. Default: `8`.

The number of columns between tab stops. It is used both to count the columns
of tabs in the input and to place tabs in the result.

### `MaxEmptyLinesToKeep`

Type: unsigned integer. Default: `1`.

The maximum number of consecutive empty lines to keep. Empty lines inside
multi-line comments and strings are not counted.

### `KeepEmptyLines`

Type: mapping. Default: `{AtStartOfFile: true, AtEndOfFile: false}`.

Which empty lines are kept. Kept empty lines are still limited by
`MaxEmptyLinesToKeep`.

| Key | Type | Meaning |
|---|---|---|
| `AtStartOfFile` | boolean | Keep the empty lines at the start of the file. |
| `AtEndOfFile` | boolean | Keep the empty lines at the end of the file. |

### `LineEnding`

Type: `Keep`, `LF`, `CRLF`, `DeriveLF` or `DeriveCRLF`. Default: `Keep`.

The line terminators of the result.

| Value | Meaning |
|---|---|
| `Keep` | Each line keeps its line terminator. |
| `LF` | Use LF. |
| `CRLF` | Use CR followed by LF. |
| `DeriveLF` | Use LF, unless more lines of the input end with CR LF. |
| `DeriveCRLF` | Use CR LF, unless more lines of the input end with LF. |

Line terminators inside multi-line strings are not changed.

### `InsertNewlineAtEOF`

Type: boolean. Default: `true`.

If `true`, a line terminator is added at the end of the file if it is
missing. The terminator given by `LineEnding` is used. With `LineEnding: Keep`,
the line terminator of the previous line is used, or LF if the file has only
one line. No terminator is added if the file ends inside a comment or string
that the terminator would become part of.

### `DisableFormat`

Type: boolean. Default: `false`.

If `true`, the files are not formatted. This is useful in `Overrides` to
exclude particular files.

### `Overrides`

Type: list of mappings. Default: empty.

Style options for particular files. Each item has the key `Files` and any
other options except `Overrides`. `Files` is a list of glob patterns, which
are matched against the path of the source file relative to the directory of
the style file, with `/` as the separator. The options of an item apply to the
files that match one of its patterns. Items are applied in order, so a later
item takes precedence over an earlier one.

| Pattern | Matches |
|---|---|
| `*` | Any sequence of bytes except `/`. |
| `?` | Any byte except `/`. |
| `[...]` | Any byte in the set. |
| `**` | Any sequence of path components, as a whole component. |

```yaml
Dialect: sdas
Overrides:
  - Files: ["sm83/*.asm"]
    Dialect: rgbds
  - Files: ["third_party/**"]
    DisableFormat: true
```

## Disabling formatting

Formatting can be turned off for a range of lines with special comments. The
lines from a comment `asm-format off` to a comment `asm-format on`, including
the lines of these comments, are not formatted. The text of a comment is
compared without the comment markers and the surrounding whitespace. Without a
following `asm-format on`, formatting stays off to the end of the file.

```asm
; asm-format off
table:  .db 1,2,3,  4,5,6
; asm-format on
        ld      a, b
```

## Errors

asm-format reports an error and leaves the file unchanged if:

- no dialect is set for the file;
- the style file or the dialect file is invalid;
- the result differs from the input in anything other than whitespace outside
  comments and strings, or removes the whitespace between two words of a line.

The exit status is 0 on success and 1 if an error was reported.

## Limitations

A dialect cannot describe directives that change the syntax in the middle of a
file, such as `.feature` in ca65 or `ICTL` in HLASM. A file that uses them
needs its own dialect, selected with `Overrides`, or `asm-format off` comments
around the affected part.
