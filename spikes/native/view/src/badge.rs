//! The semantic status an area's pure view logic chooses. Toolkit
//! colours and rendering stay outside this crate, so the same decision
//! applies on every surface (Paseo's `StatusBadgeVariant`).

/// The four semantic signals shared by every area's status labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Variant {
    /// Available, granted or successfully completed.
    Success,
    /// A state that needs attention.
    Warning,
    /// An error or refused operation.
    Error,
    /// Neutral information with no success or failure claim.
    Muted,
}
