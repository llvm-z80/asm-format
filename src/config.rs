use std::fmt::{self, Write};
use std::marker::PhantomData;
use std::path::{Path, PathBuf};

use globset::{GlobBuilder, GlobMatcher};
use serde::de::value::{MapAccessDeserializer, StrDeserializer};
use serde::de::{MapAccess, Visitor};
use serde::{Deserialize, Deserializer};

use crate::{Dialect, Error};

const STYLE_FILES: [&str; 4] = [
    ".asm-format",
    "asm-format",
    ".asm-format.yaml",
    "asm-format.yaml",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub enum UseTab {
    Never,
    ForIndentation,
    Always,
}

/// Whether something is aligned: `None` turns the alignment off.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub enum Alignment {
    None,
    Left,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrailingComments {
    pub kind: Alignment,
    pub over_empty_lines: usize,
    pub align_to_tab: bool,
    pub max_column: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AlignConsecutive {
    pub kind: Alignment,
    pub across_empty_lines: bool,
    pub across_comments: bool,
    pub max_padding: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub enum OperandAlignment {
    None,
    Left,
    Right,
    RightFirst,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArrayOfOperands {
    pub kind: OperandAlignment,
    pub max_padding: usize,
    pub numbers: NumberAlignment,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub enum NumberAlignment {
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub enum EscapedNewlines {
    None,
    Left,
    LeftWithLastLine,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeepEmptyLines {
    pub at_start_of_file: bool,
    pub at_end_of_file: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub enum LineEnding {
    Keep,
    #[serde(rename = "LF")]
    Lf,
    #[serde(rename = "CRLF")]
    Crlf,
    #[serde(rename = "DeriveLF")]
    DeriveLf,
    #[serde(rename = "DeriveCRLF")]
    DeriveCrlf,
}

#[derive(Clone, Debug)]
pub struct Style {
    pub dialect: Option<String>,
    pub indent_width: usize,
    pub operand_column: usize,
    pub directive_operand_column: Option<usize>,
    pub comment_column: usize,
    pub continuation_indent_width: usize,
    pub indent_blocks: bool,
    pub block_indent_width: usize,
    pub spaces_before_trailing_comments: usize,
    pub align_trailing_comments: TrailingComments,
    pub align_consecutive_mnemonics: AlignConsecutive,
    pub align_consecutive_operands: AlignConsecutive,
    pub align_array_of_operands: ArrayOfOperands,
    pub align_escaped_newlines: EscapedNewlines,
    pub space_after_comma: bool,
    pub indent_comments: bool,
    pub use_tab: UseTab,
    pub tab_width: usize,
    pub max_empty_lines_to_keep: usize,
    pub keep_empty_lines: KeepEmptyLines,
    pub line_ending: LineEnding,
    pub insert_newline_at_eof: bool,
    pub disable_format: bool,
    pub dialect_dir: PathBuf,
}

impl Default for Style {
    fn default() -> Self {
        let consecutive = |max_padding| AlignConsecutive {
            kind: Alignment::Left,
            across_empty_lines: false,
            across_comments: false,
            max_padding,
        };
        Style {
            dialect: None,
            indent_width: 8,
            operand_column: 16,
            directive_operand_column: None,
            comment_column: 0,
            continuation_indent_width: 8,
            indent_blocks: true,
            block_indent_width: 8,
            spaces_before_trailing_comments: 1,
            align_trailing_comments: TrailingComments {
                kind: Alignment::Left,
                over_empty_lines: 0,
                align_to_tab: true,
                max_column: 80,
            },
            align_consecutive_mnemonics: consecutive(0),
            align_consecutive_operands: consecutive(4),
            align_array_of_operands: ArrayOfOperands {
                kind: OperandAlignment::Left,
                max_padding: 4,
                numbers: NumberAlignment::Left,
            },
            align_escaped_newlines: EscapedNewlines::Left,
            space_after_comma: true,
            indent_comments: true,
            use_tab: UseTab::Never,
            tab_width: 8,
            max_empty_lines_to_keep: 1,
            keep_empty_lines: KeepEmptyLines {
                at_start_of_file: true,
                at_end_of_file: false,
            },
            line_ending: LineEnding::Keep,
            insert_newline_at_eof: true,
            disable_format: false,
            dialect_dir: PathBuf::from("."),
        }
    }
}

/// A mapping of options that can also be given as the value of its `Kind`.
trait Shorthand: Sized {
    fn from_str<E: serde::de::Error>(value: &str) -> Result<Self, E>;
}

fn kind<'de, K: Deserialize<'de>, E: serde::de::Error>(value: &'de str) -> Result<K, E> {
    K::deserialize(StrDeserializer::<E>::new(value))
}

struct Short<T>(T);

impl<'de, T: Deserialize<'de> + Shorthand> Deserialize<'de> for Short<T> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V<T>(PhantomData<T>);
        impl<'de, T: Deserialize<'de> + Shorthand> Visitor<'de> for V<T> {
            type Value = T;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a mapping or a value of Kind")
            }
            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<T, E> {
                T::from_str(v)
            }
            fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<T, A::Error> {
                T::deserialize(MapAccessDeserializer::new(map))
            }
        }
        d.deserialize_any(V(PhantomData)).map(Short)
    }
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "PascalCase")]
struct TrailingCommentsOptions {
    kind: Option<Alignment>,
    over_empty_lines: Option<usize>,
    align_to_tab: Option<bool>,
    max_column: Option<usize>,
}

impl Shorthand for TrailingCommentsOptions {
    fn from_str<E: serde::de::Error>(value: &str) -> Result<Self, E> {
        Ok(TrailingCommentsOptions {
            kind: Some(kind(value)?),
            ..Default::default()
        })
    }
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "PascalCase")]
struct AlignConsecutiveOptions {
    kind: Option<Alignment>,
    across_empty_lines: Option<bool>,
    across_comments: Option<bool>,
    max_padding: Option<usize>,
}

impl Shorthand for AlignConsecutiveOptions {
    fn from_str<E: serde::de::Error>(value: &str) -> Result<Self, E> {
        Ok(AlignConsecutiveOptions {
            kind: Some(kind(value)?),
            ..Default::default()
        })
    }
}

impl AlignConsecutiveOptions {
    fn apply(&self, a: &mut AlignConsecutive) {
        set(&mut a.kind, self.kind);
        set(&mut a.across_empty_lines, self.across_empty_lines);
        set(&mut a.across_comments, self.across_comments);
        set(&mut a.max_padding, self.max_padding);
    }
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "PascalCase")]
struct ArrayOfOperandsOptions {
    kind: Option<OperandAlignment>,
    max_padding: Option<usize>,
    numbers: Option<NumberAlignment>,
}

impl Shorthand for ArrayOfOperandsOptions {
    fn from_str<E: serde::de::Error>(value: &str) -> Result<Self, E> {
        Ok(ArrayOfOperandsOptions {
            kind: Some(kind(value)?),
            ..Default::default()
        })
    }
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "PascalCase")]
struct KeepEmptyLinesOptions {
    at_start_of_file: Option<bool>,
    at_end_of_file: Option<bool>,
}

