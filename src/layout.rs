use crate::config::{
    AlignConsecutive, Alignment, EscapedNewlines, LineEnding, NumberAlignment, OperandAlignment,
    Style, UseTab,
};
use crate::dialect::{Dialect, Kind, Role};
use crate::lex::{self, Analysis, LineInfo, Token, advance, is_ws};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Shape {
    Empty,
    Fixed,
    Kept,
    Continued,
    Comment,
    Statement,
}

struct Plan {
    labels: Vec<u8>,
    label_end: Option<usize>,
    mnemonic: Option<(usize, usize)>,
    operands: Option<(Vec<Vec<u8>>, bool)>,
}

struct Note {
    token: usize,
    column: usize,
    follows: Option<usize>,
}

struct Row {
    shape: Shape,
    out: Vec<u8>,
    col: usize,
    plan: Option<Plan>,
    shift: usize,
    next_shift: usize,
    block: bool,
    directive: bool,
    lcol: usize,
    bare: bool,
    mcol: usize,
    mend: usize,
    ocol: usize,
    ostart: usize,
    fcols: Vec<usize>,
    head: (usize, usize),
    indent: bool,
    esc: Option<(usize, usize, bool)>,
    note: Option<Note>,
    ccol: usize,
    rest: (usize, usize),
    tail: bool,
    links: bool,
}

impl Row {
    fn new(shape: Shape) -> Row {
        Row {
            shape,
            out: Vec::new(),
            col: 0,
            plan: None,
            shift: 0,
            next_shift: 0,
            block: false,
            directive: false,
            lcol: 0,
            bare: false,
            mcol: 0,
            mend: 0,
            ocol: 0,
            ostart: 0,
            fcols: Vec::new(),
            head: (0, 0),
            indent: false,
            esc: None,
            note: None,
            ccol: 0,
            rest: (0, 0),
            tail: false,
            links: false,
        }
    }

    fn emit(&mut self, bytes: &[u8], style: &Style) {
        for &b in bytes {
            self.col = advance(self.col, b, style.tab_width);
        }
        self.out.extend_from_slice(bytes);
    }

    fn fill(&mut self, target: usize, indent: bool, style: &Style) {
        let tabs = match style.use_tab {
            UseTab::Never => false,
            UseTab::ForIndentation => indent,
            UseTab::Always => true,
        };
        let width = style.tab_width;
        if tabs && target > self.col + 1 {
            while (self.col / width + 1) * width <= target {
                self.out.push(b'\t');
                self.col = (self.col / width + 1) * width;
            }
        }
        self.pad(target);
    }

    fn pad(&mut self, target: usize) {
        while self.col < target {
            self.out.push(b' ');
            self.col += 1;
        }
    }
}

pub(crate) fn layout(src: &[u8], a: &Analysis, d: &Dialect, style: &Style) -> Vec<u8> {
    let mut rows = build(src, a, d, style);
    align_mnemonics(&mut rows, style);
    align_operands(&mut rows, src, style);
    align_fields(&mut rows, style);
    render(&mut rows, src, style);
    align_escapes(&mut rows, src, style);
    align_comments(&mut rows, style);
    emit(&rows, src, a, d, style)
}

