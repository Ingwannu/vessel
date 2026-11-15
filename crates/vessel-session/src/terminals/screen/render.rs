//! Map shared translated rows to wire DTOs; no parser, ANSI renderer or range resolver.
use super::CaptureError;
use vessel_proto::{
    TerminalId,
    terminals::{CaptureLineCount, CaptureOptions, CaptureStyle, TerminalCapture},
};
use vessel_terminal_screen::{Capture, CaptureRange, LineOffset, ScreenError};

pub(super) fn range(options: CaptureOptions) -> CaptureRange {
    // Both reference modes return translateToString/trimEnd rows, not synthesized SGR.
    match options.style {
        CaptureStyle::Plain | CaptureStyle::Preserve => CaptureRange {
            start: options.range.start.map(|line| LineOffset::from(line.get())),
            end: options.range.end.map(|line| LineOffset::from(line.get())),
        },
    }
}
pub(super) fn capture(
    terminal: TerminalId,
    captured: Result<Capture, ScreenError>,
) -> Result<TerminalCapture, CaptureError> {
    let captured = captured.map_err(CaptureError::from)?;
    Ok(TerminalCapture {
        terminal,
        lines: captured.lines,
        total_lines: CaptureLineCount::new(captured.total.get()),
    })
}
