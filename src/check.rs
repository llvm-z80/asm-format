use crate::Error;
use crate::dialect::{Dialect, Kind};
use crate::lex::{self, Analysis, Token, is_ws};

pub(crate) fn check(
    src: &[u8],
    a: &Analysis,
    out: &[u8],
    d: &Dialect,
    tab_width: usize,
) -> Result<(), Error> {
    let b = lex::analyze(out, d, tab_width);
    let tokens = a.tokens.len() == b.tokens.len()
        && a.tokens.iter().zip(&b.tokens).all(|(x, y)| {
            let (a, b) = (&src[x.start..x.end], &out[y.start..y.end]);
            x.kind == y.kind && (a == b || x.kind == Kind::Comment && lf(a) == lf(b))
        });
    let (text, gaps) = squeeze(src, &a.tokens);
    let (new_text, new_gaps) = squeeze(out, &b.tokens);
    if tokens && text == new_text && gaps.iter().all(|g| new_gaps.binary_search(g).is_ok()) {
        Ok(())
    } else {
        Err(Error::new(
            "the result differs from the input in more than whitespace",
        ))
    }
}

/// Removes whitespace outside tokens and collapses line breaks. The second
/// result lists the positions in the text that follow whitespace between two
/// bytes of a line, except before comments.
fn squeeze(src: &[u8], tokens: &[Token]) -> (Vec<u8>, Vec<usize>) {
    let mut v = Vec::with_capacity(src.len());
    let mut gaps = Vec::new();
    let mut gap = false;
    let mut tokens = tokens.iter().peekable();
    let mut i = 0;
    while i < src.len() {
        if let Some(t) = tokens.next_if(|t| t.start == i) {
            if gap && t.kind != Kind::Comment {
                gaps.push(v.len());
            }
            gap = false;
            v.extend_from_slice(&lf(&src[t.start..t.end]));
            i = t.end;
            continue;
        }
        match src[i] {
            b if is_ws(b) => gap = v.last().is_some_and(|&c| c != b'\n'),
            b'\n' | b'\r' => {
                gap = false;
                if v.last().is_some_and(|&c| c != b'\n') {
                    v.push(b'\n');
                }
            }
            b => {
                if gap {
                    gaps.push(v.len());
                }
                gap = false;
                v.push(b);
            }
        }
        i += 1;
    }
    if v.last() == Some(&b'\n') {
        v.pop();
    }
    (v, gaps)
}

fn lf(text: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(text.len());
    let mut i = 0;
    while i < text.len() {
        if text[i] == b'\r' {
            v.push(b'\n');
            if text.get(i + 1) == Some(&b'\n') {
                i += 1;
            }
        } else {
            v.push(text[i]);
        }
        i += 1;
    }
    v
}