fn build(src: &[u8], a: &Analysis, d: &Dialect, style: &Style) -> Vec<Row> {
    let disabled = disabled(src, a);
    let mut shapes = Vec::with_capacity(a.lines.len());
    let mut continued = false;
    for (info, &off) in a.lines.iter().zip(&disabled) {
        shapes.push(shape_of(src, info, off, continued, d, style));
        continued = info.continues;
    }
    let words: Vec<_> = a
        .lines
        .iter()
        .zip(&shapes)
        .map(|(info, &shape)| words(src, info, shape))
        .collect();
    let roles: Vec<_> = words
        .iter()
        .map(|w| w.and_then(|(name, next)| d.block(name, next)))
        .collect();
    let kept: Vec<bool> = shapes
        .iter()
        .zip(&words)
        .map(|(&shape, w)| shape == Shape::Kept && w.is_some())
        .collect();
    let flat = flat_blocks(&kept, &roles);
    let fields = field_lines(&roles, d);
    let mut rows = Vec::with_capacity(a.lines.len());
    let mut trail: Option<(usize, usize)> = None;
    let mut blocks: Vec<(usize, bool)> = Vec::new();
    let depth = |blocks: &[(usize, bool)]| blocks.iter().filter(|b| b.1).count();
    for (i, info) in a.lines.iter().enumerate() {
        let line = info.line;
        let fixed = disabled[i] || info.inner;
        let shape = shapes[i];
        let role = roles[i];
        let mut row = Row::new(shape);
        let (mut here, mut after) = (depth(&blocks), depth(&blocks));
        row.directive = words[i].is_some_and(|(name, next)| d.is_directive(name, next));
        match role {
            _ if !style.indent_blocks => {}
            Some((b, Role::Begin)) => {
                let indented = match shape {
                    Shape::Statement => info.stmt.labels.is_empty(),
                    _ => is_ws(src[line.start]),
                };
                blocks.push((b, (indented || style.indent_width == 0) && !flat[i]));
                after = depth(&blocks);
            }
            Some((b, role)) => {
                if let Some(p) = blocks.iter().rposition(|e| e.0 == b) {
                    here = depth(&blocks[..p]);
                    blocks.truncate(if role == Role::Middle { p + 1 } else { p });
                    after = depth(&blocks);
                    if role == Role::End {
                        here = after;
                    }
                }
            }
            None => {}
        }
        row.block = role.is_some();
        row.shift = here * style.block_indent_width;
        row.next_shift = after * style.block_indent_width;
        row.links = info.continues && d.column.is_none() && !fixed;
        row.tail = matches!(shape, Shape::Statement | Shape::Kept | Shape::Continued)
            && d.column.is_none()
            && !info.open
            && info.last < line.end
            && trailing_matters(src, line.start, info.last, line.end, d, style);
        match shape {
            Shape::Empty | Shape::Fixed => trail = None,
            Shape::Kept | Shape::Continued => {
                trail = None;
                let start = if shape == Shape::Kept {
                    line.start
                } else {
                    row.indent = is_ws(src[line.start]);
                    skip_ws(src, line.start, line.end)
                };
                row.head = (start, info.last);
                if row.links
                    && let Some(t) = escape(a, info)
                {
                    let gap = trim_ws(src, start, t.start);
                    row.esc = Some((t.start, t.end, gap < t.start));
                    row.head = (start, gap);
                    row.rest = (t.end, info.last);
                }
                // A kept line keeps its trailing comment where it is, so the
                // comment lines that continue it stay at that column.
                if shape == Shape::Kept
                    && !info.continues
                    && let Some(ci) = info.stmt.comment
                    && a.tokens[ci].end <= line.end
                {
                    row.ccol = column(src, line.start, a.tokens[ci].start, style.tab_width);
                    trail = Some((row.ccol, rows.len()));
                }
            }
            Shape::Comment => {
                let ci = info.stmt.comment.unwrap();
                let t = a.tokens[ci];
                let column = column(src, line.start, t.start, style.tab_width);
                row.indent = style.indent_comments && t.end <= info.cut;
                if !row.indent {
                    row.emit(&src[line.start..t.start], style);
                }
                let follows = match trail {
                    Some((from, leader)) if from == column && row.indent => Some(leader),
                    _ => None,
                };
                trail = follows.map(|leader| (column, leader));
                row.note = Some(Note {
                    token: ci,
                    column,
                    follows,
                });
            }
            Shape::Statement => {
                let end = info.last.min(info.cut);
                let st = &info.stmt;
                let mut labels = Row::new(shape);
                let mut prev = line.start;
                for (k, &(s, e)) in st.labels.iter().enumerate() {
                    if k > 0 && s > prev {
                        labels.emit(b" ", style);
                    }
                    labels.emit(&src[s..e], style);
                    prev = e;
                }
                if fields[i] && !st.labels.is_empty() {
                    row.lcol = style.indent_width + row.shift;
                }
                let label_end = (!st.labels.is_empty()).then_some(row.lcol + labels.col);
                if let Some((_, e)) = st.mnemonic {
                    prev = e;
                }
                let esc = if row.links && st.kept.is_none() {
                    escape(a, info)
                } else {
                    None
                };
                let mut operands = None;
                if let Some((s, e)) = st.operands {
                    let mut e = e.min(end);
                    if let Some(t) = esc {
                        let gap = trim_ws(src, prev, t.start);
                        row.esc = Some((t.start, t.end, gap < t.start));
                        e = trim_ws(src, s, t.start);
                    }
                    if e > s {
                        let fields =
                            operand_fields(src, s, e, &a.tokens[info.tokens.clone()], d, style);
                        operands = Some((fields, s > prev));
                    }
                    prev = st.operands.unwrap().1.min(end);
                }
                if st.kept.is_some() || (st.mnemonic.is_none() && st.labels.is_empty()) {
                    row.rest = (prev, end);
                }
                if let Some(ci) = st.comment {
                    let t = a.tokens[ci];
                    row.note = Some(Note {
                        token: ci,
                        column: column(src, line.start, t.start, style.tab_width),
                        follows: None,
                    });
                }
                row.bare = st.mnemonic.is_none() && st.kept.is_none() && !st.labels.is_empty();
                trail = row
                    .note
                    .as_ref()
                    .filter(|_| !row.bare)
                    .map(|n| (n.column, rows.len()));
                row.plan = Some(Plan {
                    labels: labels.out,
                    label_end,
                    mnemonic: st.mnemonic,
                    operands,
                });
            }
        }
        rows.push(row);
    }
    rows
}

