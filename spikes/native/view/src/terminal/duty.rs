//! Which viewer may type a live query's reply.
//!
//! Paseo's daemon answers with no viewer in the decision. This app has no
//! parser on the node, so one view answers, or none do. The arguments are
//! enums so a call cannot swap "focused" with "has typed".

/// Where this view's terminal came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalOrigin {
    /// This view created the terminal.
    Opened,
    /// This view attached, or the view that created it has closed.
    Attached,
}

/// Whether this view's window currently holds focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewerFocus {
    /// The window is focused.
    Held,
    /// The window is not focused.
    Lost,
}

/// Whether the person has typed in this view.
///
/// Typing sticks: the first key is enough, as [`super::frame::ResizeRule`]
/// already records it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeysTyped {
    /// Nothing typed in this view yet.
    Never,
    /// At least one key.
    Once,
}

/// Whether some other viewer already qualifies under the resize rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OtherAnswerer {
    /// Another viewer has focus and has typed. The link sets this on every
    /// viewer except the one it chose, so two windows cannot both answer.
    Qualifies,
    /// No other viewer qualifies.
    None,
}

/// Whether this view types the reply.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryDuty {
    /// Apply the bytes with [`super::grid::TerminalModel::feed`].
    Answer,
    /// Apply the bytes with [`super::grid::TerminalModel::replay`].
    ///
    /// When every viewer is silent, the program that asked waits.
    Silent,
}

/// Who types a live query's reply.
///
/// A viewer that has focus and has typed answers, unless the link has
/// already given that job to another. Until anyone qualifies, the view
/// that created the terminal answers, even with focus lost and nothing
/// typed. An attach with nobody qualified stays silent.
#[must_use]
pub const fn query_duty(
    origin: TerminalOrigin,
    focus: ViewerFocus,
    typed: KeysTyped,
    other: OtherAnswerer,
) -> QueryDuty {
    match other {
        OtherAnswerer::Qualifies => QueryDuty::Silent,
        OtherAnswerer::None => match (focus, typed, origin) {
            (
                ViewerFocus::Held,
                KeysTyped::Once,
                TerminalOrigin::Opened | TerminalOrigin::Attached,
            ) => QueryDuty::Answer,
            (
                ViewerFocus::Held | ViewerFocus::Lost,
                KeysTyped::Never | KeysTyped::Once,
                TerminalOrigin::Opened,
            ) => QueryDuty::Answer,
            (ViewerFocus::Held, KeysTyped::Never, TerminalOrigin::Attached)
            | (ViewerFocus::Lost, KeysTyped::Never | KeysTyped::Once, TerminalOrigin::Attached) => {
                QueryDuty::Silent
            }
        },
    }
}
