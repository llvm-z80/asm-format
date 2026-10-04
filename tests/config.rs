use std::path::Path;

use asm_format::{
    Alignment, Config, Dialect, EscapedNewlines, LineEnding, OperandAlignment, UseTab, format,
};

fn config(yaml: &str) -> Result<Config, String> {
    Config::from_yaml(yaml, Path::new("/project")).map_err(|e| e.to_string())
}

#[track_caller]
fn error(yaml: &str, message: &str) {
    match config(yaml) {
        Ok(_) => panic!("no error for {yaml:?}"),
        Err(e) => assert!(e.contains(message), "{e:?} does not contain {message:?}"),
    }
}

#[test]
fn options() {
    let style =
        config("{Dialect: rgbds, IndentWidth: 4, UseTab: Always, InsertNewlineAtEOF: false}")
            .unwrap()
            .style_for(None);
    assert_eq!(style.dialect.as_deref(), Some("rgbds"));
    assert_eq!(style.indent_width, 4);
    assert_eq!(style.operand_column, 16);
    assert_eq!(style.use_tab, UseTab::Always);
    assert!(!style.insert_newline_at_eof);
}

#[test]
fn overrides() {
    let c = config(
        "Dialect: sdas\n\
         Overrides:\n\
         \x20 - Files: [\"sm83/*.asm\"]\n\
         \x20   Dialect: rgbds\n\
         \x20 - Files: [\"third_party/**\"]\n\
         \x20   DisableFormat: true\n",
    )
    .unwrap();
    let style = |p: &str| c.style_for(Some(Path::new(p)));
    assert_eq!(style("/project/sm83/a.asm").dialect.unwrap(), "rgbds");
    assert_eq!(style("/project/sm83/x/a.asm").dialect.unwrap(), "sdas");
    assert_eq!(style("/project/z80/a.asm").dialect.unwrap(), "sdas");
    assert!(style("/project/third_party/a/b.s").disable_format);
    assert!(!c.style_for(None).disable_format);
}

#[test]
fn errors() {
    error("{Dialect: sdas, Indent: 4}", "unknown field `Indent`");
    error("{Files: [a]}", "Files is only allowed in Overrides");
    error(
        "{Overrides: [{IndentWidth: 4}]}",
        "each item of Overrides needs Files",
    );
    error(
        "{Overrides: [{Files: [a], Overrides: []}]}",
        "Overrides cannot be nested",
    );
    error("{TabWidth: 0}", "TabWidth must be at least 1");
    error("{UseTab: Sometimes}", "Sometimes");
}

#[test]
fn unknown_dialect() {
    let style = config("{Dialect: nope}").unwrap().style_for(None);
    let e = style.load_dialect().err().unwrap().to_string();
    assert!(e.contains("unknown dialect 'nope'"));
}

#[test]
fn nested_options() {
    let style = config(
        "{AlignTrailingComments: {Kind: None, MaxColumn: 64}, \
         AlignConsecutiveOperands: None, AlignConsecutiveMnemonics: {MaxPadding: 32}, \
         AlignEscapedNewlines: None, KeepEmptyLines: {AtEndOfFile: true}, LineEnding: DeriveCRLF}",
    )
    .unwrap()
    .style_for(None);
    let t = style.align_trailing_comments;
    assert_eq!(t.kind, Alignment::None);
    assert_eq!(t.max_column, 64);
    assert!(t.align_to_tab);
    assert_eq!(style.align_consecutive_operands.kind, Alignment::None);
    assert_eq!(style.align_consecutive_operands.max_padding, 4);
    assert_eq!(style.align_consecutive_mnemonics.kind, Alignment::Left);
    assert_eq!(style.align_consecutive_mnemonics.max_padding, 32);
    assert_eq!(style.align_escaped_newlines, EscapedNewlines::None);
    assert!(style.keep_empty_lines.at_end_of_file);
    assert!(style.keep_empty_lines.at_start_of_file);
    assert_eq!(style.line_ending, LineEnding::DeriveCrlf);
    let style = config("{IndentBlocks: false, BlockIndentWidth: 4}")
        .unwrap()
        .style_for(None);
    assert!(!style.indent_blocks);
    assert_eq!(style.block_indent_width, 4);
    let style = config("{AlignArrayOfOperands: None}")
        .unwrap()
        .style_for(None);
    assert_eq!(style.align_array_of_operands.kind, OperandAlignment::None);
    assert_eq!(style.align_array_of_operands.max_padding, 4);
    let style = config("{AlignArrayOfOperands: {MaxPadding: 8}}")
        .unwrap()
        .style_for(None);
    assert_eq!(style.align_array_of_operands.kind, OperandAlignment::Left);
    assert_eq!(style.align_array_of_operands.max_padding, 8);
    let style = config("{AlignTrailingComments: None}")
        .unwrap()
        .style_for(None);
    assert_eq!(style.align_trailing_comments.kind, Alignment::None);
}