fn trailing_matters(
    src: &[u8],
    start: usize,
    last: usize,
    end: usize,
    d: &Dialect,
    style: &Style,
) -> bool {
    let shape = |line: &[u8]| {
        let mut text = line.to_vec();
        text.extend_from_slice(b"\nx");
        let a = lex::analyze(&text, d, style.tab_width);
        let tokens: Vec<_> = a
            .tokens
            .iter()
            .map(|t| (t.kind, t.start, t.end > line.len()))
            .collect();
        (tokens, a.lines.first().is_some_and(|l| l.continues))
    };
    shape(&src[start..end]) != shape(&src[start..last])
}

fn escape<'t>(a: &'t Analysis, info: &LineInfo) -> Option<&'t Token> {
    a.tokens[info.tokens.clone()]
        .iter()
        .rev()
        .find(|t| t.kind == Kind::Continuation)
}

fn skip_ws(src: &[u8], mut p: usize, end: usize) -> usize {
    while p < end && is_ws(src[p]) {
        p += 1;
    }
    p
}

fn trim_ws(src: &[u8], from: usize, mut end: usize) -> usize {
    while end > from && is_ws(src[end - 1]) {
        end -= 1;
    }
    end
}

#[derive(PartialEq)]
enum Step<K> {
    Match(K, usize),
    Skip,
    Empty,
    Comment,
    Break,
}

fn shape_of(
    src: &[u8],
    info: &LineInfo,
    off: bool,
    continued: bool,
    d: &Dialect,
    style: &Style,
) -> Shape {
    let line = info.line;
    let blank = src[line.start..line.end].iter().all(|&b| is_ws(b));
    if off || info.inner || (continued && (blank || d.column.is_some())) {
        Shape::Fixed
    } else if continued {
        Shape::Continued
    } else if blank {
        Shape::Empty
    } else if !is_ws(src[line.start]) && info.stmt.labels.is_empty() && style.indent_width > 0 {
        Shape::Kept
    } else if info.stmt.labels.is_empty()
        && info.stmt.mnemonic.is_none()
        && info.stmt.kept.is_none()
        && info.stmt.comment.is_some()
    {
        Shape::Comment
    } else {
        Shape::Statement
    }
}

/// The mnemonic of a line and the first word of its operands.
fn words<'a>(src: &'a [u8], info: &LineInfo, shape: Shape) -> Option<(&'a [u8], &'a [u8])> {
    match (shape, info.stmt.mnemonic) {
        (Shape::Statement | Shape::Kept | Shape::Fixed, Some((s, e))) if !info.inner => {
            let next = info.stmt.operands.map_or(&[][..], |(o, e)| {
                let w = src[o..e].iter().position(|&b| is_ws(b));
                &src[o..w.map_or(e, |w| o + w)]
            });
            Some((&src[s..e], next))
        }
        _ => None,
    }
}

/// Marks the openers of blocks that contain code kept at column 0; such blocks
/// do not indent their lines.
fn flat_blocks(kept: &[bool], roles: &[Option<(usize, Role)>]) -> Vec<bool> {
    let mut flat = vec![false; kept.len()];
    let mut open: Vec<(usize, usize)> = Vec::new();
    for (i, (&kept, &role)) in kept.iter().zip(roles).enumerate() {
        if let Some((b, role)) = role
            && role != Role::Begin
            && let Some(p) = open.iter().rposition(|e| e.0 == b)
        {
            open.truncate(if role == Role::Middle { p + 1 } else { p });
        }
        if kept {
            for &(_, opener) in &open {
                flat[opener] = true;
            }
        }
        if let Some((b, Role::Begin)) = role {
            open.push((b, i));
        }
    }
    flat
}

/// Marks the lines inside blocks whose labels name fields.
fn field_lines(roles: &[Option<(usize, Role)>], d: &Dialect) -> Vec<bool> {
    let mut inside = vec![false; roles.len()];
    let mut open: Vec<usize> = Vec::new();
    let any = |open: &[usize]| open.iter().any(|&b| d.blocks[b].fields);
    for (i, &role) in roles.iter().enumerate() {
        inside[i] = any(&open);
        match role {
            Some((b, Role::Begin)) => open.push(b),
            Some((b, role)) => {
                if let Some(p) = open.iter().rposition(|&o| o == b) {
                    inside[i] = any(&open[..p]);
                    open.truncate(if role == Role::Middle { p + 1 } else { p });
                }
            }
            None => {}
        }
    }
    inside
}

