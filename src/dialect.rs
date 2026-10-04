use std::fmt::Write;

use regex_automata::PatternID;
use regex_automata::meta::Regex;
use regex_automata::util::syntax;
use toml::{Table, Value};

use crate::Error;

const BUILTIN: &[(&str, &str)] = &[
    ("asxxxx", include_str!("../dialects/asxxxx.toml")),
    ("sdas", include_str!("../dialects/sdas.toml")),
    ("rgbds", include_str!("../dialects/rgbds.toml")),
    ("gas-x86", include_str!("../dialects/gas-x86.toml")),
    ("gas-arm", include_str!("../dialects/gas-arm.toml")),
    ("gas-aarch64", include_str!("../dialects/gas-aarch64.toml")),
    ("gas-riscv", include_str!("../dialects/gas-riscv.toml")),
    ("nasm", include_str!("../dialects/nasm.toml")),
    ("masm", include_str!("../dialects/masm.toml")),
    ("fasm", include_str!("../dialects/fasm.toml")),
    ("go", include_str!("../dialects/go.toml")),
    ("ca65", include_str!("../dialects/ca65.toml")),
    ("z88dk", include_str!("../dialects/z88dk.toml")),
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Kind {
    Comment,
    Literal,
    Label,
    Continuation,
    Separator,
}

pub(crate) enum Piece {
    Text(String),
    Group(usize),
}

pub(crate) enum End {
    Fixed(Regex),
    Template(Vec<Piece>),
}

pub(crate) struct Entry {
    pub id: usize,
    pub kind: Kind,
    pub begin: Regex,
    pub end: Option<End>,
    pub escape: Option<u8>,
    pub multiline: bool,
    pub nest: bool,
    pub before: Vec<Vec<u8>>,
}

pub(crate) struct Block {
    pub begin: Vec<Vec<u8>>,
    pub middle: Vec<Vec<u8>>,
    pub end: Vec<Vec<u8>>,
    pub fields: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Role {
    Begin,
    Middle,
    End,
}

pub(crate) struct Operands {
    pub of: Vec<Vec<u8>>,
    pub consecutive: bool,
    pub entries: Vec<Entry>,
}

pub struct Dialect {
    pub(crate) entries: Vec<Entry>,
    pub(crate) column: Option<usize>,
    pub(crate) mnemonic: Regex,
    pub(crate) positional: bool,
    pub(crate) operands: Vec<Operands>,
    pub(crate) blocks: Vec<Block>,
    pub(crate) directives: Vec<Vec<u8>>,
    pub(crate) entry_count: usize,
}

impl Dialect {
    pub fn parse(text: &str) -> Result<Dialect, Error> {
        let table: Table = toml::from_str(text).map_err(|e| Error::new(e.to_string()))?;
        Parser { ids: 0 }.dialect(&table)
    }

    pub fn builtin(name: &str) -> Option<Dialect> {
        BUILTIN
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, text)| Dialect::parse(text).expect("built-in dialect"))
    }

    pub fn builtin_names() -> impl Iterator<Item = &'static str> {
        BUILTIN.iter().map(|(name, _)| *name)
    }

    pub(crate) fn block(&self, name: &[u8], next: &[u8]) -> Option<(usize, Role)> {
        let name = name.to_ascii_lowercase();
        let next = next.to_ascii_lowercase();
        let any = |words: &[Vec<u8>]| words.iter().any(|w| word_matches(w, &name, &next));
        for role in [Role::End, Role::Middle, Role::Begin] {
            for (i, b) in self.blocks.iter().enumerate() {
                let words = match role {
                    Role::Begin => &b.begin,
                    Role::Middle => &b.middle,
                    Role::End => &b.end,
                };
                if any(words) {
                    return Some((i, role));
                }
            }
        }
        None
    }

    pub(crate) fn is_directive(&self, name: &[u8], next: &[u8]) -> bool {
        let name = name.to_ascii_lowercase();
        let next = next.to_ascii_lowercase();
        self.directives
            .iter()
            .any(|w| word_matches(w, &name, &next))
    }
}

fn word_matches(word: &[u8], name: &[u8], next: &[u8]) -> bool {
    let same = |pattern: &[u8], s: &[u8]| match pattern.strip_suffix(b"*") {
        Some(prefix) => s.starts_with(prefix),
        None => pattern == s,
    };
    match word.iter().position(|&b| b == b' ') {
        Some(sp) => same(&word[..sp], name) && same(&word[sp + 1..], next),
        None => same(word, name),
    }
}

struct Parser {
    ids: usize,
}

