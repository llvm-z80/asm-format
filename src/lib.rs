mod check;
mod config;
mod dialect;
mod layout;
mod lex;

use std::fmt;

pub use config::{
    AlignConsecutive, Alignment, ArrayOfOperands, Config, EscapedNewlines, KeepEmptyLines,
    LineEnding, OperandAlignment, Style, StyleSource, TrailingComments, UseTab,
};
pub use dialect::Dialect;

#[derive(Debug)]
pub struct Error(String);

impl Error {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Error(message.into())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

pub fn format(src: &[u8], dialect: &Dialect, style: &Style) -> Result<Vec<u8>, Error> {
    if style.disable_format {
        return Ok(src.to_vec());
    }
    let analysis = lex::analyze(src, dialect, style.tab_width);
    let out = layout::layout(src, &analysis, dialect, style);
    check::check(src, &analysis, &out, dialect, style.tab_width)?;
    Ok(out)
}