fn align_runs<K: PartialEq + Copy>(
    steps: &[Step<K>],
    opt: &AlignConsecutive,
    style: &Style,
) -> Vec<usize> {
    let mut widths = vec![0; steps.len()];
    let mut run: Vec<usize> = Vec::new();
    let mut key = None;
    let mut width = 0;
    let mut low = usize::MAX;
    let flush = |widths: &mut Vec<usize>, run: &mut Vec<usize>, width: usize| {
        for &i in run.iter() {
            widths[i] = width;
        }
        run.clear();
    };
    for (i, step) in steps.iter().enumerate() {
        match step {
            Step::Match(k, base) => {
                let spread = width.max(*base) - low.min(*base);
                if opt.kind == Alignment::None
                    || key != Some(*k)
                    || (opt.max_padding > 0 && !run.is_empty() && spread > opt.max_padding)
                {
                    flush(&mut widths, &mut run, width);
                    width = 0;
                    low = usize::MAX;
                }
                key = Some(*k);
                width = width.max(*base);
                low = low.min(*base);
                run.push(i);
            }
            Step::Skip => {}
            Step::Empty if opt.across_empty_lines || style.max_empty_lines_to_keep == 0 => {}
            Step::Comment if opt.across_comments => {}
            _ => {
                flush(&mut widths, &mut run, width);
                width = 0;
                low = usize::MAX;
                key = None;
            }
        }
    }
    flush(&mut widths, &mut run, width);
    widths
}

fn align_mnemonics(rows: &mut [Row], style: &Style) {
    let mut steps = Vec::with_capacity(rows.len());
    for r in rows.iter_mut() {
        let indent = style.indent_width + r.shift;
        let step = match (&r.plan, r.shape) {
            (Some(p), _) if p.mnemonic.is_some() => {
                let own = p.label_end.filter(|&end| end >= indent).map(|end| end + 1);
                r.mcol = own.unwrap_or(indent);
                match (own, p.label_end) {
                    _ if r.block => Step::Break,
                    (Some(own), _) => Step::Match((), own),
                    (None, Some(_)) => Step::Skip,
                    (None, None) => Step::Break,
                }
            }
            (_, Shape::Empty) => Step::Empty,
            (_, Shape::Comment) => Step::Comment,
            _ => Step::Break,
        };
        steps.push(step);
    }
    let widths = align_runs(&steps, &style.align_consecutive_mnemonics, style);
    for (r, (step, w)) in rows.iter_mut().zip(steps.iter().zip(widths)) {
        if matches!(step, Step::Match(..)) {
            r.mcol = w;
        }
    }
}

fn align_operands(rows: &mut [Row], src: &[u8], style: &Style) {
    let mut steps = Vec::with_capacity(rows.len());
    for r in rows.iter_mut() {
        let step = match (&r.plan, r.shape) {
            (Some(p), _) => match (p.mnemonic, &p.operands) {
                (Some((s, e)), Some((_, true))) => {
                    let end = width(&src[s..e], r.mcol, style);
                    r.mend = end;
                    let next = if style.use_tab == UseTab::Always {
                        (end / style.tab_width + 1) * style.tab_width
                    } else {
                        end + 1
                    };
                    let own = r.directive && style.directive_operand_column.is_some();
                    let column = match style.directive_operand_column {
                        Some(c) if own => c,
                        _ => style.operand_column,
                    };
                    r.ocol = if column == 0 {
                        end + 1
                    } else {
                        (column + r.shift).max(next)
                    };
                    // A line whose mnemonic ends before the column takes part
                    // too, so it follows a longer mnemonic in its run.
                    if r.block {
                        Step::Break
                    } else {
                        Step::Match((r.mcol, own), r.ocol)
                    }
                }
                _ => Step::Break,
            },
            (_, Shape::Empty) => Step::Empty,
            (_, Shape::Comment) => Step::Comment,
            _ => Step::Break,
        };
        steps.push(step);
    }
    let widths = align_runs(&steps, &style.align_consecutive_operands, style);
    for (r, (step, w)) in rows.iter_mut().zip(steps.iter().zip(widths)) {
        if matches!(step, Step::Match(..)) {
            r.ocol = w;
        }
        if let Some(Plan {
            mnemonic: Some((s, e)),
            operands: Some((_, gap)),
            ..
        }) = &r.plan
        {
            r.ostart = if *gap {
                r.ocol
            } else {
                width(&src[*s..*e], r.mcol, style)
            };
        }
    }
}

fn width(bytes: &[u8], from: usize, style: &Style) -> usize {
    bytes
        .iter()
        .fold(from, |col, &b| advance(col, b, style.tab_width))
}