fn set<T>(field: &mut T, value: Option<T>) {
    if let Some(v) = value {
        *field = v;
    }
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "PascalCase")]
struct Options {
    based_on_style: Option<String>,
    dialect: Option<String>,
    indent_width: Option<usize>,
    operand_column: Option<usize>,
    directive_operand_column: Option<usize>,
    comment_column: Option<usize>,
    continuation_indent_width: Option<usize>,
    indent_blocks: Option<bool>,
    block_indent_width: Option<usize>,
    spaces_before_trailing_comments: Option<usize>,
    align_trailing_comments: Option<Short<TrailingCommentsOptions>>,
    align_consecutive_mnemonics: Option<Short<AlignConsecutiveOptions>>,
    align_consecutive_operands: Option<Short<AlignConsecutiveOptions>>,
    align_array_of_operands: Option<Short<ArrayOfOperandsOptions>>,
    align_escaped_newlines: Option<EscapedNewlines>,
    space_after_comma: Option<bool>,
    indent_comments: Option<bool>,
    use_tab: Option<UseTab>,
    tab_width: Option<usize>,
    max_empty_lines_to_keep: Option<usize>,
    keep_empty_lines: Option<KeepEmptyLinesOptions>,
    line_ending: Option<LineEnding>,
    #[serde(rename = "InsertNewlineAtEOF")]
    insert_newline_at_eof: Option<bool>,
    disable_format: Option<bool>,
    files: Option<Vec<String>>,
    overrides: Option<Vec<Options>>,
}

