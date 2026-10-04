use std::collections::HashMap;
use std::ops::Range;

use regex_automata::meta::Regex;
use regex_automata::util::captures::Captures;
use regex_automata::{Anchored, Input};

use crate::dialect::{self, Dialect, End, Entry, Kind, Operands};

#[derive(Clone, Copy, Debug)]
pub(crate) struct Line {
    pub start: usize,
    pub end: usize,
    pub next: usize,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Token {
    pub kind: Kind,
    pub start: usize,
    pub end: usize,
    pub text: (usize, usize),
}

#[derive(Default, Debug)]
pub(crate) struct Stmt {
    pub labels: Vec<(usize, usize)>,
    pub mnemonic: Option<(usize, usize)>,
    pub operands: Option<(usize, usize)>,
    pub comment: Option<usize>,
    pub kept: Option<usize>,
}

pub(crate) struct LineInfo {
    pub line: Line,
    pub inner: bool,
    pub open: bool,
    pub continues: bool,
    pub cut: usize,
    pub last: usize,
    pub tokens: Range<usize>,
    pub stmt: Stmt,
}

pub(crate) struct Analysis {
    pub tokens: Vec<Token>,
    pub lines: Vec<LineInfo>,
}

pub(crate) fn is_ws(b: u8) -> bool {
    b == b' ' || b == b'\t'
}

pub(crate) fn advance(col: usize, b: u8, tab_width: usize) -> usize {
    match b {
        b'\t' => (col / tab_width + 1) * tab_width,
        0x80..=0xBF => col,
        _ => col + 1,
    }
}

pub(crate) fn split_lines(src: &[u8]) -> Vec<Line> {
    let mut lines = Vec::new();
    let mut start = 0;
    let mut i = 0;
    while i < src.len() {
        match src[i] {
            b'\n' => {
                lines.push(Line {
                    start,
                    end: i,
                    next: i + 1,
                });
                start = i + 1;
            }
            b'\r' => {
                let next = if src.get(i + 1) == Some(&b'\n') {
                    i + 2
                } else {
                    i + 1
                };
                lines.push(Line {
                    start,
                    end: i,
                    next,
                });
                start = next;
                i = next - 1;
            }
            _ => {}
        }
        i += 1;
    }
    if start < src.len() {
        lines.push(Line {
            start,
            end: src.len(),
            next: src.len(),
        });
    }
    lines
}

pub(crate) fn analyze(src: &[u8], d: &Dialect, tab_width: usize) -> Analysis {
    let lines = split_lines(src);
    let cuts = lines
        .iter()
        .map(|l| cut(src, l, d.column, tab_width))
        .collect();
    let mut lx = Lexer {
        src,
        d,
        lines,
        cuts,
        tokens: Vec::new(),
        ends: HashMap::new(),
        cache_line: usize::MAX,
        next: vec![(usize::MAX, usize::MAX); d.entry_count],
    };
    let mut infos = Vec::with_capacity(lx.lines.len());
    let mut resume = None;
    let mut li = 0;
    while li < lx.lines.len() {
        let line = lx.lines[li];
        let first = lx.tokens.len();
        let from = resume.take();
        let mut info = LineInfo {
            line,
            inner: from.is_some(),
            open: false,
            continues: false,
            cut: lx.cuts[li],
            last: line.end,
            tokens: 0..0,
            stmt: Stmt::default(),
        };
        let from = from.unwrap_or(line.start);
        let open = lx.lex_line(li, from, &mut info);
        info.tokens = first..lx.tokens.len();
        info.open = open.is_some();
        if !info.open {
            let tokens = &lx.tokens[info.tokens.clone()];
            info.continues = tokens.iter().any(|t| t.kind == Kind::Continuation)
                || (info.cut < line.end && !is_ws(src[info.cut]));
            info.last = last(src, &line, from, tokens);
        }
        infos.push(info);
        li += 1;
        if let Some(end) = open {
            while li < lx.lines.len() && end > lx.lines[li].end {
                let line = lx.lines[li];
                let t = lx.tokens.len();
                infos.push(LineInfo {
                    line,
                    inner: true,
                    open: true,
                    continues: false,
                    cut: lx.cuts[li],
                    last: line.end,
                    tokens: t..t,
                    stmt: Stmt::default(),
                });
                li += 1;
            }
            if li < lx.lines.len() {
                resume = Some(end);
            }
        }
    }
    Analysis {
        tokens: lx.tokens,
        lines: infos,
    }
}

fn cut(src: &[u8], line: &Line, column: Option<usize>, tab_width: usize) -> usize {
    let Some(column) = column else {
        return line.end;
    };
    let mut col = 0;
    for (i, &b) in src[line.start..line.end].iter().enumerate() {
        if col >= column {
            return line.start + i;
        }
        col = advance(col, b, tab_width);
    }
    line.end
}

fn last(src: &[u8], line: &Line, from: usize, tokens: &[Token]) -> usize {
    let mut end = line.end;
    while end > from && is_ws(src[end - 1]) {
        end -= 1;
    }
    tokens
        .iter()
        .map(|t| t.end)
        .filter(|&e| e <= line.end)
        .fold(end, usize::max)
}

enum Next {
    Done,
    Separator(usize),
    Open(usize),
}

struct Cand<'d> {
    entry: &'d Entry,
    start: usize,
    end: usize,
    text: (usize, usize),
}

