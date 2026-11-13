//! The meaning of an area's notice, without a toolkit type. Alerts have
//! five meanings in Paseo; a badge's four-state vocabulary is different.

/// Paseo's `AlertVariant`, chosen by pure area logic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Variant {
    /// A notice with no semantic icon or accent.
    Default,
    /// Informational guidance.
    Info,
    /// A successful outcome.
    Success,
    /// A warning the person should read before proceeding.
    Warning,
    /// A failed or refused action.
    Error,
}