impl Options {
    fn apply(&self, s: &mut Style, dir: &Path) {
        if let Some(v) = &self.dialect {
            s.dialect = Some(v.clone());
            s.dialect_dir = dir.to_path_buf();
        }
        set(&mut s.indent_width, self.indent_width);
        set(&mut s.operand_column, self.operand_column);
        if self.directive_operand_column.is_some() {
            s.directive_operand_column = self.directive_operand_column;
        }
        set(&mut s.comment_column, self.comment_column);
        set(
            &mut s.continuation_indent_width,
            self.continuation_indent_width,
        );
        set(&mut s.indent_blocks, self.indent_blocks);
        set(&mut s.block_indent_width, self.block_indent_width);
        set(
            &mut s.spaces_before_trailing_comments,
            self.spaces_before_trailing_comments,
        );
        if let Some(Short(o)) = &self.align_trailing_comments {
            let t = &mut s.align_trailing_comments;
            set(&mut t.kind, o.kind);
            set(&mut t.over_empty_lines, o.over_empty_lines);
            set(&mut t.align_to_tab, o.align_to_tab);
            set(&mut t.max_column, o.max_column);
        }
        if let Some(Short(o)) = &self.align_consecutive_mnemonics {
            o.apply(&mut s.align_consecutive_mnemonics);
        }
        if let Some(Short(o)) = &self.align_consecutive_operands {
            o.apply(&mut s.align_consecutive_operands);
        }
        if let Some(Short(o)) = &self.align_array_of_operands {
            set(&mut s.align_array_of_operands.kind, o.kind);
            set(&mut s.align_array_of_operands.max_padding, o.max_padding);
            set(&mut s.align_array_of_operands.numbers, o.numbers);
        }
        set(&mut s.align_escaped_newlines, self.align_escaped_newlines);
        set(&mut s.space_after_comma, self.space_after_comma);
        set(&mut s.indent_comments, self.indent_comments);
        set(&mut s.use_tab, self.use_tab);
        set(&mut s.tab_width, self.tab_width);
        set(&mut s.max_empty_lines_to_keep, self.max_empty_lines_to_keep);
        if let Some(o) = &self.keep_empty_lines {
            set(&mut s.keep_empty_lines.at_start_of_file, o.at_start_of_file);
            set(&mut s.keep_empty_lines.at_end_of_file, o.at_end_of_file);
        }
        set(&mut s.line_ending, self.line_ending);
        set(&mut s.insert_newline_at_eof, self.insert_newline_at_eof);
        set(&mut s.disable_format, self.disable_format);
    }

    fn validate(&self, top: bool) -> Result<(), String> {
        if self.tab_width == Some(0) {
            return Err("TabWidth must be at least 1".into());
        }
        if let Some(name) = &self.based_on_style {
            if !top {
                return Err("BasedOnStyle is not allowed in Overrides".into());
            }
            if !name.eq_ignore_ascii_case("InheritParentConfig") {
                return Err(format!("unknown BasedOnStyle '{name}'"));
            }
        }
        if top && self.files.is_some() {
            return Err("Files is only allowed in Overrides".into());
        }
        if !top && self.overrides.is_some() {
            return Err("Overrides cannot be nested".into());
        }
        if !top && self.files.is_none() {
            return Err("each item of Overrides needs Files".into());
        }
        Ok(())
    }
}

pub enum StyleSource {
    File,
    Path(PathBuf),
    Inline(String),
}

impl StyleSource {
    pub fn parse(arg: &str) -> Result<StyleSource, Error> {
        if arg == "file" {
            Ok(StyleSource::File)
        } else if let Some(path) = arg.strip_prefix("file:") {
            Ok(StyleSource::Path(PathBuf::from(path)))
        } else if arg.trim_start().starts_with('{') {
            Ok(StyleSource::Inline(arg.to_string()))
        } else {
            Err(Error::new(format!("invalid value for --style: '{arg}'")))
        }
    }
}

pub struct Config {
    options: Options,
    overrides: Vec<(Vec<GlobMatcher>, Options)>,
    dir: PathBuf,
    parent: Option<Box<Config>>,
}

