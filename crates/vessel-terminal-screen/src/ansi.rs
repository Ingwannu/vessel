//! A bounded physical-display serializer, never mutable-engine restoration or a query source.
use crate::{
    Attributes, Cell, Color, Column, Cursor, FrameCursor, FrameView, InputModes, ModeState, Row,
    ScreenError, Size, Style, Visibility, Width,
};
use std::fmt::Write as _;

/// Serialized display bytes are admitted before the caller can write any of them.
pub const MAX_ANSI_FRAME_BYTES: usize = 1024 * 1024;
/// Display output only; consumers must never feed it back into the source engine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnsiFrame(Vec<u8>);
impl AnsiFrame {
    /// One physical stdout write after complete validation/encoding succeeds.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
    /// Pair once with the CLI's existing raw-mode reset guard, never once per frame.
    #[must_use]
    pub fn enter_display() -> Self {
        // One local Kitty stack entry preserves the host flags while snapshots replace them.
        Self(b"\x1b[>0u\x1b[?1049h\x1b[?25l".to_vec())
    }
}
struct Output {
    text: String,
    refusal: Option<ScreenError>,
}
impl Output {
    fn new() -> Self {
        Self {
            text: String::new(),
            refusal: None,
        }
    }
    fn push(&mut self, args: std::fmt::Arguments<'_>) -> Result<(), ScreenError> {
        self.write_fmt(args)
            .map_err(|_format_signal| self.refusal.unwrap_or(ScreenError::DisplayTooLarge))
    }
    fn finish(self) -> AnsiFrame {
        AnsiFrame(self.text.into_bytes())
    }
}
impl std::fmt::Write for Output {
    fn write_str(&mut self, text: &str) -> std::fmt::Result {
        if self.refusal.is_some() {
            return Err(std::fmt::Error);
        }
        if self
            .text
            .len()
            .checked_add(text.len())
            .is_none_or(|next| next > MAX_ANSI_FRAME_BYTES)
            || self.text.try_reserve(text.len()).is_err()
        {
            self.refusal = Some(ScreenError::DisplayTooLarge);
            return Err(std::fmt::Error);
        }
        self.text.push_str(text);
        Ok(())
    }
}

/// Paint one already-selected authoritative viewport without a cell-grid clone.
///
/// # Errors
/// Refuses malformed cells/cursor geometry or an oversized serialized display update.
pub fn paint_frame(frame: &FrameView<'_>) -> Result<AnsiFrame, ScreenError> {
    let size = Size::new(frame.columns(), frame.rows().get())?;
    paint_cells(size, frame.cursor(), frame.input_modes(), |row, column| {
        frame.cell(row, column).ok_or(ScreenError::Unavailable)
    })
}

