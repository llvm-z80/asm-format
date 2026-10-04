# Dialect Files

A dialect file describes the syntax of an assembler. asm-format uses it to
divide each line into fields and to find the bytes that must not change.
Layout settings are not part of a dialect. They are set in the style file
described in [Usage.md](Usage.md), which also describes how the fields of a
statement are placed.

## File

A dialect file is a TOML 1.0 document. The name of the dialect is the file
name without the `.toml` extension. Unknown keys are errors.

| Key | Type | Default |
|---|---|---|
| `comment` | array of entries | `[]` |
| `literal` | array of entries | `[]` |
| `label` | array of entries | `[]` |
| `separator` | array of entries | `[]` |
| `continuation` | array of entries | `[]` |
| `mnemonic` | pattern | `'\S+'` |
| `positional` | boolean | `false` |
| `operands` | array of tables | `[]` |
| `block` | array of tables | `[]` |
| `directive` | array of strings | `[]` |

## Lines

A line is a sequence of bytes ended by a line terminator or by the end of the
file. A line terminator is LF, CR followed by LF, or CR not followed by LF.
In this document, whitespace means the bytes space (0x20) and tab (0x09).
Columns are counted as described in [Usage.md](Usage.md#columns), except for
the `column` key of a continuation.

## Patterns

A pattern is a regular expression in the syntax of the Rust
[`regex`](https://docs.rs/regex) crate.

- Patterns match bytes. Unicode support is disabled, so `\w`, `\s`, `\b`
  and `(?i)` are limited to ASCII.
- A pattern is matched against one line without its line terminator. `^`
  matches only at the beginning of the line and `$` only at its end.
- `\b` and `\B` examine the byte before the position where matching starts.
  For example, `\B'` does not match the `'` in `af'`.
- An `end` pattern can refer to a capture group of its `begin` pattern; see
  [Delimited tokens](#delimited-tokens).

TOML literal strings (`'...'`) keep backslashes unchanged. A pattern that
contains `'` can be written as a multi-line literal string (`'''...'''`).

## Entries

An entry is a string or a table. A string is a shorthand for a table:

| Key | String | Equivalent table |
|---|---|---|
| `comment` | `';'` | `{ begin = ';' }` |
| `literal` | `'"'` | `{ begin = '"', end = '"' }` |
| `label`, `separator`, `continuation` | `'::'` | `{ begin = '::' }` |

Table keys:

| Key | Type | Used in | Description |
|---|---|---|---|
| `begin` | pattern | all | The start of the token. Required unless `column` is given. |
| `end` | pattern | `comment`, `literal` | The end of the token. |
| `escape` | string of one byte | `comment`, `literal` | A byte that, together with the byte after it, is skipped during the search for `end`. |
| `multiline` | boolean | `comment`, `literal` | If `true`, `end` can be found on a later line. Default `false`. |
| `nest` | boolean | `comment`, `literal` | If `true`, each inner match of `begin` needs its own `end`. Default `false`. |
| `before` | array of strings | `label` | The words that must follow the label. |
| `column` | integer | `continuation` | The column of the continuation byte. |

## Comments

A comment begins where `begin` matches. Without `end`, it extends to the end
of the line, without the whitespace at the end of the line. With `end`, it is
a [delimited token](#delimited-tokens).

```toml
comment = [';', '^\*', { begin = '/\*', end = '\*/', multiline = true }]
```

## Literals

A literal is a string or character constant. Without `end`, the token is the
text matched by `begin`. With `end`, it is a
[delimited token](#delimited-tokens).

```toml
literal = [
    { begin = '"', end = '"', escape = '\' },   # "a\"b"
    { begin = '''\B'.''' },                     # 'A
]
```

## Delimited tokens

A comment or literal entry with `end` produces a delimited token.

- The search for `end` begins at the end of the `begin` match.
- In `end`, `\1` to `\9` stand for the text matched by the corresponding
  capture group of `begin`, matched literally. A group that did not take part
  in the match stands for the empty string.
- With `escape`, each occurrence of the escape byte and the byte after it are
  skipped during the search.
- With `nest = true`, each match of `begin` found during the search must be
  closed by its own match of `end` before the token can end. Where both
  patterns match at the same position, `end` is used. `\1` to `\9` refer to
  the first `begin` match.
- Without `multiline`, the entry does not match if `end` is not found on the
  same line. With `multiline = true`, the search continues on the following
  lines. If `end` is not found before the end of the file, the token extends
  to the end of the file.

An entry that accepts any delimiter:

```toml
{ begin = '(\S)', end = '\1' }   # /text/  |text|
```

## Labels

Label entries are tried only at label positions, described in
[Statements](#statements). The token is the text matched by `begin`,
including any colons.

With `before`, the entry matches only if the label is followed by whitespace
and then by one of the listed words. Words are compared without regard to
ASCII case. A word ends at whitespace, at the end of the line, or where a
comment, literal, separator or continuation token begins.

```toml
label = ['[\w.$]+::?', { begin = '[\w.$]+', before = ['db', 'equ'] }]
```

## Separators

A separator divides a line into statements. The token is the text matched by
`begin`.

```toml
separator = ['::']
```

## Continuations

A continuation joins the next line to the current statement. The next line is
formatted as described in [Usage.md](Usage.md#continued-lines).

With `begin`, the token is the text matched by `begin`. The entry matches
only if the rest of the line contains nothing but whitespace and comment
tokens.

```toml
continuation = ['\\']
```

With `column`, a line continues if the byte at that column is not whitespace.
The column is numbered from 1. In every line, the bytes from that column to
the end of the line keep their columns, and continued lines are not changed.

```toml
continuation = [{ column = 72 }]
```

## Mnemonic

`mnemonic` is the pattern that starts the mnemonic field. The field is
described in [Statements](#statements).

```toml
mnemonic = '(?i)(?:(?:lock|rep\w*)\s+)*\S+'   # lock xadd, rep movsb
```

## Positional syntax

If `positional` is `true`, the operand field ends at the first whitespace
byte outside tokens, and the rest of the statement is the trailing comment.
asm-format does not change whitespace inside the operand field of a
positional dialect.

```toml
positional = true   # LDA #1 comment
```

## Operand rules

An `[[operands]]` table adds comment and literal entries for the operand
field of particular mnemonics.

| Key | Type | Description |
|---|---|---|
| `of` | array of strings | The mnemonics. Required. Compared without regard to ASCII case. |
| `consecutive` | boolean | If `true`, the entries are tried again after each token they produce. Default `false`. |
| `comment` | array of entries | The comment entries. |
| `literal` | array of entries | The literal entries. |

The entries are tried at the first byte of the operand field. With
`consecutive = true`, they are also tried after each token produced by one of
them and any whitespace that follows it. Where the entries of a table are
tried and one of them matches, the entries of the dialect are not used.

```toml
[[operands]]
of = ['.title', '.sbttl']
literal = [{ begin = '\S.*' }]   # the rest of the line, including ;

[[operands]]
of = ['rts', 'nop']
comment = ['\S']                 # text after rts is a comment
```

## Blocks

A `[[block]]` table names the directives that open and close a block, such as
a conditional or a macro definition. The lines inside a block are indented as
described for
[`IndentBlocks`](Usage.md#indentblocks).

| Key | Type | Description |
|---|---|---|
| `begin` | array of strings | The mnemonics that open the block. Required. |
| `middle` | array of strings | The mnemonics that divide the block, such as `.else`. |
| `end` | array of strings | The mnemonics that close the block. Required. |

Words are compared with the mnemonic without regard to ASCII case. A word
that ends with `*` matches every mnemonic that begins with the rest of the
word. A word that contains a space matches the mnemonic followed by the first
word of the operands.

A `middle` or `end` word applies to the innermost open block of the same
table. A word that matches no open block is ignored.

```toml
[[block]]
begin = ['.if*']
middle = ['.elseif', '.else']
end = ['.endif']

[[block]]
begin = ['if']
middle = ['else']
end = ['end if']                 # flat assembler
```

## Directives

`directive` lists the mnemonics of directives. The operands of a directive
begin at
[`DirectiveOperandColumn`](Usage.md#directiveoperandcolumn) instead of
`OperandColumn`. The words are compared with the mnemonic in the same way as
the words of a block.

```toml
directive = ['.*']       # every mnemonic that begins with a period
```

## Lexical analysis

asm-format divides each line into tokens and plain bytes. At each position
that is not inside a token and does not hold whitespace, it tries these
entries:

- the `comment`, `literal`, `separator` and `continuation` entries of the
  dialect;
- the `label` entries, at label positions;
- the entries of an `[[operands]]` table, where they apply.

If an entry of an `[[operands]]` table matches, only the entries of that
table are considered. If no entry matches, the byte at the position is a
plain byte. Otherwise the entry with the longest token is chosen. The length
of a multi-line token includes its bytes on the following lines. Among tokens
of equal length, the choice is made by these rules, in order:

1. Kind: comment, literal, label, continuation, separator.
2. Order in the file.

## Statements

A statement begins at the start of a line or after a separator token. It ends
at the next separator token or at the end of the line. It consists of the
following parts, each of which can be absent.

1. Labels. The first byte after the start of the statement and any whitespace
   is a label position. After a label token and any whitespace, the next byte
   is again a label position.
2. Mnemonic field. It begins at the first byte after the labels and any
   whitespace, unless a comment, separator or continuation token begins
   there. It is the text matched by the `mnemonic` pattern at that byte, or
   the text up to the next whitespace if the pattern does not match. It does
   not end inside a literal token and does not include a comment, separator
   or continuation token.
3. Operand field. It begins at the first byte after the mnemonic field and
   any whitespace. It ends at the end of the statement, excluding the
   trailing comment and the whitespace before it. In a positional dialect, it
   ends at the first whitespace byte outside tokens.
4. Trailing comment. A comment token after which the line contains only
   whitespace. In a positional dialect, it is the rest of the statement after
   the operand field and the whitespace after it.

An `[[operands]]` table applies to the operand field when one of the strings
in `of` is equal to the mnemonic field, compared without regard to ASCII
case.

## Examples

### GNU as for x86

```toml
comment = [
    '#',
    '//',
    { begin = '^#.*\\$', end = '(?:^|[^\\])$', multiline = true },   # #define ... \
    { begin = '/\*', end = '\*/', multiline = true },
    '^/(?:[^*]|$)',                                                     # / in the first column
]
literal = [
    { begin = '"', end = '"', escape = '\' },
    { begin = '''\B'(?:\\(?:[0-7]{1,3}|[xX][0-9A-Fa-f]+|.)|.)'?''' },
]
label = ['(?:[\w.$]+|"(?:[^"\\]|\\.)*"):']                              # a:  1:  .L1:  "a b":
separator = [';']
continuation = ['\\']                                                   # cpp in .S files
mnemonic = '(?i)(?:(?:\{\w+\}|rep(?:n?[ez])?|lock|notrack|bnd|xacquire|xrelease|data(?:16|32)|addr(?:16|32)|rex(?:\.\w+)?)\s+)*\S+'
directive = ['.*']

[[block]]
begin = ['.if*']
middle = ['.elseif', '.else']
end = ['.endif']

[[block]]
begin = ['.macro']
end = ['.endm']

[[block]]
begin = ['.rept', '.irp', '.irpc']
end = ['.endr']
```

### GNU as for RISC-V

```toml
comment = [
    '#',
    '//',
    { begin = '^#.*\\$', end = '(?:^|[^\\])$', multiline = true },   # #define ... \
    { begin = '/\*', end = '\*/', multiline = true },
]
literal = [
    { begin = '"', end = '"', escape = '\' },
    { begin = '''\B'(?:\\(?:[0-7]{1,3}|[xX][0-9A-Fa-f]+|.)|.)'?''' },
]
label = ['(?:[\w.$]+|"(?:[^"\\]|\\.)*"):']                              # a:  1:  .L1:  "a b":
separator = [';']
continuation = ['\\']                                                   # cpp in .S files
directive = ['.*']

[[block]]
begin = ['.if*']
middle = ['.elseif', '.else']
end = ['.endif']

[[block]]
begin = ['.macro']
end = ['.endm']

[[block]]
begin = ['.rept', '.irp', '.irpc']
end = ['.endr']
```

### SDCC sdas

The assemblers of SDCC, such as sdasz80 and sdasgb.

```toml
comment = [';']
literal = [
    { begin = '''\B'(?:\\(?:[0-7]{1,3}|.)|.)''' },     # 'A  '\n  '\001
    { begin = '"(?:\\(?:[0-7]{1,3}|.)|.){2}' },        # "AB
]
label = ['[\w.$]+::?']                                 # a:  a::  10$:
mnemonic = '[\w.$]+'                                   # .ascii/text/
directive = ['.*']

[[block]]
begin = [
    '.if', '.ifeq', '.ifne', '.ifgt', '.iflt', '.ifge', '.ifle',
    '.ifdef', '.ifndef', '.ifb', '.ifnb', '.ifidn', '.ifdif',
]
middle = ['.else', '.iff', '.ift', '.iftf']
end = ['.endif']

[[block]]
begin = ['.macro', '.irp', '.irpc', '.rept']
end = ['.endm']

[[operands]]
of = ['.ascii', '.asciz', '.ascis', '.str', '.strz', '.strs', '.fcc']
literal = [{ begin = '(\S)', end = '\1' }]             # /text/  ;text;

[[operands]]
of = ['.include', '.incbin', '.msg']
literal = [
    { begin = '\^(.)', end = '\1' },                   # ^/text/
    { begin = '([^\s;^])', end = '\1' },               # /text/
]

[[operands]]
of = ['.define']
literal = [
    { begin = '[\w.$]+\s*,?\s*\^(.)', end = '\1' },    # kw ^/text/
    { begin = '[\w.$]+(?:\s*,\s*|\s+)([^\s;^])', end = '\1' },
    { begin = '[\w.$]+([^\w.$\s;^,])', end = '\1' },   # kw/text/
]

[[operands]]
of = ['.title', '.sbttl']
literal = [{ begin = '\S.*' }]                         # the rest of the line
```

### ASxxxx

ASxxxx assemblers accept several segments in a string directive. The rest of
the dialect is the same as for SDCC sdas.

```toml
# .asciz /Hello/(13)(10)/World/
[[operands]]
of = ['.ascii', '.asciz', '.ascis', '.str', '.strz', '.strs', '.fcc']
consecutive = true
literal = [
    { begin = '\(', end = '\)', nest = true },   # (13)
    { begin = '\^(.)', end = '\1' },             # ^/text/
    { begin = '([^\s(;^])', end = '\1' },        # /text/
]
```

### RGBDS

```toml
comment = [';', { begin = '/\*', end = '\*/', multiline = true }]
literal = [
    { begin = '"', end = '"', escape = '\', multiline = true },   # "a\ newline b"
    { begin = '"""', end = '"""', escape = '\', multiline = true },
    { begin = '#"', end = '"' },                                  # raw string
    { begin = '#"""', end = '"""', multiline = true },
    { begin = "'", end = "'", escape = '\' },
    { begin = '\\<[^>]*>' },                                      # \<10>
    { begin = '\\\S' },                                           # \1  \@  \#  \,
]
label = [
    # Label:  Label::  .local:  :
    '(?:[\w.@#$]|\{[^}]*\}|\\(?:[1-9@#]|<[^>]*>))*::?',
    # .local  Parent.local
    '(?:[\w@#$]|\{[^}]*\}|\\(?:[1-9@#]|<[^>]*>))*\.(?:[\w@#$]|\{[^}]*\}|\\(?:[1-9@#]|<[^>]*>))+',
]
separator = ['::']
continuation = ['\\']
directive = [
    'db', 'dw', 'dl', 'ds', 'def', 'redef', 'section', 'endsection', 'load', 'endl',
    'union', 'nextu', 'endu', 'include', 'incbin', 'export', 'purge', 'charmap',
    'newcharmap', 'setcharmap', 'pushc', 'popc', 'pushs', 'pops', 'pusho', 'popo',
    'opt', 'rsreset', 'rsset', 'shift', 'fail', 'warn', 'assert', 'static_assert',
    'print*', 'align', 'if', 'elif', 'else', 'endc', 'macro', 'endm', 'rept', 'for',
    'endr', 'break',
]

[[block]]
begin = ['if']
middle = ['elif', 'else']
end = ['endc']

[[block]]
begin = ['macro']
end = ['endm']

[[block]]
begin = ['rept', 'for']
end = ['endr']
```