fn align_fields(rows: &mut [Row], style: &Style) {
    let opt = &style.align_array_of_operands;
    if opt.kind == OperandAlignment::None {
        return;
    }
    let first = opt.kind == OperandAlignment::RightFirst;
    let mut i = 0;
    while i < rows.len() {
        let mut group: Vec<usize> = Vec::new();
        let (mut lo, mut hi) = (usize::MAX, 0);
        let mut j = i;
        while j < rows.len() {
            if let Some((low, high)) = first_end(&rows[j], first, style) {
                let same = first
                    || group
                        .first()
                        .is_none_or(|&k| rows[k].ostart == rows[j].ostart);
                let (l, h) = (lo.min(low), hi.max(high));
                if !same || (opt.max_padding > 0 && h.saturating_sub(l) > opt.max_padding) {
                    break;
                }
                (lo, hi) = (l, h);
                group.push(j);
            } else if !(rows[j].shape == Shape::Empty && style.max_empty_lines_to_keep == 0) {
                break;
            }
            j += 1;
        }
        align_group(rows, &group, first.then(|| lo.max(hi)), style);
        i = if group.is_empty() { j + 1 } else { j };
    }
}

/// Where the first operand ends at the operand column and, for `RightFirst`, right after the mnemonic.
fn first_end(r: &Row, first: bool, style: &Style) -> Option<(usize, usize)> {
    let fields = operand_list(r);
    if fields.len() < 2 || r.block {
        return None;
    }
    if !first {
        let end = width(&fields[0], r.ostart, style);
        return Some((end, end));
    }
    if !gap(r) || fields[0].contains(&b'\t') {
        return None;
    }
    let w = width(&fields[0], 0, style);
    Some((r.ostart + w, r.mend + 1 + w))
}

fn operand_list(r: &Row) -> &[Vec<u8>] {
    match &r.plan {
        Some(Plan {
            operands: Some((f, _)),
            ..
        }) => f,
        _ => &[],
    }
}

fn align_group(rows: &mut [Row], group: &[usize], first: Option<usize>, style: &Style) {
    let opt = &style.align_array_of_operands;
    let all = opt.kind == OperandAlignment::Right;
    let numbers = opt.numbers == NumberAlignment::Right;
    let right = |rows: &[Row], n: usize| all || (numbers && numeric(rows, group, n));
    if let Some(end) = first {
        for &k in group {
            let start = end - width(&operand_list(&rows[k])[0], 0, style);
            rows[k].ocol = start;
            rows[k].ostart = start;
        }
    } else if right(rows, 0) && group.iter().all(|&k| gap(&rows[k])) {
        let widest = group
            .iter()
            .map(|&k| width(value(&operand_list(&rows[k])[0]), 0, style))
            .max()
            .unwrap_or(0);
        for &k in group {
            let pad = widest - width(value(&operand_list(&rows[k])[0]), 0, style);
            rows[k].ocol += pad;
            rows[k].ostart += pad;
        }
    }
    let mut ends: Vec<usize> = group
        .iter()
        .map(|&k| width(&operand_list(&rows[k])[0], rows[k].ostart, style))
        .collect();
    let most = group.iter().map(|&k| operand_list(&rows[k]).len()).max();
    let limit = style.align_array_of_operands.max_padding;
    for n in 1..most.unwrap_or(0) {
        let starts = || {
            group
                .iter()
                .zip(&ends)
                .filter(|&(&k, _)| operand_list(&rows[k]).len() > n)
                .map(|(_, &e)| e + 1)
        };
        let column = starts().max().unwrap_or(0);
        if limit > 0 && column - starts().min().unwrap_or(column) > limit {
            break;
        }
        // A column aligned to the right ends at the leftmost column that every
        // line allows, so a long operand before it does not move the others.
        let edge = right(rows, n).then(|| {
            group
                .iter()
                .zip(&ends)
                .filter_map(|(&k, &e)| {
                    let field = operand_list(&rows[k]).get(n)?;
                    Some(e + 1 + width(value(field), 0, style))
                })
                .max()
                .unwrap_or(column)
        });
        for (m, &k) in group.iter().enumerate() {
            if let Some(field) = operand_list(&rows[k]).get(n) {
                let start = edge.map_or(column, |r| r - width(value(field), 0, style));
                ends[m] = width(field, start, style);
                rows[k].fcols.push(start);
            }
        }
    }
}

fn gap(r: &Row) -> bool {
    matches!(
        r.plan,
        Some(Plan {
            operands: Some((_, true)),
            ..
        })
    )
}

/// Whether every operand in column `n` of a group is a number.
fn numeric(rows: &[Row], group: &[usize], n: usize) -> bool {
    let fields: Vec<&Vec<u8>> = group
        .iter()
        .filter_map(|&k| operand_list(&rows[k]).get(n))
        .collect();
    fields.len() > 1 && fields.iter().all(|f| is_number(f))
}

