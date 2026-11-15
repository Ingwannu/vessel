//! Transient node-selected voice context is separate from persisted core policy.
//! It is deliberately not deserializable from ordinary environment or prompt data.

/// Explicit per-session voice context; actual playback still requires its node lease.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum VoiceTools {
    /// Ordinary agent launches expose no voice tool.
    #[default]
    Disabled,
    /// The authenticated launch/reload explicitly selected voice context.
    Enabled,
}