/// Share one renderer between engine projections and a validated wire-frame cell adapter.
/// The callback supplies readonly cells, never raw terminal tokens or mutable decoder state.
///
/// # Errors
/// Refuses a missing/inconsistent cell, invalid cursor, or encoded-byte overflow before output.
pub fn paint_cells<'a>(
    size: Size,
    cursor: FrameCursor,
    modes: InputModes,
    mut cell: impl FnMut(Row, Column) -> Result<Cell<'a>, ScreenError>,
) -> Result<AnsiFrame, ScreenError> {
    validate_cursor(size, cursor)?;
    let mut output = Output::new();
    // Explicit positioning plus disabled wrapping protects the bottom-right cell from scrolling.
    output.push(format_args!("\x1b[?25l\x1b[?7l\x1b[0m\x1b[2J"))?;
    let mut painter = Painter {
        output,
        columns: size.cols(),
        ink: Attributes::new(Color::Default, Color::Default, []),
        continuation: None,
    };
    for row in 0..size.rows() {
        painter.continuation = None;
        for column in 0..size.cols() {
            painter.cell(row, column, cell(Row(row), Column(column))?)?;
        }
    }
    let mut output = painter.output;
    output.push(format_args!("\x1b[0m\x1b[?7h"))?;
    modes_output(&mut output, modes)?;
    if let FrameCursor::Present(cursor) = cursor {
        position(
            &mut output,
            cursor.row.0,
            cursor.column.0.min(size.cols().saturating_sub(1)),
        )?;
        match cursor.visibility {
            Visibility::Shown => output.push(format_args!("\x1b[?25h"))?,
            Visibility::Hidden => {}
        }
    }
    Ok(output.finish())
}
struct Painter {
    output: Output,
    columns: u16,
    ink: Attributes,
    continuation: Option<Attributes>,
}
impl Painter {
    fn cell(&mut self, row: u16, column: u16, body: Cell<'_>) -> Result<(), ScreenError> {
        crate::cell::validate_display_text(body.text(), body.width())?;
        match body.width() {
            Width::Continuation => {
                if self.continuation.take() != Some(body.attributes()) {
                    return Err(ScreenError::CellContents);
                }
                return Ok(());
            }
            Width::One | Width::Two if self.continuation.is_some() => {
                return Err(ScreenError::CellContents);
            }
            Width::One | Width::Two => {}
        }
        let clipped = body.width() == Width::Two && column.saturating_add(1) == self.columns;
        if body.width() == Width::Two && !clipped {
            self.continuation = Some(body.attributes());
        }
        if body.text().is_empty()
            && body.attributes() == Attributes::new(Color::Default, Color::Default, [])
        {
            return Ok(());
        }
        position(&mut self.output, row, column)?;
        if self.ink != body.attributes() {
            attributes(&mut self.output, body.attributes())?;
            self.ink = body.attributes();
        }
        let text = if clipped || body.text().is_empty() {
            " "
        } else {
            body.text()
        };
        self.output.push(format_args!("{text}"))
    }
}
fn validate_cursor(size: Size, cursor: FrameCursor) -> Result<(), ScreenError> {
    match cursor {
        FrameCursor::OutsideViewport => Ok(()),
        FrameCursor::Present(Cursor { row, column, .. })
            if row.0 < size.rows() && column.0 <= size.cols() =>
        {
            Ok(())
        }
        FrameCursor::Present(_) => Err(ScreenError::Geometry),
    }
}
fn position(output: &mut Output, row: u16, column: u16) -> Result<(), ScreenError> {
    output.push(format_args!(
        "\x1b[{};{}H",
        u32::from(row).saturating_add(1),
        u32::from(column).saturating_add(1)
    ))
}
fn attributes(output: &mut Output, ink: Attributes) -> Result<(), ScreenError> {
    output.push(format_args!("\x1b[0"))?;
    for (style, code) in [
        (Style::Bold, 1),
        (Style::Dim, 2),
        (Style::Italic, 3),
        (Style::Underline, 4),
        (Style::Inverse, 7),
    ] {
        if ink.has(style) {
            output.push(format_args!(";{code}"))?;
        }
    }
    color(output, Channel::Foreground, ink.foreground())?;
    color(output, Channel::Background, ink.background())?;
    output.push(format_args!("m"))
}
enum Channel {
    Foreground,
    Background,
}
fn color(output: &mut Output, channel: Channel, color: Color) -> Result<(), ScreenError> {
    let (plain, indexed) = match channel {
        Channel::Foreground => (39, 38),
        Channel::Background => (49, 48),
    };
    match color {
        Color::Default => output.push(format_args!(";{plain}")),
        Color::Indexed(index) => output.push(format_args!(";{indexed};5;{index}")),
        Color::Rgb(red, green, blue) => {
            output.push(format_args!(";{indexed};2;{red};{green};{blue}"))
        }
    }
}
fn modes_output(output: &mut Output, modes: InputModes) -> Result<(), ScreenError> {
    output.push(format_args!("\x1b[={};1u", modes.kitty.get()))?;
    for (code, state) in [
        (1, modes.application_cursor),
        (2004, modes.bracketed_paste),
        (9001, modes.win32),
    ] {
        let final_byte = match state {
            ModeState::Enabled => 'h',
            ModeState::Disabled => 'l',
        };
        output.push(format_args!("\x1b[?{code}{final_byte}"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
