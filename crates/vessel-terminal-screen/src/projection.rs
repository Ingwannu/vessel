//! Pure row/cell projection fixes representation differences at the single engine boundary.
use crate::{Capture, CaptureRange, Cell, Column, LineCount, MAX_CAPTURE_BYTES, Row, ScreenError};

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

pub(crate) fn capture(screen: &vt100::Screen, range: CaptureRange) -> Result<Capture, ScreenError> {
    let total = screen.buffer_len();
    let total = LineCount(u32::try_from(total).map_err(|_overflow| ScreenError::CaptureTooLarge)?);
    let mut lines = Vec::new();
    let mut bytes = 0_usize;
    if let Some(range) = range.resolve(total) {
        for index in range {
            let index = usize::try_from(index).map_err(|_overflow| ScreenError::CaptureTooLarge)?;
            let line = screen.buffer_line(index).ok_or(ScreenError::Unavailable)?;
            let line = line.trim_end_matches(js_whitespace).to_owned();
            bytes = bytes
                .checked_add(line.len())
                .and_then(|n| n.checked_add(1))
                .ok_or(ScreenError::CaptureTooLarge)?;
            if bytes > MAX_CAPTURE_BYTES {
                return Err(ScreenError::CaptureTooLarge);
            }
            lines.push(line);
        }
    }
    Ok(Capture { lines, total })
}

// JS trimEnd (Paseo's capture contract) differs from Rust's Unicode White_Space:
// ECMAScript trims BOM and excludes NEL. Do not normalize the captured row text.
const fn js_whitespace(ch: char) -> bool {
    matches!(ch,'\u{0009}'..='\u{000d}'|'\u{0020}'|'\u{00a0}'|'\u{1680}'|'\u{2000}'..='\u{200a}'|'\u{2028}'|'\u{2029}'|'\u{202f}'|'\u{205f}'|'\u{3000}'|'\u{feff}')
}