impl Config {
    pub fn from_yaml(text: &str, dir: &Path) -> Result<Config, Error> {
        let mut options: Options = serde_saphyr::from_str(text).map_err(|e| {
            let e = e.to_string();
            Error::new(e.strip_prefix("error: ").unwrap_or(&e))
        })?;
        options.validate(true).map_err(Error::new)?;
        let mut overrides = Vec::new();
        for item in options.overrides.take().unwrap_or_default() {
            item.validate(false).map_err(Error::new)?;
            let globs = item
                .files
                .iter()
                .flatten()
                .map(|p| {
                    GlobBuilder::new(p)
                        .literal_separator(true)
                        .build()
                        .map(|g| g.compile_matcher())
                        .map_err(|e| Error::new(format!("Files: {e}")))
                })
                .collect::<Result<_, _>>()?;
            overrides.push((globs, item));
        }
        Ok(Config {
            options,
            overrides,
            dir: dir.to_path_buf(),
            parent: None,
        })
    }

    pub fn load(path: &Path) -> Result<Config, Error> {
        let path = absolute(path)?;
        let text = std::fs::read_to_string(&path)
            .map_err(|e| Error::new(format!("{}: {e}", path.display())))?;
        let dir = path
            .parent()
            .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
        let mut config = Config::from_yaml(&text, &dir)
            .map_err(|e| Error::new(format!("{}: {e}", path.display())))?;
        if let Some(parent) = dir.parent() {
            config.inherit(parent)?;
        }
        Ok(config)
    }

    fn inherit(&mut self, start: &Path) -> Result<(), Error> {
        if self.options.based_on_style.is_some()
            && let Some(path) = search(start)?
        {
            self.parent = Some(Box::new(Config::load(&path)?));
        }
        Ok(())
    }

    pub fn style_for(&self, file: Option<&Path>) -> Style {
        let mut style = match &self.parent {
            Some(parent) => parent.style_for(file),
            None => Style {
                dialect_dir: self.dir.clone(),
                ..Style::default()
            },
        };
        self.options.apply(&mut style, &self.dir);
        if let Some(rel) = file.and_then(|f| relative(&self.dir, f)) {
            for (globs, options) in &self.overrides {
                if globs.iter().any(|g| g.is_match(&rel)) {
                    options.apply(&mut style, &self.dir);
                }
            }
        }
        style
    }
}

impl Style {
    pub fn find(source: &StyleSource, file: Option<&Path>) -> Result<Style, Error> {
        let cwd = std::env::current_dir().map_err(|e| Error::new(e.to_string()))?;
        let start = match file {
            Some(f) => absolute(f)?
                .parent()
                .map_or_else(|| cwd.clone(), Path::to_path_buf),
            None => cwd.clone(),
        };
        let config = match source {
            StyleSource::File => match search(&start)? {
                Some(path) => Config::load(&path)?,
                None => {
                    return Ok(Style {
                        dialect_dir: cwd,
                        ..Style::default()
                    });
                }
            },
            StyleSource::Path(path) => Config::load(path)?,
            StyleSource::Inline(text) => {
                let mut config = Config::from_yaml(text, &cwd)
                    .map_err(|e| Error::new(format!("--style: {e}")))?;
                config.inherit(&start)?;
                config
            }
        };
        Ok(config.style_for(file))
    }

    pub fn load_dialect(&self) -> Result<Dialect, Error> {
        let name = self
            .dialect
            .as_deref()
            .ok_or_else(|| Error::new("no dialect is set"))?;
        if name.is_empty() || name.contains(['/', '\\']) {
            return Err(Error::new(format!("invalid dialect name '{name}'")));
        }
        let path = self.dialect_dir.join(format!("{name}.toml"));
        if path.is_file() {
            let text = std::fs::read_to_string(&path)
                .map_err(|e| Error::new(format!("{}: {e}", path.display())))?;
            return Dialect::parse(&text)
                .map_err(|e| Error::new(format!("{}: {e}", path.display())));
        }
        Dialect::builtin(name).ok_or_else(|| Error::new(format!("unknown dialect '{name}'")))
    }

