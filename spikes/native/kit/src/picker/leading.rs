//! Existing leading data is re-exported unchanged; form slot is an explicit opt-in.
use crate::assets::Glyph;
use gpui::{Hsla, SharedString};
/// What leads a row.
#[derive(Debug, Clone, PartialEq)]
pub enum Leading {
    /// A glyph, muted.
    Glyph(Glyph),
    /// A glyph in the foreground: a permissions mode's mark (Paseo's
    /// `ModeComboboxOption` draws its `DensityIcon` with `color="foreground"`).
    Mark(Glyph),
    /// A 14-point leading glyph, as Paseo’s branch-switcher `renderOption`.
    SmallGlyph(Glyph),
    /// An official agent mark, or the generic bot when unknown.
    AgentMark(Option<crate::agent_mark::Artwork>),
    /// A state's dot (a machine).
    Dot(Hsla),
    /// A project's mark: its initial on a tile ([`super::initial_tile`]).
    Initial(SharedString),
    /// Form options keep a muted 16-point glyph inside an 18-point slot.
    /// Pinned schedule-form-sheet.tsx967–1020/1066–1071.
    FormGlyph(Glyph),
}
