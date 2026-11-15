//! Registry-only instructions; removal wording cannot describe a read failure.

/// Typed, translated outcomes of the existing registry read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Problem {
    /// Do not read through a book route that now points to another machine.
    NodeChanged,
    /// No worker slot is currently available.
    Busy,
    /// Reconnect before trying again.
    Offline,
    /// An absent older-peer method is not an empty registry.
    Unsupported,
    /// The node refused this device's access.
    Permission,
    /// Waiting ended, without asserting that the node's HTTP fetch stopped.
    Timeout,
    /// Keep the unmodified source for diagnostic details.
    Read,
    /// This browser has no node transport.
    NoTransport,
}
impl Problem {
    /// A pending read itself is neutral; only actual failures enter this alert.
    #[must_use]
    pub const fn variant(self) -> crate::alert::Variant {
        match self {
            Self::NodeChanged
            | Self::Busy
            | Self::Unsupported
            | Self::Permission
            | Self::NoTransport => crate::alert::Variant::Warning,
            Self::Offline | Self::Timeout | Self::Read => crate::alert::Variant::Error,
        }
    }
}