impl Parser {
    fn dialect(&mut self, table: &Table) -> Result<Dialect, Error> {
        check_keys(
            table,
            "",
            &[
                "comment",
                "literal",
                "label",
                "separator",
                "continuation",
                "mnemonic",
                "positional",
                "operands",
                "block",
                "directive",
            ],
        )?;
        let mut entries = Vec::new();
        for (key, kind) in [
            ("comment", Kind::Comment),
            ("literal", Kind::Literal),
            ("label", Kind::Label),
        ] {
            entries.extend(self.entries(table, key, kind)?);
        }
        let mut column = None;
        for (i, value) in array(table, "continuation", "")?.iter().enumerate() {
            let path = format!("continuation[{i}]");
            if let Value::Table(t) = value
                && t.contains_key("column")
            {
                check_keys(t, &path, &["column"])?;
                let n = t["column"]
                    .as_integer()
                    .filter(|&n| n >= 1)
                    .ok_or_else(|| {
                        Error::new(format!("{path}.column: expected a positive integer"))
                    })?;
                if column.replace(n as usize - 1).is_some() {
                    return Err(Error::new(format!(
                        "{path}: only one `column` entry is allowed"
                    )));
                }
            } else {
                entries.push(self.entry(value, Kind::Continuation, &path)?);
            }
        }
        entries.extend(self.entries(table, "separator", Kind::Separator)?);
        let mnemonic = match table.get("mnemonic") {
            None => regex(r"\S+", "mnemonic")?,
            Some(Value::String(s)) => regex(s, "mnemonic")?,
            Some(_) => return Err(Error::new("mnemonic: expected a string")),
        };
        let positional = boolean(table, "positional", "")?;
        let mut operands = Vec::new();
        for (i, value) in array(table, "operands", "")?.iter().enumerate() {
            let path = format!("operands[{i}]");
            let Value::Table(t) = value else {
                return Err(Error::new(format!("{path}: expected a table")));
            };
            operands.push(self.operands(t, &path)?);
        }
        let mut blocks = Vec::new();
        for (i, value) in array(table, "block", "")?.iter().enumerate() {
            let path = format!("block[{i}]");
            let Value::Table(t) = value else {
                return Err(Error::new(format!("{path}: expected a table")));
            };
            check_keys(t, &path, &["begin", "middle", "end", "fields"])?;
            let required = |key: &str| {
                strings(t, key, &path)?
                    .filter(|w| !w.is_empty())
                    .ok_or_else(|| Error::new(format!("{path}: `{key}` is required")))
            };
            blocks.push(Block {
                begin: required("begin")?,
                middle: strings(t, "middle", &path)?.unwrap_or_default(),
                end: required("end")?,
                fields: boolean(t, "fields", &path)?,
            });
        }
        Ok(Dialect {
            entries,
            column,
            mnemonic,
            positional,
            operands,
            blocks,
            directives: strings(table, "directive", "")?.unwrap_or_default(),
            entry_count: self.ids,
        })
    }

    fn operands(&mut self, t: &Table, path: &str) -> Result<Operands, Error> {
        check_keys(t, path, &["of", "consecutive", "comment", "literal"])?;
        let of = strings(t, "of", path)?
            .ok_or_else(|| Error::new(format!("{path}: `of` is required")))?;
        let mut entries = self.entries_at(t, "comment", Kind::Comment, path)?;
        entries.extend(self.entries_at(t, "literal", Kind::Literal, path)?);
        Ok(Operands {
            of,
            consecutive: boolean(t, "consecutive", path)?,
            entries,
        })
    }

    fn entries(&mut self, t: &Table, key: &str, kind: Kind) -> Result<Vec<Entry>, Error> {
        self.entries_at(t, key, kind, "")
    }

    fn entries_at(
        &mut self,
        t: &Table,
        key: &str,
        kind: Kind,
        path: &str,
    ) -> Result<Vec<Entry>, Error> {
        let prefix = join(path, key);
        array(t, key, path)?
            .iter()
            .enumerate()
            .map(|(i, v)| self.entry(v, kind, &format!("{prefix}[{i}]")))
            .collect()
    }

    fn entry(&mut self, value: &Value, kind: Kind, path: &str) -> Result<Entry, Error> {
        let id = self.ids;
        self.ids += 1;
        let t = match value {
            Value::String(s) => {
                let begin = regex(s, path)?;
                let end = match kind {
                    Kind::Literal => Some(end_pattern(s, &begin, path)?),
                    _ => None,
                };
                return Ok(Entry {
                    id,
                    kind,
                    begin,
                    end,
                    escape: None,
                    multiline: false,
                    nest: false,
                    before: Vec::new(),
                });
            }
            Value::Table(t) => t,
            _ => return Err(Error::new(format!("{path}: expected a string or a table"))),
        };
        let allowed: &[&str] = match kind {
            Kind::Comment | Kind::Literal => &["begin", "end", "escape", "multiline", "nest"],
            Kind::Label => &["begin", "before"],
            Kind::Separator | Kind::Continuation => &["begin"],
        };
        check_keys(t, path, allowed)?;
        let begin = string(t, "begin", path)?
            .ok_or_else(|| Error::new(format!("{path}: `begin` is required")))?;
        let begin = regex(begin, &join(path, "begin"))?;
        let end = match string(t, "end", path)? {
            Some(s) => Some(end_pattern(s, &begin, &join(path, "end"))?),
            None => None,
        };
        let escape = match string(t, "escape", path)? {
            None => None,
            Some(s) if s.len() == 1 => Some(s.as_bytes()[0]),
            Some(_) => return Err(Error::new(format!("{path}.escape: expected one byte"))),
        };
        let before = strings(t, "before", path)?.unwrap_or_default();
        Ok(Entry {
            id,
            kind,
            begin,
            end,
            escape,
            multiline: boolean(t, "multiline", path)?,
            nest: boolean(t, "nest", path)?,
            before,
        })
    }
}