struct Lexer<'a> {
    src: &'a [u8],
    d: &'a Dialect,
    lines: Vec<Line>,
    cuts: Vec<usize>,
    tokens: Vec<Token>,
    ends: HashMap<String, Option<Regex>>,
    cache_line: usize,
    next: Vec<(usize, usize)>,
}

impl<'a> Lexer<'a> {
    fn hay(&self, li: usize) -> &'a [u8] {
        &self.src[self.lines[li].start..self.cuts[li]]
    }

    fn skip_ws(&self, mut p: usize, lim: usize) -> usize {
        while p < lim && is_ws(self.src[p]) {
            p += 1;
        }
        p
    }

    fn trim_end(&self, from: usize, mut end: usize) -> usize {
        while end > from && is_ws(self.src[end - 1]) {
            end -= 1;
        }
        end
    }

    fn only_ws(&self, p: usize, lim: usize) -> bool {
        self.src[p..lim].iter().all(|&b| is_ws(b))
    }

    fn push(&mut self, c: &Cand) -> usize {
        self.push_raw(c.entry.kind, c.start, c.end, c.text)
    }

    fn push_raw(&mut self, kind: Kind, start: usize, end: usize, text: (usize, usize)) -> usize {
        self.tokens.push(Token {
            kind,
            start,
            end,
            text,
        });
        self.tokens.len() - 1
    }

    fn lex_line(&mut self, li: usize, from: usize, info: &mut LineInfo) -> Option<usize> {
        let mut scratch = Stmt::default();
        let mut record = !info.inner;
        let mut p = from;
        loop {
            let st = if record { &mut info.stmt } else { &mut scratch };
            match self.statement(li, p, st) {
                Next::Separator(q) => {
                    p = q;
                    record = false;
                    scratch = Stmt::default();
                }
                Next::Open(end) => return Some(end),
                Next::Done => return None,
            }
        }
    }

    fn statement(&mut self, li: usize, p: usize, st: &mut Stmt) -> Next {
        let d = self.d;
        let lim = self.cuts[li];
        let mut p = self.skip_ws(p, lim);
        let first = loop {
            if p >= lim {
                return Next::Done;
            }
            match self.best(li, p, d.entries.iter()) {
                Some(c) if c.entry.kind == Kind::Label => {
                    self.push(&c);
                    st.labels.push((c.start, c.end));
                    p = self.skip_ws(c.end, lim);
                }
                other => break other,
            }
        };
        if let Some(c) = first {
            match c.entry.kind {
                Kind::Comment => {
                    let i = self.push(&c);
                    if c.end > lim {
                        st.comment = Some(i);
                        return Next::Open(c.end);
                    }
                    if self.only_ws(c.end, lim) {
                        st.comment = Some(i);
                        return Next::Done;
                    }
                    st.kept = Some(c.start);
                    return self.operands(li, c.end, None, &mut Stmt::default());
                }
                Kind::Separator => {
                    self.push(&c);
                    st.kept = Some(c.start);
                    return Next::Separator(c.end);
                }
                Kind::Continuation => {
                    self.push(&c);
                    st.kept = Some(c.start);
                    return Next::Done;
                }
                Kind::Literal | Kind::Label => {}
            }
        }
        let (end, open) = self.mnemonic(li, p, lim);
        st.mnemonic = Some((p, end));
        if let Some(open) = open {
            return Next::Open(open);
        }
        let name = self.src[p..end].to_ascii_lowercase();
        let table = d.operands.iter().find(|t| t.of.contains(&name));
        self.operands(li, end, table, st)
    }

    fn mnemonic(&mut self, li: usize, p: usize, lim: usize) -> (usize, Option<usize>) {
        let d = self.d;
        let base = self.lines[li].start;
        let input = Input::new(self.hay(li))
            .range(p - base..)
            .anchored(Anchored::Yes);
        let matched = d
            .mnemonic
            .search(&input)
            .map(|m| base + m.end())
            .filter(|&e| e > p);
        let mut q = p;
        let mut kept = p;
        while q < lim {
            if is_ws(self.src[q]) {
                if matched.is_none_or(|m| q >= m) {
                    break;
                }
                q += 1;
                continue;
            }
            match self.best(li, q, d.entries.iter().filter(|e| e.kind != Kind::Label)) {
                Some(c) if c.entry.kind != Kind::Literal => break,
                Some(c) => {
                    if matched.is_some_and(|m| q >= m) {
                        break;
                    }
                    self.push(&c);
                    if c.end > lim {
                        return (lim, Some(c.end));
                    }
                    q = c.end;
                    kept = q;
                }
                None => {
                    if matched.is_some_and(|m| q >= m) {
                        break;
                    }
                    q += 1;
                }
            }
        }
        (self.trim_end(kept, q), None)
    }

    fn operands(
        &mut self,
        li: usize,
        from: usize,
        table: Option<&'a Operands>,
        st: &mut Stmt,
    ) -> Next {
        let d = self.d;
        let lim = self.cuts[li];
        let mut q = self.skip_ws(from, lim);
        let o = q;
        let mut last = o;
        let mut active = table.is_some();
        while q < lim {
            if is_ws(self.src[q]) {
                if d.positional && q > o {
                    st.operands = Some((o, last));
                    let r = self.skip_ws(q, lim);
                    if r < lim {
                        let end = self.trim_end(r, lim);
                        st.comment = Some(self.push_raw(Kind::Comment, r, end, (r, end)));
                    }
                    return Next::Done;
                }
                q += 1;
                continue;
            }
            let mut cand = None;
            if active && let Some(t) = table {
                cand = self.best(li, q, t.entries.iter());
                active = cand.is_some() && t.consecutive;
            }
            if cand.is_none() {
                cand = self.best(li, q, d.entries.iter().filter(|e| e.kind != Kind::Label));
            }
            let Some(c) = cand else {
                q += 1;
                last = q;
                continue;
            };
            let i = self.push(&c);
            match c.entry.kind {
                Kind::Comment if c.end > lim || self.only_ws(c.end, lim) => {
                    if last > o {
                        st.operands = Some((o, last));
                    }
                    st.comment = Some(i);
                    return if c.end > lim {
                        Next::Open(c.end)
                    } else {
                        Next::Done
                    };
                }
                Kind::Separator => {
                    if last > o {
                        st.operands = Some((o, last));
                    }
                    st.kept = Some(c.start);
                    return Next::Separator(c.end);
                }
                _ => {
                    if c.end > lim {
                        st.operands = Some((o, lim));
                        return Next::Open(c.end);
                    }
                    q = c.end;
                    last = q;
                }
            }
        }
        if last > o {
            st.operands = Some((o, last));
        }
        Next::Done
    }

    fn best<I>(&mut self, li: usize, p: usize, entries: I) -> Option<Cand<'a>>
    where
        I: Iterator<Item = &'a Entry>,
    {
        let mut best: Option<Cand> = None;
        for e in entries {
            if let Some(c) = self.try_entry(e, li, p)
                && best
                    .as_ref()
                    .is_none_or(|b| c.end - c.start > b.end - b.start)
            {
                best = Some(c);
            }
        }
        best
    }

    fn try_entry(&mut self, e: &'a Entry, li: usize, p: usize) -> Option<Cand<'a>> {
        let (bend, caps) = self.begin_at(e, li, p)?;
        let lim = self.cuts[li];
        let (end, text) = match (&e.end, e.kind) {
            (Some(end), _) => {
                let re = self.end_regex(end, caps.as_ref(), li)?;
                let (end, text_end) = self.find_end(e, &re, li, bend)?;
                (end, (bend, text_end))
            }
            (None, Kind::Comment) => {
                let end = self.trim_end(bend, lim);
                (end, (bend, end))
            }
            (None, _) => (bend, (p, bend)),
        };
        if end <= p {
            return None;
        }
        match e.kind {
            Kind::Label if !e.before.is_empty() && !self.followed_by(li, end, &e.before) => None,
            Kind::Continuation if !self.rest_is_comments(li, end) => None,
            _ => Some(Cand {
                entry: e,
                start: p,
                end,
                text,
            }),
        }
    }

    fn begin_at(&mut self, e: &Entry, li: usize, p: usize) -> Option<(usize, Option<Captures>)> {
        let base = self.lines[li].start;
        let hay = self.hay(li);
        let rel = p - base;
        if rel >= hay.len() {
            return None;
        }
        if self.cache_line != li {
            self.next.fill((usize::MAX, usize::MAX));
            self.cache_line = li;
        }
        let (from, next) = self.next[e.id];
        if from == usize::MAX || rel < from || (next != usize::MAX && rel > next) {
            let found = e
                .begin
                .search(&Input::new(hay).range(rel..))
                .map_or(usize::MAX, |m| m.start());
            self.next[e.id] = (rel, found);
        }
        if self.next[e.id].1 != rel {
            return None;
        }
        let input = Input::new(hay).range(rel..).anchored(Anchored::Yes);
        if matches!(e.end, Some(End::Template(_))) {
            let mut caps = e.begin.create_captures();
            e.begin.search_captures(&input, &mut caps);
            let m = caps.get_match()?;
            Some((base + m.end(), Some(caps)))
        } else {
            e.begin.search(&input).map(|m| (base + m.end(), None))
        }
    }

    fn end_regex(&mut self, end: &End, caps: Option<&Captures>, li: usize) -> Option<Regex> {
        let pieces = match end {
            End::Fixed(re) => return Some(re.clone()),
            End::Template(pieces) => pieces,
        };
        let hay = self.hay(li);
        let pattern = dialect::instantiate(pieces, |n| {
            caps.and_then(|c| c.get_group(n))
                .map_or(String::new(), |s| dialect::escape(&hay[s.range()]))
        });
        self.ends
            .entry(pattern)
            .or_insert_with_key(|p| dialect::regex(p, "").ok())
            .clone()
    }

    fn find_end(
        &self,
        e: &Entry,
        end: &Regex,
        mut li: usize,
        from: usize,
    ) -> Option<(usize, usize)> {
        let mut depth = 0usize;
        let mut q = from - self.lines[li].start;
        loop {
            let base = self.lines[li].start;
            let hay = self.hay(li);
            while q <= hay.len() {
                let s = end.search(&Input::new(hay).range(q..));
                let b = if e.nest {
                    e.begin.search(&Input::new(hay).range(q..))
                } else {
                    None
                };
                let limit = [s.map(|m| m.start()), b.map(|m| m.start())]
                    .into_iter()
                    .flatten()
                    .min()
                    .unwrap_or(hay.len());
                if let Some(esc) = e.escape
                    && let Some(i) = hay[q..limit].iter().position(|&c| c == esc)
                {
                    q += i + 2;
                    continue;
                }
                match (s, b) {
                    (Some(s), Some(b)) if b.start() < s.start() => {
                        depth += 1;
                        q = b.end().max(b.start() + 1);
                    }
                    (Some(s), _) => {
                        if depth == 0 {
                            return Some((base + s.end(), base + s.start()));
                        }
                        depth -= 1;
                        q = s.end().max(s.start() + 1);
                    }
                    (None, Some(b)) => {
                        depth += 1;
                        q = b.end().max(b.start() + 1);
                    }
                    (None, None) => break,
                }
            }
            if !e.multiline {
                return None;
            }
            li += 1;
            if li == self.lines.len() {
                return Some((self.src.len(), self.src.len()));
            }
            q = 0;
        }
    }

    fn followed_by(&mut self, li: usize, p: usize, words: &[Vec<u8>]) -> bool {
        let lim = self.cuts[li];
        if p >= lim || !is_ws(self.src[p]) {
            return false;
        }
        let start = self.skip_ws(p, lim);
        let mut q = start;
        while q < lim && !is_ws(self.src[q]) && !self.token_starts(li, q) {
            q += 1;
        }
        let word = self.src[start..q].to_ascii_lowercase();
        q > start && words.contains(&word)
    }

    fn token_starts(&mut self, li: usize, p: usize) -> bool {
        let d = self.d;
        d.entries
            .iter()
            .filter(|e| e.kind != Kind::Label)
            .any(|e| self.try_entry(e, li, p).is_some())
    }

    fn rest_is_comments(&mut self, li: usize, mut p: usize) -> bool {
        let d = self.d;
        let lim = self.cuts[li];
        loop {
            p = self.skip_ws(p, lim);
            if p >= lim {
                return true;
            }
            match self.best(li, p, d.entries.iter().filter(|e| e.kind == Kind::Comment)) {
                Some(c) if c.end > lim => return true,
                Some(c) => p = c.end,
                None => return false,
            }
        }
    }
}
