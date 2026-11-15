//! Document owners report quit admission without granting save/discard authority.

/// Every retained document and original write ticket must settle before Ready.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentReadiness {
    /// No unsaved modifications or unsettled writes remain.
    Ready,
    /// Explicit saving or discarding is still required in the editor.
    Dirty,
    /// A captured write or uncertain completion is still in flight.
    Pending,
    /// The editor retains modifications after a failed write.
    Failed,
}