#[test]
fn option_errors() {
    error("{AlignTrailingComments: {Kind: Leave}}", "Leave");
    error(
        "{AlignTrailingComments: {Column: 1}}",
        "unknown field `Column`",
    );
    error("{AlignEscapedNewlines: Right}", "Right");
    error("{AlignArrayOfOperands: Center}", "Center");
    error("{AlignArrayOfOperands: true}", "boolean");
    error("{AlignTrailingComments: false}", "boolean");
    error("{AlignEscapedNewlines: DontAlign}", "DontAlign");
    error(
        "{AlignConsecutiveOperands: {Enabled: true}}",
        "unknown field `Enabled`",
    );
    error("{LineEnding: CR}", "CR");
    error("{BasedOnStyle: LLVM}", "unknown BasedOnStyle 'LLVM'");
    error(
        "{Overrides: [{Files: [a], BasedOnStyle: InheritParentConfig}]}",
        "BasedOnStyle is not allowed in Overrides",
    );
}

#[test]
fn dump() {
    let dump = config("{Dialect: sdas, CommentColumn: 28}")
        .unwrap()
        .style_for(None)
        .dump();
    assert_eq!(
        dump,
        "---\n\
         AlignArrayOfOperands:\n  Kind: Left\n  MaxPadding: 4\n  Numbers: Left\n\
         AlignConsecutiveMnemonics:\n  AcrossComments: false\n  AcrossEmptyLines: false\n  \
         Kind: Left\n  MaxPadding: 0\n\
         AlignConsecutiveOperands:\n  AcrossComments: false\n  AcrossEmptyLines: false\n  \
         Kind: Left\n  MaxPadding: 4\n\
         AlignEscapedNewlines: Left\n\
         AlignTrailingComments:\n  AlignToTab: true\n  Kind: Left\n  MaxColumn: 80\n  \
         OverEmptyLines: 0\n\
         BlockIndentWidth: 8\n\
         CommentColumn: 28\nContinuationIndentWidth: 8\nDialect: sdas\n\
         DirectiveOperandColumn: 16\nDisableFormat: false\n\
         IndentBlocks: true\nIndentComments: true\nIndentWidth: 8\nInsertNewlineAtEOF: true\n\
         KeepEmptyLines:\n  AtEndOfFile: false\n  AtStartOfFile: true\n\
         LineEnding: Keep\nMaxEmptyLinesToKeep: 1\nOperandColumn: 16\nSpaceAfterComma: true\n\
         SpacesBeforeTrailingComments: 1\nTabWidth: 8\nUseTab: Never\n...\n"
    );
    let reparsed = Config::from_yaml(&dump, Path::new("/project"))
        .unwrap()
        .style_for(None)
        .dump();
    assert_eq!(reparsed, dump);
}

#[test]
fn dialect_errors() {
    let e = |text: &str| Dialect::parse(text).err().unwrap().to_string();
    assert!(
        e("comment = [{ begin = ';', ending = 'x' }]").contains("comment[0].ending: unknown key")
    );
    assert!(e("literal = [{ begin = '\"', end = '\\2' }]").contains("has no group 2"));
    assert!(e("continuation = [{ column = 0 }]").contains("positive integer"));
    assert!(e("label = ['(']").contains("label[0]"));
    assert!(e("[[block]]\nbegin = ['.if']").contains("block[0]: `end` is required"));
    assert!(
        e("[[block]]\nbegin = ['.if']\nend = ['.endif']\nelse = ['.else']")
            .contains("block[0].else: unknown key")
    );
}

#[test]
fn preservation_check() {
    let d = Dialect::parse("literal = [{ begin = ', ' }]").unwrap();
    let style = config("{}").unwrap().style_for(None);
    let e = format(b"  db 1,2\n", &d, &style).err().unwrap().to_string();
    assert!(e.contains("differs from the input"));
}

#[test]
fn builtin_dialects() {
    for name in Dialect::builtin_names() {
        assert!(Dialect::builtin(name).is_some());
    }
}
