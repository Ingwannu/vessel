//! Unified hunk headers are the only source of patch line numbers. File
//! headers are skipped outside hunks; identical-looking source lines survive.

use super::{DiffLine, MAX_LINES, Side, split_lines};

struct Cursor {
    old: Option<usize>,
    new: Option<usize>,
    old_left: usize,
    new_left: usize,
}

impl Cursor {
    const fn finished(&self) -> bool {
        self.old_left == 0 && self.new_left == 0
    }

    fn number(&mut self, side: Side) -> Option<usize> {
        let number = match side {
            Side::Removed => self.old,
            Side::Added | Side::Context => self.new,
            Side::Header => return None,
        };
        if (side != Side::Added && self.old_left == 0)
            || (side != Side::Removed && self.new_left == 0)
        {
            return None;
        }
        if side != Side::Added {
            self.old_left = self.old_left.saturating_sub(1);
            self.old = self.old.and_then(|n| n.checked_add(1));
        }
        if side != Side::Removed {
            self.new_left = self.new_left.saturating_sub(1);
            self.new = self.new.and_then(|n| n.checked_add(1));
        }
        number
    }
}

/// Unified patches preserve source colors, hunk offsets and no-newline notes.
/// Invalid or exhausted hunk offsets stay unnumbered rather than being invented.
#[must_use]
pub fn unified_lines(text: &str) -> (Vec<DiffLine<'_>>, bool) {
    let input = split_lines(text);
    let cut = input.len() > MAX_LINES;
    let mut cursor: Option<Cursor> = None;
    let mut rows = Vec::new();
    for line in input.into_iter().take(MAX_LINES) {
        if line.starts_with("diff --git ") {
            cursor = None;
        } else if line.starts_with("@@") {
            cursor = hunk(line);
            rows.push(DiffLine {
                side: Side::Header,
                number: None,
                text: line,
            });
        } else if !(cursor.as_ref().is_none_or(Cursor::finished)
            && (line.starts_with("--- ") || line.starts_with("+++ ") || line.starts_with("index ")))
        {
            rows.push(source_line(line, cursor.as_mut()));
        }
    }
    (rows, cut)
}

fn source_line<'a>(line: &'a str, cursor: Option<&mut Cursor>) -> DiffLine<'a> {
    let (side, text) = if let Some(text) = line.strip_prefix('+') {
        (Side::Added, text)
    } else if let Some(text) = line.strip_prefix('-') {
        (Side::Removed, text)
    } else if let Some(text) = line.strip_prefix(' ') {
        (Side::Context, text)
    } else {
        (Side::Header, line)
    };
    DiffLine {
        side,
        number: cursor.and_then(|cursor| cursor.number(side)),
        text,
    }
}

fn range(text: &str) -> Option<(usize, usize)> {
    let mut parts = text.split(',');
    let start = parts.next()?.parse().ok()?;
    // A range without an explicit count represents one line in unified syntax.
    let count = parts
        .next()
        .map(str::parse::<usize>)
        .transpose()
        .ok()?
        .unwrap_or(1);
    parts.next().is_none().then_some((start, count))
}

fn hunk(header: &str) -> Option<Cursor> {
    let body = header.strip_prefix("@@ -")?;
    let (old, rest) = body.split_once(' ')?;
    let (new, end) = rest.strip_prefix('+')?.split_once(' ')?;
    if !end.starts_with("@@") {
        return None;
    }
    let (old, old_left) = range(old)?;
    let (new, new_left) = range(new)?;
    Some(Cursor {
        old: Some(old),
        new: Some(new),
        old_left,
        new_left,
    })
}