/// An operand without the comma that follows it.
fn value(field: &[u8]) -> &[u8] {
    field.strip_suffix(b",").unwrap_or(field).trim_ascii_end()
}

/// Whether an operand is a number: a decimal digit followed by letters,
/// digits and underscores, `$` followed by such a number or by hexadecimal
/// digits, or `%` followed by binary digits, after an optional `#` and `-`.
fn is_number(field: &[u8]) -> bool {
    let f = value(field).trim_ascii_start();
    let f = f.strip_prefix(b"#").unwrap_or(f);
    let f = f.strip_prefix(b"-").unwrap_or(f);
    let plain = |s: &[u8]| {
        s.first().is_some_and(u8::is_ascii_digit)
            && s.iter().all(|&b| b.is_ascii_alphanumeric() || b == b'_')
    };
    let digits = |s: &[u8], digit: fn(&u8) -> bool| {
        s.first().is_some_and(digit) && s.iter().all(|b| digit(b) || *b == b'_')
    };
    match f {
        [b'$', rest @ ..] => plain(rest) || digits(rest, u8::is_ascii_hexdigit),
        [b'%', rest @ ..] => digits(rest, |&b| b == b'0' || b == b'1'),
        _ => plain(f),
    }
}

fn render(rows: &mut [Row], src: &[u8], style: &Style) {
    let mut base = 0;
    for r in rows.iter_mut() {
        match r.shape {
            Shape::Statement => {
                let p = r.plan.take().unwrap();
                r.fill(r.lcol, true, style);
                r.emit(&p.labels, style);
                if let Some((s, e)) = p.mnemonic {
                    r.fill(r.mcol, true, style);
                    r.emit(&src[s..e], style);
                }
                if let Some((fields, gap)) = &p.operands {
                    if *gap {
                        r.fill(r.ocol, false, style);
                    }
                    for (n, field) in fields.iter().enumerate() {
                        match n.checked_sub(1).and_then(|k| r.fcols.get(k)) {
                            Some(&column) => r.pad(column),
                            None if n > 0 => r.emit(b" ", style),
                            None => {}
                        }
                        r.emit(field, style);
                    }
                }
                base = if p.mnemonic.is_some() { r.mcol } else { 0 };
            }
            Shape::Kept => {
                r.emit(&src[r.head.0..r.head.1], style);
                base = 0;
            }
            Shape::Continued => {
                if r.indent {
                    r.fill((base + style.continuation_indent_width).max(1), true, style);
                }
                r.emit(&src[r.head.0..r.head.1], style);
            }
            Shape::Fixed => base = 0,
            _ => {}
        }
    }
}

fn align_escapes(rows: &mut [Row], src: &[u8], style: &Style) {
    let mut i = 0;
    while i < rows.len() {
        if !rows[i].links {
            i += 1;
            continue;
        }
        let mut j = i;
        while j < rows.len() && rows[j].links {
            j += 1;
        }
        let mut width = rows[i..j]
            .iter()
            .filter(|r| r.esc.is_some_and(|(_, _, gap)| gap))
            .map(|r| r.col + 1)
            .max()
            .unwrap_or(0);
        if style.align_escaped_newlines == EscapedNewlines::LeftWithLastLine
            && j < rows.len()
            && matches!(
                rows[j].shape,
                Shape::Statement | Shape::Kept | Shape::Continued
            )
        {
            width = width.max(rows[j].col + 1);
        }
        for r in &mut rows[i..j] {
            let Some((start, end, gap)) = r.esc else {
                continue;
            };
            if gap {
                let target = match style.align_escaped_newlines {
                    EscapedNewlines::None => r.col + 1,
                    _ => width,
                };
                r.fill(target, false, style);
            }
            r.emit(&src[start..end], style);
        }
        i = j;
    }
}