fn join(path: &str, key: &str) -> String {
    if path.is_empty() {
        key.to_string()
    } else {
        format!("{path}.{key}")
    }
}

fn check_keys(t: &Table, path: &str, allowed: &[&str]) -> Result<(), Error> {
    match t.keys().find(|k| !allowed.contains(&k.as_str())) {
        Some(k) => Err(Error::new(format!("{}: unknown key", join(path, k)))),
        None => Ok(()),
    }
}

fn array<'t>(t: &'t Table, key: &str, path: &str) -> Result<&'t [Value], Error> {
    match t.get(key) {
        None => Ok(&[]),
        Some(Value::Array(a)) => Ok(a),
        Some(_) => Err(Error::new(format!(
            "{}: expected an array",
            join(path, key)
        ))),
    }
}

fn string<'t>(t: &'t Table, key: &str, path: &str) -> Result<Option<&'t str>, Error> {
    match t.get(key) {
        None => Ok(None),
        Some(Value::String(s)) => Ok(Some(s)),
        Some(_) => Err(Error::new(format!(
            "{}: expected a string",
            join(path, key)
        ))),
    }
}

fn boolean(t: &Table, key: &str, path: &str) -> Result<bool, Error> {
    match t.get(key) {
        None => Ok(false),
        Some(Value::Boolean(b)) => Ok(*b),
        Some(_) => Err(Error::new(format!(
            "{}: expected a boolean",
            join(path, key)
        ))),
    }
}

fn strings(t: &Table, key: &str, path: &str) -> Result<Option<Vec<Vec<u8>>>, Error> {
    let Some(value) = t.get(key) else {
        return Ok(None);
    };
    let error = || Error::new(format!("{}: expected an array of strings", join(path, key)));
    let Value::Array(a) = value else {
        return Err(error());
    };
    a.iter()
        .map(|v| {
            v.as_str()
                .map(|s| s.to_ascii_lowercase().into_bytes())
                .ok_or_else(error)
        })
        .collect::<Result<_, _>>()
        .map(Some)
}

pub(crate) fn regex(pattern: &str, path: &str) -> Result<Regex, Error> {
    Regex::builder()
        .syntax(syntax::Config::new().unicode(false).utf8(false))
        .configure(Regex::config().utf8_empty(false))
        .build(pattern)
        .map_err(|e| Error::new(format!("{path}: {e}")))
}

fn end_pattern(pattern: &str, begin: &Regex, path: &str) -> Result<End, Error> {
    let pieces = template(pattern);
    if !pieces.iter().any(|p| matches!(p, Piece::Group(_))) {
        return Ok(End::Fixed(regex(pattern, path)?));
    }
    let groups = begin.group_info().group_len(PatternID::ZERO);
    for piece in &pieces {
        if let Piece::Group(n) = piece
            && *n >= groups
        {
            return Err(Error::new(format!("{path}: `begin` has no group {n}")));
        }
    }
    regex(&instantiate(&pieces, |_| "x".to_string()), path)?;
    Ok(End::Template(pieces))
}

fn template(pattern: &str) -> Vec<Piece> {
    let mut pieces = Vec::new();
    let mut text = String::new();
    let mut chars = pattern.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            text.push(c);
            continue;
        }
        match chars.next() {
            Some(d @ '1'..='9') => {
                if !text.is_empty() {
                    pieces.push(Piece::Text(std::mem::take(&mut text)));
                }
                pieces.push(Piece::Group(d as usize - '0' as usize));
            }
            Some(d) => {
                text.push('\\');
                text.push(d);
            }
            None => text.push('\\'),
        }
    }
    if !text.is_empty() {
        pieces.push(Piece::Text(text));
    }
    pieces
}

pub(crate) fn instantiate(pieces: &[Piece], group: impl Fn(usize) -> String) -> String {
    pieces
        .iter()
        .map(|p| match p {
            Piece::Text(t) => t.clone(),
            Piece::Group(n) => group(*n),
        })
        .collect()
}

pub(crate) fn escape(bytes: &[u8]) -> String {
    let mut s = String::new();
    for &b in bytes {
        if b.is_ascii_alphanumeric() {
            s.push(b as char);
        } else {
            write!(s, r"\x{b:02X}").unwrap();
        }
    }
    s
}
