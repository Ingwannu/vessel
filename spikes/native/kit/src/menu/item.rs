//! Action labels and tone, independent of menu placement and drawing.

use gpui::SharedString;

/// A row's tone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// An ordinary action.
    Plain,
    /// The one destructive action (delete, kill), kept apart by a line.
    Danger,
}

/// One row of a menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuItem {
    /// Its words.
    pub label: SharedString,
    /// Its tone.
    pub tone: Tone,
    /// Shown but not chosen.
    pub disabled: bool,
    /// A hairline above it: it starts a group.
    pub starts_group: bool,
}

impl MenuItem {
    /// A plain row.
    #[must_use]
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            tone: Tone::Plain,
            disabled: false,
            starts_group: false,
        }
    }

    /// The destructive row, in a group of its own.
    #[must_use]
    pub fn danger(label: impl Into<SharedString>) -> Self {
        Self {
            tone: Tone::Danger,
            starts_group: true,
            ..Self::new(label)
        }
    }

    /// The same row, starting a group.
    #[must_use]
    pub fn grouped(self) -> Self {
        Self {
            starts_group: true,
            ..self
        }
    }
}