    pub fn dump(&self) -> String {
        let mut s = String::from("---\n");
        let mut line = |key: &str, value: &dyn fmt::Display| {
            let value = value.to_string();
            let sep = if value.starts_with('\n') { "" } else { " " };
            writeln!(s, "{key}:{sep}{value}").unwrap();
        };
        let consecutive = |a: &AlignConsecutive| {
            format!(
                "\n  AcrossComments: {}\n  AcrossEmptyLines: {}\n  Kind: {:?}\n  MaxPadding: {}",
                a.across_comments, a.across_empty_lines, a.kind, a.max_padding
            )
        };
        let a = &self.align_array_of_operands;
        line(
            "AlignArrayOfOperands",
            &format!(
                "\n  Kind: {:?}\n  MaxPadding: {}\n  Numbers: {:?}",
                a.kind, a.max_padding, a.numbers
            ),
        );
        line(
            "AlignConsecutiveMnemonics",
            &consecutive(&self.align_consecutive_mnemonics),
        );
        line(
            "AlignConsecutiveOperands",
            &consecutive(&self.align_consecutive_operands),
        );
        line(
            "AlignEscapedNewlines",
            &format!("{:?}", self.align_escaped_newlines),
        );
        let t = &self.align_trailing_comments;
        line(
            "AlignTrailingComments",
            &format!(
                "\n  AlignToTab: {}\n  Kind: {:?}\n  MaxColumn: {}\n  OverEmptyLines: {}",
                t.align_to_tab, t.kind, t.max_column, t.over_empty_lines
            ),
        );
        line("BlockIndentWidth", &self.block_indent_width);
        line("CommentColumn", &self.comment_column);
        line("ContinuationIndentWidth", &self.continuation_indent_width);
        if let Some(d) = &self.dialect {
            line("Dialect", d);
        }
        line(
            "DirectiveOperandColumn",
            &self.directive_operand_column.unwrap_or(self.operand_column),
        );
        line("DisableFormat", &self.disable_format);
        line("IndentBlocks", &self.indent_blocks);
        line("IndentComments", &self.indent_comments);
        line("IndentWidth", &self.indent_width);
        line("InsertNewlineAtEOF", &self.insert_newline_at_eof);
        let k = &self.keep_empty_lines;
        line(
            "KeepEmptyLines",
            &format!(
                "\n  AtEndOfFile: {}\n  AtStartOfFile: {}",
                k.at_end_of_file, k.at_start_of_file
            ),
        );
        let ending = match self.line_ending {
            LineEnding::Keep => "Keep",
            LineEnding::Lf => "LF",
            LineEnding::Crlf => "CRLF",
            LineEnding::DeriveLf => "DeriveLF",
            LineEnding::DeriveCrlf => "DeriveCRLF",
        };
        line("LineEnding", &ending);
        line("MaxEmptyLinesToKeep", &self.max_empty_lines_to_keep);
        line("OperandColumn", &self.operand_column);
        line("SpaceAfterComma", &self.space_after_comma);
        line(
            "SpacesBeforeTrailingComments",
            &self.spaces_before_trailing_comments,
        );
        line("TabWidth", &self.tab_width);
        line("UseTab", &format!("{:?}", self.use_tab));
        s.push_str("...\n");
        s
    }
}

fn absolute(path: &Path) -> Result<PathBuf, Error> {
    std::path::absolute(path).map_err(|e| Error::new(format!("{}: {e}", path.display())))
}

fn search(start: &Path) -> Result<Option<PathBuf>, Error> {
    let mut dir = Some(start);
    while let Some(d) = dir {
        let found: Vec<PathBuf> = STYLE_FILES
            .iter()
            .map(|n| d.join(n))
            .filter(|p| p.is_file())
            .collect();
        match found.len() {
            0 => dir = d.parent(),
            1 => return Ok(found.into_iter().next()),
            _ => {
                return Err(Error::new(format!(
                    "{}: more than one style file",
                    d.display()
                )));
            }
        }
    }
    Ok(None)
}

fn relative(dir: &Path, file: &Path) -> Option<String> {
    let dir = std::path::absolute(dir).ok()?;
    let file = std::path::absolute(file).ok()?;
    let rel = file.strip_prefix(dir).ok()?;
    let parts: Vec<_> = rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect();
    Some(parts.join("/"))
}
