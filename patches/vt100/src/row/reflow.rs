//! Vessel patch 3: reflow physical rows with the cursor's logical run held.
//! Reference: exact xterm headless334 Buffer/BufferReflow; no ANSI is replayed.
use super::Row;
use alloc::vec::Vec;
use core::ops::RangeInclusive;

pub(crate) fn reflow_rows(
    rows: Vec<Row>,
    cursor: Option<usize>,
    old_cols: u16,
    cols: u16,
) -> (Vec<Row>, Vec<usize>) {
    let held = cursor.map(|cursor| cursor_run(&rows, cursor));
    let mut mapping = vec![0; rows.len()];
    let mut out = Vec::with_capacity(rows.len());
    let mut joined = Vec::new();
    let mut marks = Vec::new();
    let mut source = rows.into_iter().enumerate().peekable();
    while let Some((index, mut row)) = source.next() {
        if held.as_ref().is_some_and(|held| held.contains(&index)) {
            if !marks.is_empty() {
                cut(
                    core::mem::take(&mut joined),
                    core::mem::take(&mut marks),
                    cols,
                    &mut out,
                    &mut mapping,
                );
            }
            if let Some(at) = mapping.get_mut(index) {
                *at = out.len();
            }
            let wrapped = row.wrapped;
            if row.cells.len() > usize::from(cols) {
                row.truncate(cols);
            }
            row.wrapped = wrapped;
            out.push(row);
            continue;
        }
        marks.push((index, joined.len()));
        if row.wrapped {
            // History can store trimmed physical rows. Restore internal gaps,
            // except the single padding cell before a wide glyph wraps.
            row.cells.resize(usize::from(old_cols), crate::Cell::new());
            if row.cells.last().is_some_and(|cell| {
                !cell.has_contents() && !cell.is_wide_continuation()
            }) && source.peek().is_some_and(|(_, next)| {
                next.cells.first().is_some_and(crate::Cell::is_wide)
            }) {
                drop(row.cells.pop());
            }
        } else {
            let keep = row
                .cells
                .iter()
                .rposition(|cell| {
                    cell.has_contents() || cell.is_wide_continuation()
                })
                .map_or(0, |last| last.saturating_add(1));
            row.cells.truncate(keep);
        }
        joined.append(&mut row.cells);
        if !row.wrapped {
            cut(
                core::mem::take(&mut joined),
                core::mem::take(&mut marks),
                cols,
                &mut out,
                &mut mapping,
            );
        }
    }
    if !marks.is_empty() {
        cut(joined, marks, cols, &mut out, &mut mapping);
    }
    (out, mapping)
}

fn cursor_run(rows: &[Row], cursor: usize) -> RangeInclusive<usize> {
    let start = rows
        .get(..cursor)
        .unwrap_or_default()
        .iter()
        .rposition(|row| !row.wrapped)
        .map_or(0, |last| last.saturating_add(1));
    let end = rows
        .get(cursor..)
        .unwrap_or_default()
        .iter()
        .position(|row| !row.wrapped)
        .map_or(rows.len().saturating_sub(1), |after| {
            cursor.saturating_add(after)
        });
    start..=end
}

fn cut(
    cells: Vec<crate::Cell>,
    marks: Vec<(usize, usize)>,
    cols: u16,
    out: &mut Vec<Row>,
    mapping: &mut [usize],
) {
    let total = cells.len();
    let mut cells = cells.into_iter().peekable();
    let mut marks = marks.into_iter().peekable();
    let mut consumed = 0_usize;
    loop {
        let mut row = Vec::new();
        while row.len() < usize::from(cols) {
            if cols > 1
                && row.len().saturating_add(1) == usize::from(cols)
                && cells.peek().is_some_and(crate::Cell::is_wide)
            {
                break;
            }
            let Some(cell) = cells.next() else {
                break;
            };
            row.push(cell);
            consumed = consumed.saturating_add(1);
        }
        let wrapped = cells.peek().is_some();
        while marks.peek().is_some_and(|(_, offset)| {
            *offset < consumed || !wrapped && *offset <= total
        }) {
            if let Some((old, _)) = marks.next() {
                if let Some(at) = mapping.get_mut(old) {
                    *at = out.len();
                }
            }
        }
        if wrapped {
            row.resize(usize::from(cols), crate::Cell::new());
        }
        out.push(Row {
            cells: row,
            wrapped,
        });
        if !wrapped {
            break;
        }
    }
}
