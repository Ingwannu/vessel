//! Local lifecycle intent and UI correlation, never network admission authority.
//! Canonical Residents alone mints the operation capability used for effects.
/// A lifecycle result is correlated independently from navigation lookups.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LifecycleTicket(u64);
impl LifecycleTicket {
    /// First local UI decision; this is never a resident operation capability.
    pub const FIRST: Self = Self(1);

    /// Never recycle a decision after exhaustion: a delayed callback can survive.
    #[must_use]
    pub const fn next(self) -> Option<Self> {
        match self.0.checked_add(1) {
            Some(next) => Some(Self(next)),
            None => None,
        }
    }
}
/// The user's intended effect after the authoritative node result arrives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleAction {
    /// Keep the conversation available in history.
    Archive,
    /// Restore visibility without starting a provider.
    Unarchive,
    /// Continue the same provider conversation.
    Resume,
    /// Permanently delete an ended session, behind confirmation.
    Delete,
    /// Archive before removing the visible session.
    Exit,
    /// Archive before opening a new draft with the same launch configuration.
    Fresh,
}
