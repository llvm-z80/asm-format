# asm-format

A **universal** assembly formatter.

## Supported assemblers

| Assembler | Dialect |
|---|---|
| GNU as for x86 | `gas-x86` |
| GNU as for ARM | `gas-arm` |
| GNU as for AArch64 | `gas-aarch64` |
| GNU as for RISC-V | `gas-riscv` |
| NASM, YASM | `nasm` |
| MASM and compatible assemblers | `masm` |
| flat assembler | `fasm` |
| Go assembler | `go` |
| SDCC sdas (sdasz80, sdasgb, ...) | `sdas` |
| ASxxxx | `asxxxx` |
| RGBDS rgbasm | `rgbds` |
| cc65 ca65 | `ca65` |
| z88dk z80asm | `z88dk` |

Other assemblers can be added with a dialect file.

## Example

```yaml
# .asm-format
Dialect: gas-x86
```

```sh
asm-format -i src/*.S
```

Input:

```asm
sum: xorl %eax,%eax # total = 0
loop: addl (%rdi),%eax
  addq $4,%rdi # next element
  decq %rsi
  jnz loop
  ret
```

Output:

```asm
sum:    xorl    %eax,   %eax    # total = 0
loop:   addl    (%rdi), %eax
        addq    $4,     %rdi    # next element
        decq    %rsi
        jnz     loop
        ret
```

## Building

```sh
cargo build --release
```

Rust 1.89 or later is required.

## Documentation

- [Usage.md](docs/Usage.md): command line, style files and formatting rules.
- [Dialects.md](docs/Dialects.md): dialect file format.

## License

Licensed under either Apache-2.0 ([LICENSE-APACHE](LICENSE-APACHE)) or MIT
([LICENSE-MIT](LICENSE-MIT)), at your option.
