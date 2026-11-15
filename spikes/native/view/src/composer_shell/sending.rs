//! Gesture identity only; receipt/delivery admission remains with the session actor.

/// The send button/shortcut uses either its ordinary or alternate action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Ordinary button press or configured shortcut.
    Default,
    /// Long press or eligible modified Enter.
    Alternate,
}
