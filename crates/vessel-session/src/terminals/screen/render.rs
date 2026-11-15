//! Capture follows Paseo's translated row contents, including empty lines and clamping.
use super::{CaptureError, MAX_CAPTURE_BYTES};
use vessel_proto::{
    TerminalId,
    terminals::{CaptureLineCount, CaptureOptions, TerminalCapture},
};

pub(super) fn capture(
    terminal: TerminalId,
    screen: &vt100::Screen,
    options: CaptureOptions,
) -> Result<TerminalCapture, CaptureError> {
    let mut screen = screen.clone();
    let (rows, cols) = screen.size();
    let history = screen.scrollback_len();
    let total = history
        .checked_add(usize::from(rows.get()))
        .ok_or(CaptureError::TooLarge)?;
    let total_lines =
        CaptureLineCount::new(u32::try_from(total).map_err(|_too_many| CaptureError::TooLarge)?);
    let Some(range) = options.range.resolve(total_lines) else {
        return Ok(TerminalCapture {
            terminal,
            lines: Vec::new(),
            total_lines,
        });
    };
    let mut lines = Vec::new();
    let mut bytes = 0_usize;
    for index in range {
        let index = usize::try_from(index).map_err(|_range| CaptureError::TooLarge)?;
        let row = if index < history {
            screen.set_scrollback(history.saturating_sub(index));
            0
        } else {
            screen.set_scrollback(0);
            index.saturating_sub(history)
        };
        let row = u16::try_from(row).map_err(|_range| CaptureError::Unavailable)?;
        // xterm's translateToString(true).trimEnd returns text even when
        // stripAnsi=false, so style preservation must not invent SGR bytes.
        let line = screen
            .rows(0, cols.get())
            .nth(usize::from(row))
            .ok_or(CaptureError::Unavailable)?
            .trim_end()
            .to_owned();
        bytes = bytes
            .checked_add(line.len())
            .and_then(|count| count.checked_add(1))
            .ok_or(CaptureError::TooLarge)?;
        if bytes > MAX_CAPTURE_BYTES {
            return Err(CaptureError::TooLarge);
        }
        lines.push(line);
    }
    Ok(TerminalCapture {
        terminal,
        lines,
        total_lines,
    })
}
