//! Pure row/cell projection fixes representation differences at the single engine boundary.
use crate::{
    Capture, CaptureRange, Cell, Column, LineCount, LineIndex, MAX_CAPTURE_BYTES, Row, ScreenError,
};

pub(crate) fn cell(screen: &vt100::Screen, row: Row, column: Column) -> Option<Cell<'_>> {
    let (rows, cols) = screen.size();
    if row.0 >= rows.get() || column.0 >= cols.get() {
        return None;
    }
    let body = screen.cell(row.0, column.0)?;
    let lead = if body.is_wide_continuation() {
        column
            .0
            .checked_sub(1)
            .and_then(|col| screen.cell(row.0, col))
            .filter(|cell| cell.is_wide())
            .unwrap_or(body)
    } else {
        body
    };
    Some(Cell::of(body, lead))
}

pub(crate) fn buffer_cell(
    screen: &vt100::Screen,
    line: LineIndex,
    column: Column,
) -> Option<Cell<'_>> {
    let index = usize::try_from(line.0).ok()?;
    let body = screen.buffer_cell(index, column.0)?;
    let lead = if body.is_wide_continuation() {
        column
            .0
            .checked_sub(1)
            .and_then(|col| screen.buffer_cell(index, col))
            .filter(|cell| cell.is_wide())
            .unwrap_or(body)
    } else {
        body
    };
    Some(Cell::of(body, lead))
}

pub(crate) fn frame_cell(
    screen: &vt100::Screen,
    line: LineIndex,
    column: Column,
) -> Option<Cell<'_>> {
    let index = usize::try_from(line.0).ok()?;
    let stored_columns = screen.buffer_row_width(index)?;
    // History trims trailing default cells. Project that implicit padding only
    // after establishing the row exists; missing rows must remain unavailable.
    if column.0 >= stored_columns {
        Some(Cell::blank())
    } else {
        buffer_cell(screen, line, column)
    }
}

pub(crate) fn capture(screen: &vt100::Screen, range: CaptureRange) -> Result<Capture, ScreenError> {
    let total = screen.buffer_len();
    let total = LineCount(u32::try_from(total).map_err(|_overflow| ScreenError::CaptureTooLarge)?);
    let mut lines = Vec::new();
    let mut bytes = 0_usize;
    if let Some(range) = range.resolve(total) {
        for index in range {
            let index = usize::try_from(index).map_err(|_overflow| ScreenError::CaptureTooLarge)?;
            let remaining = MAX_CAPTURE_BYTES
                .checked_sub(bytes)
                .and_then(|left| left.checked_sub(1))
                .ok_or(ScreenError::CaptureTooLarge)?;
            let line = bounded_line(screen, index, remaining)?;
            bytes = bytes
                .checked_add(line.len())
                .and_then(|n| n.checked_add(1))
                .ok_or(ScreenError::CaptureTooLarge)?;
            lines.push(line);
        }
    }
    Ok(Capture { lines, total })
}

// Admit each append before allocating: even an unusually wide row cannot
// allocate an oversized temporary string before the response budget is checked.
fn bounded_line(
    screen: &vt100::Screen,
    index: usize,
    remaining: usize,
) -> Result<String, ScreenError> {
    let cols = screen
        .buffer_row_width(index)
        .ok_or(ScreenError::Unavailable)?;
    let mut line = String::new();
    let mut blanks = 0_usize;
    let mut continuation = false;
    for column in 0..cols {
        if continuation {
            continuation = false;
            continue;
        }
        let cell = screen
            .buffer_cell(index, column)
            .ok_or(ScreenError::Unavailable)?;
        continuation = cell.is_wide();
        if cell.has_contents() {
            let needed = line
                .len()
                .checked_add(blanks)
                .and_then(|n| n.checked_add(cell.contents().len()))
                .ok_or(ScreenError::CaptureTooLarge)?;
            if needed > remaining {
                return Err(ScreenError::CaptureTooLarge);
            }
            line.extend(std::iter::repeat_n(' ', blanks));
            line.push_str(cell.contents());
            blanks = 0;
        } else {
            blanks = blanks.saturating_add(if continuation { 2 } else { 1 });
        }
    }
    // The suffix length comes from a borrowed valid UTF-8 prefix, so truncate
    // cannot split a code point. It avoids allocating another full line.
    let kept = line.trim_end_matches(js_whitespace).len();
    line.truncate(kept);
    Ok(line)
}

// JS trimEnd (Paseo's capture contract) differs from Rust's Unicode White_Space:
// ECMAScript trims BOM and excludes NEL. Do not normalize the captured row text.
pub(crate) const fn js_whitespace(ch: char) -> bool {
    matches!(ch,'\u{0009}'..='\u{000d}'|'\u{0020}'|'\u{00a0}'|'\u{1680}'|'\u{2000}'..='\u{200a}'|'\u{2028}'|'\u{2029}'|'\u{202f}'|'\u{205f}'|'\u{3000}'|'\u{feff}')
}