fn align_comments(rows: &mut [Row], style: &Style) {
    let opt = &style.align_trailing_comments;
    let spaces = style.spaces_before_trailing_comments;
    let round = |c: usize| {
        if opt.align_to_tab {
            c.div_ceil(style.tab_width) * style.tab_width
        } else {
            c
        }
    };
    let outlier = |r: &Row, c: usize| opt.max_column > 0 && round(c) > opt.max_column + r.shift;
    let least = |r: &Row| {
        let own = r.col + spaces;
        if r.bare {
            own
        } else {
            own.max(style.indent_width + r.next_shift + 1)
        }
    };
    let flush = |rows: &mut [Row], block: &mut Vec<usize>| {
        let width = block
            .iter()
            .filter(|&&i| !outlier(&rows[i], least(&rows[i])))
            .map(|&i| least(&rows[i]))
            .map(round)
            .max()
            .unwrap_or(0);
        let others = block.iter().any(|&i| !rows[i].bare);
        for &i in block.iter() {
            let own = least(&rows[i]);
            if rows[i].bare && !others && own <= style.indent_width + rows[i].shift {
                rows[i].ccol = own.max(style.indent_width + rows[i].shift);
                continue;
            }
            let target = if opt.kind == Alignment::None || outlier(&rows[i], own) {
                own
            } else {
                width
            };
            rows[i].ccol = style.comment_column.max(target);
        }
        block.clear();
    };
    let mut block = Vec::new();
    let mut empty = 0;
    for i in 0..rows.len() {
        match rows[i].shape {
            Shape::Empty => {
                empty += 1;
                if empty.min(style.max_empty_lines_to_keep) > opt.over_empty_lines {
                    flush(rows, &mut block);
                }
                continue;
            }
            Shape::Statement => {
                if rows[i].note.is_some() {
                    block.push(i);
                }
            }
            Shape::Comment if rows[i].note.as_ref().is_some_and(|n| n.follows.is_some()) => {}
            _ => flush(rows, &mut block),
        }
        empty = 0;
    }
    flush(rows, &mut block);
    for i in 0..rows.len() {
        let Some(follows) = rows[i].note.as_ref().map(|n| n.follows) else {
            continue;
        };
        match rows[i].shape {
            Shape::Statement => {
                let target = rows[i].ccol;
                rows[i].fill(target, false, style);
            }
            Shape::Comment if rows[i].indent => match follows {
                Some(leader) => {
                    let target = rows[leader].ccol;
                    rows[i].ccol = target;
                    let indent = style.indent_width + rows[i].shift;
                    rows[i].fill(indent.min(target), true, style);
                    rows[i].fill(target, false, style);
                }
                None => {
                    let indent = style.indent_width + rows[i].shift;
                    rows[i].fill(indent, true, style);
                }
            },
            _ => {}
        }
    }
}

fn emit(rows: &[Row], src: &[u8], a: &Analysis, d: &Dialect, style: &Style) -> Vec<u8> {
    let inside = inside_literals(a);
    let ending = line_ending(src, a, &inside, style.line_ending);
    let mut out = Vec::with_capacity(src.len() + src.len() / 4);
    let mut pending: Vec<&[u8]> = Vec::new();
    let mut started = false;
    let mut terminator: &[u8] = b"\n";
    for (i, row) in rows.iter().enumerate() {
        let info = &a.lines[i];
        let line = info.line;
        let original = &src[line.end..line.next];
        let term = match ending {
            Some(e) if !original.is_empty() && !inside[i] => e,
            _ => original,
        };
        if !term.is_empty() {
            terminator = term;
        }
        if row.shape == Shape::Empty {
            pending.push(term);
            continue;
        }
        let keep = if started || style.keep_empty_lines.at_start_of_file {
            style.max_empty_lines_to_keep
        } else {
            0
        };
        for t in pending.drain(..).take(keep) {
            empty_line(&mut out, t);
        }
        started = true;
        out.extend_from_slice(&text(row, src, a, info, d, style));
        out.extend_from_slice(term);
    }
    if style.keep_empty_lines.at_end_of_file {
        for t in pending.into_iter().take(style.max_empty_lines_to_keep) {
            empty_line(&mut out, t);
        }
    }
    if style.insert_newline_at_eof && !matches!(out.last(), None | Some(b'\n' | b'\r')) {
        let mut with = out.clone();
        with.extend_from_slice(ending.unwrap_or(terminator));
        let tokens = |text: &[u8]| {
            let a = lex::analyze(text, d, style.tab_width);
            a.tokens
                .iter()
                .map(|t| (t.kind, t.start, t.end))
                .collect::<Vec<_>>()
        };
        if tokens(&out) == tokens(&with) {
            return with;
        }
    }
    out
}

fn empty_line(out: &mut Vec<u8>, term: &[u8]) {
    if out.last() == Some(&b'\r') && term.first() == Some(&b'\n') {
        out.push(b' ');
    }
    out.extend_from_slice(term);
}

fn text(
    row: &Row,
    src: &[u8],
    a: &Analysis,
    info: &LineInfo,
    d: &Dialect,
    style: &Style,
) -> Vec<u8> {
    let line = info.line;
    match row.shape {
        Shape::Fixed => return src[line.start..line.end].to_vec(),
        Shape::Empty => return Vec::new(),
        _ => {}
    }
    let end = info.last.min(info.cut);
    let mut out = row.out.clone();
    let mut col = row.col;
    if let Some(note) = &row.note {
        let t = a.tokens[note.token];
        for &b in &src[t.start..t.end.min(end)] {
            col = advance(col, b, style.tab_width);
        }
        out.extend_from_slice(&src[t.start..t.end.min(end)]);
    }
    out.extend_from_slice(&src[row.rest.0..row.rest.1]);
    for &b in &src[row.rest.0..row.rest.1] {
        col = advance(col, b, style.tab_width);
    }
    if row.tail {
        out.extend_from_slice(&src[info.last..line.end]);
    }
    if let (Some(column), Shape::Statement | Shape::Comment) = (d.column, row.shape) {
        if info.open {
            return src[line.start..info.last].to_vec();
        }
        if info.cut < info.last {
            if col > column {
                return src[line.start..info.last].to_vec();
            }
            out.extend(std::iter::repeat_n(b' ', column - col));
            out.extend_from_slice(&src[info.cut..info.last]);
        }
    }
    out
}

