//! Request correlation and safe UI failures; label values have one protocol owner.

pub(super) use vessel_proto::workspace_labels::LabelKey as Key;

/// Form policy is independent of the toolkit's field implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditAccess {
    /// Connected capable host and idle form; catalog error is shown separately.
    Editable,
    /// Keep selection/copy while refusing user edits and color changes.
    ReadOnly,
}
pub use vessel_proto::workspace_labels::{
    Assignment, Label, LabelColor as Color, LabelCount as Count, LabelName as Name,
    LabelNameError as NameError,
};

/// Correlation identity survives closing/reopening and host changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Ticket(u64);

/// A window's ticket source. Exhaustion refuses work rather than reusing identity.
#[derive(Debug, Default)]
pub struct Tickets(u64);

impl Tickets {
    /// Allocate the next operation identity.
    /// # Errors
    /// Refuses identity exhaustion.
    pub fn issue(&mut self) -> Result<Ticket, Failure> {
        self.0 = self.0.checked_add(1).ok_or(Failure::TicketExhausted)?;
        Ok(Ticket(self.0))
    }
}

/// Safe semantic errors; raw RPC diagnostics never become form labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Failure {
    /// The host is not connected.
    #[error("host is offline")]
    Offline,
    /// The selected alias or admission has been replaced.
    #[error("selected label connection changed")]
    Changed,
    /// The node has no assigned contract for this operation.
    #[error("host does not support workspace labels")]
    Unsupported,
    /// Another operation owns the surface or label.
    #[error("a label operation is pending")]
    Pending,
    /// Another definition has that normalized name.
    #[error("a label already uses this name")]
    NameTaken,
    /// The edited definition was removed.
    #[error("label was not found")]
    NotFound,
    /// No valid changed draft exists.
    #[error("there is no valid label change")]
    NoChange,
    /// A validated host response could not be loaded.
    #[error("labels could not be loaded")]
    Load,
    /// The operation failed without a more specific public reason.
    #[error("labels could not be updated")]
    Update,
    /// Every unique identity has been used.
    #[error("label operation identities are exhausted")]
    TicketExhausted,
}
