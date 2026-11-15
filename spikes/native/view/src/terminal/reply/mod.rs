//! The closed set of queries a terminal answers (ADR 0073), and the bytes
//! of each answer (Paseo's, `packages/server/src/terminal/terminal.ts`
//! 45–49 and 883–922).
//!
//! [Decision Log]
//! - Purpose: a program that asks the terminal something (nvim's device
//!   attributes, a shell's cursor position) waits for the answer; a wrong
//!   or repeated answer is typed into the program as if the person had.
//! - Choice: five shapes are answered and nothing else: primary device
//!   attributes, operating status, the cursor's cell (both forms), and the
//!   three OSC colours with an argument of exactly `?`, answered with the
//!   colours the grid is drawn in. `query` matches every action with no
//!   catch-all arm, so a variant added to the parser is unanswered until
//!   someone lists it here; ENQ is listed as never answered.
//! - Differences from Paseo: the OSC colours are the palette shown, not
//!   Paseo's fixed dark values; the cursor's cell is the grid's (a pending
//!   wrap reports the last column), counted from the screen's top as
//!   Paseo's `cursorY` is, also in origin mode.
//! - Bound: the longest answer to the shortest query of its kind is DA1's
//!   11 bytes for `CSI c`'s 3, so one call's replies are under
//!   [`REPLY_RATIO`] times its bytes, plus one answer (at most
//!   [`LONGEST_REPLY`] bytes) to a query the call before began (a test
//!   floods each kind).

#[cfg(test)]
mod tests;

use super::grid::{CursorCell, Rgb};
use super::paint::Palette;
use super::parse::{Action, Attributes, ColourSlot, Control, Csi, Dcs, Esc, Osc, Status};

/// One call's replies are under this many times its bytes (DA1: 11 for
/// 3; an OSC colour: 25 for 7; a cursor report at 1024 × 512: 12 for 4).
pub const REPLY_RATIO: usize = 4;

/// The longest reply: an OSC colour, `ESC ] 1x ; rgb:rrrr/gggg/bbbb ESC \`.
pub const LONGEST_REPLY: usize = 25;

/// A query the terminal answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Query {
    /// `CSI c`, `CSI 0 c`.
    Attributes,
    /// `CSI 5 n`.
    Operating,
    /// `CSI 6 n`.
    Cursor,
    /// `CSI ? 6 n`.
    PrivateCursor,
    /// OSC 10, 11 or 12 with `?`.
    Colour(ColourSlot),
}

/// What `action` asks, if it is in the answered set.
pub(super) const fn query(action: &Action) -> Option<Query> {
    match action {
        Action::Csi(csi) => csi_query(csi),
        Action::Osc(Osc::ColourQuery(slot)) => Some(Query::Colour(*slot)),
        // Never answered: a clipboard read (the program would read the
        // person's clipboard); ENQ (an answerback string); DECRQSS and
        // XTGETTCAP. The rest ask nothing.
        Action::Osc(Osc::ClipboardRead | Osc::Title(_) | Osc::Other)
        | Action::Control(
            Control::Enquiry
            | Control::Bell
            | Control::Backspace
            | Control::Tab
            | Control::LineFeed
            | Control::CarriageReturn
            | Control::Other,
        )
        | Action::Dcs(Dcs::RequestSetting | Dcs::TermcapQuery | Dcs::Other)
        | Action::Print(_)
        | Action::Esc(
            Esc::Index
            | Esc::NextLine
            | Esc::SetTab
            | Esc::ReverseIndex
            | Esc::SaveCursor
            | Esc::RestoreCursor
            | Esc::Reset
            | Esc::Other,
        ) => None,
    }
}

/// What a control sequence asks, if it is in the answered set.
const fn csi_query(csi: &Csi) -> Option<Query> {
    match csi {
        Csi::Attributes(Attributes::Primary) => Some(Query::Attributes),
        Csi::Status(Status::Operating) => Some(Query::Operating),
        Csi::Status(Status::Cursor) => Some(Query::Cursor),
        Csi::Status(Status::PrivateCursor) => Some(Query::PrivateCursor),
        // Never answered: DA2 and DA3, window reports (`t`), DECRQM and
        // other status codes. The rest ask nothing.
        Csi::Attributes(Attributes::Other)
        | Csi::RefusedQuery
        | Csi::Up(_)
        | Csi::Down(_)
        | Csi::Forward(_)
        | Csi::Back(_)
        | Csi::NextLine(_)
        | Csi::PreviousLine(_)
        | Csi::Column(_)
        | Csi::Goto(..)
        | Csi::Row(_)
        | Csi::EraseDisplay(_)
        | Csi::EraseLine(_)
        | Csi::InsertLines(_)
        | Csi::DeleteLines(_)
        | Csi::InsertChars(_)
        | Csi::DeleteChars(_)
        | Csi::EraseChars(_)
        | Csi::ScrollUp(_)
        | Csi::ScrollDown(_)
        | Csi::TabForward(_)
        | Csi::TabBack(_)
        | Csi::ClearTabs(_)
        | Csi::ScrollRegion(..)
        | Csi::Modes(..)
        | Csi::Graphic(_)
        | Csi::SaveCursor
        | Csi::RestoreCursor
        | Csi::CursorStyle
        | Csi::Unsupported => None,
    }
}

/// Write the answer to `query` to `out`: the cursor's cell where it asks,
/// the drawn colours where they are known (with no palette given yet, a
/// colour query is not answered).
pub(super) fn answer(
    query: Query,
    cursor: CursorCell,
    palette: Option<&Palette>,
    out: &mut Vec<u8>,
) {
    let row = u32::from(cursor.row).saturating_add(1);
    let col = u32::from(cursor.col).saturating_add(1);
    match query {
        Query::Attributes => out.extend_from_slice(b"\x1b[?62;4;22c"),
        Query::Operating => out.extend_from_slice(b"\x1b[0n"),
        Query::Cursor => out.extend_from_slice(format!("\x1b[{row};{col}R").as_bytes()),
        Query::PrivateCursor => out.extend_from_slice(format!("\x1b[?{row};{col}R").as_bytes()),
        Query::Colour(slot) => {
            if let Some(palette) = palette {
                let (code, rgb) = match slot {
                    ColourSlot::Foreground => (10, palette.foreground),
                    ColourSlot::Background => (11, palette.background),
                    ColourSlot::Cursor => (12, palette.cursor),
                };
                let Rgb { r, g, b } = rgb;
                let colour =
                    format!("\x1b]{code};rgb:{r:02x}{r:02x}/{g:02x}{g:02x}/{b:02x}{b:02x}\x1b\\");
                out.extend_from_slice(colour.as_bytes());
            }
        }
    }
}