fn inside_literals(a: &Analysis) -> Vec<bool> {
    let spans: Vec<(usize, usize)> = a
        .tokens
        .iter()
        .filter(|t| t.kind == Kind::Literal)
        .map(|t| (t.start, t.end))
        .collect();
    let mut k = 0;
    a.lines
        .iter()
        .map(|info| {
            let at = info.line.end;
            while k < spans.len() && spans[k].1 <= at {
                k += 1;
            }
            k < spans.len() && spans[k].0 <= at
        })
        .collect()
}

fn line_ending(
    src: &[u8],
    a: &Analysis,
    inside: &[bool],
    ending: LineEnding,
) -> Option<&'static [u8]> {
    let count = || {
        let (mut lf, mut crlf) = (0, 0);
        for (info, &inside) in a.lines.iter().zip(inside) {
            match &src[info.line.end..info.line.next] {
                _ if inside => {}
                b"\n" => lf += 1,
                b"\r\n" => crlf += 1,
                _ => {}
            }
        }
        (lf, crlf)
    };
    match ending {
        LineEnding::Keep => None,
        LineEnding::Lf => Some(b"\n"),
        LineEnding::Crlf => Some(b"\r\n"),
        LineEnding::DeriveLf => {
            let (lf, crlf) = count();
            Some(if crlf > lf { b"\r\n" } else { b"\n" })
        }
        LineEnding::DeriveCrlf => {
            let (lf, crlf) = count();
            Some(if lf > crlf { b"\n" } else { b"\r\n" })
        }
    }
}

fn disabled(src: &[u8], a: &Analysis) -> Vec<bool> {
    let mut off = false;
    a.lines
        .iter()
        .map(|info| {
            let mut on = false;
            for t in &a.tokens[info.tokens.clone()] {
                if t.kind == Kind::Comment {
                    match src[t.text.0..t.text.1].trim_ascii() {
                        b"asm-format off" => off = true,
                        b"asm-format on" => on = true,
                        _ => {}
                    }
                }
            }
            let line = off;
            if on {
                off = false;
            }
            line
        })
        .collect()
}

fn column(src: &[u8], start: usize, pos: usize, tab_width: usize) -> usize {
    src[start..pos]
        .iter()
        .fold(0, |col, &b| advance(col, b, tab_width))
}

fn operand_fields(
    src: &[u8],
    s: usize,
    e: usize,
    tokens: &[Token],
    d: &Dialect,
    style: &Style,
) -> Vec<Vec<u8>> {
    let text = &src[s..e];
    if !style.space_after_comma || d.positional {
        return vec![text.to_vec()];
    }
    let inside: Vec<&Token> = tokens
        .iter()
        .filter(|t| t.start >= s && t.start < e)
        .collect();
    let token_at = |i: usize| inside.iter().find(|t| t.start == i).copied();
    let mut depth = 0i32;
    let mut i = s;
    while i < e {
        if let Some(t) = token_at(i) {
            i = t.end;
            continue;
        }
        match src[i] {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth -= 1,
            _ => {}
        }
        if depth < 0 {
            return vec![text.to_vec()];
        }
        i += 1;
    }
    if depth != 0 {
        return vec![text.to_vec()];
    }
    let mut fields = Vec::new();
    let mut field = Vec::new();
    let mut i = s;
    while i < e {
        if let Some(t) = token_at(i) {
            field.extend_from_slice(&src[t.start..t.end.min(e)]);
            i = t.end;
            continue;
        }
        let b = src[i];
        if b == b'(' || b == b'[' || b == b'{' {
            depth += 1;
        } else if b == b')' || b == b']' || b == b'}' {
            depth -= 1;
        }
        field.push(b);
        i += 1;
        if b != b',' || depth != 0 {
            continue;
        }
        let mut j = i;
        while j < e && is_ws(src[j]) && token_at(j).is_none() {
            j += 1;
        }
        let keep = j >= e
            || token_at(j).is_some_and(|t| matches!(t.kind, Kind::Comment | Kind::Continuation));
        if keep {
            continue;
        }
        fields.push(std::mem::take(&mut field));
        i = j;
    }
    fields.push(field);
    fields
}
