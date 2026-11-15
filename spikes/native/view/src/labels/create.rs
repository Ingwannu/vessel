//! Create-and-assign draft lifetime: a failure never discards what was typed.

use super::{Color, Failure, HostState, Label, Name, Ticket};

/// Uncommitted input; the name is validated only when submitting.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct CreateDraft {
    text: String,
    color: Color,
}

impl CreateDraft {
    /// Input is retained verbatim to avoid changing the user's cursor/text.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }
    /// Swatch choice does not commit a definition.
    #[must_use]
    pub const fn color(&self) -> Color {
        self.color
    }
    /// Name input remains a draft until `label` validates it.
    pub fn set_text(&mut self, text: &str) {
        self.text = text.to_owned();
    }
    /// Choosing a swatch is separate from create-and-assign.
    pub const fn set_color(&mut self, color: Color) {
        self.color = color;
    }
    /// A typed payload can be sent only when a nonempty normalized name exists.
    #[must_use]
    pub fn label(&self) -> Option<Label> {
        Some(Label {
            name: Name::parse(&self.text).ok()?,
            color: self.color,
        })
    }
}

/// Picker pages; create success returns to the still-open list.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum PickerPage {
    /// Multi-select assignment list.
    #[default]
    Listing,
    /// Editable create form.
    Creating(CreateDraft),
    /// Create and assign are one operation, not two optimistic writes.
    Submitting {
        /// Correlation identity.
        ticket: Ticket,
        /// Draft retained through failure.
        draft: CreateDraft,
    },
}

impl PickerPage {
    /// Create form starts with violet and no name; opening sends nothing.
    /// # Errors
    /// A pending create must settle before its form can be replaced.
    pub fn create(&mut self) -> Result<(), Failure> {
        match self {
            Self::Submitting { .. } => Err(Failure::Pending),
            Self::Listing | Self::Creating(_) => {
                *self = Self::Creating(CreateDraft::default());
                Ok(())
            }
        }
    }
    /// Only idle form input can change.
    pub fn draft_mut(&mut self) -> Option<&mut CreateDraft> {
        match self {
            Self::Creating(draft) => Some(draft),
            Self::Listing | Self::Submitting { .. } => None,
        }
    }
    /// Validate then reserve one create-and-assign intent.
    /// # Errors
    /// Refuses missing/empty input, pending work and an unavailable host.
    pub fn submit(&mut self, host: HostState, ticket: Ticket) -> Result<Label, Failure> {
        host.writable()?;
        match self {
            Self::Creating(draft) => {
                let label = draft.label().ok_or(Failure::NoChange)?;
                *self = Self::Submitting {
                    ticket,
                    draft: draft.clone(),
                };
                Ok(label)
            }
            Self::Listing => Err(Failure::NoChange),
            Self::Submitting { .. } => Err(Failure::Pending),
        }
    }
    /// Ignore stale replies; failure leaves the form exactly as typed.
    #[must_use]
    pub fn completed(&mut self, ticket: Ticket, result: Result<(), Failure>) -> bool {
        let Self::Submitting {
            ticket: pending,
            draft,
        } = self
        else {
            return false;
        };
        if *pending != ticket {
            return false;
        }
        *self = match result {
            Ok(()) => Self::Listing,
            Err(_failure_shown_by_picker) => Self::Creating(draft.clone()),
        };
        true
    }
}
