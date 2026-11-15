//! Serde diagnostics can repeat arbitrary child field names and values.

use std::fmt;

/// A concrete JSON cause with a sealed input-bearing source chain. The original
/// parser error remains owned for classification/location, but cannot escape
/// through Debug, Display or `Error::source` into a scoped failure or trace.
pub struct PluginJsonError(serde_json::Error);

impl PluginJsonError {
    /// Seal an error at the plugin decoder/encoder boundary before it is logged
    /// or wrapped. This constructor never stringifies the rejected input.
    #[must_use]
    pub const fn new(cause: serde_json::Error) -> Self {
        Self(cause)
    }
}

impl fmt::Debug for PluginJsonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl fmt::Display for PluginJsonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "plugin JSON {:?} error at {}:{}",
            self.0.classify(),
            self.0.line(),
            self.0.column()
        )
    }
}

// The raw serde error can own helper-supplied keys. Exposing it as a source
// would undo this wrapper's diagnostic boundary.
impl std::error::Error for PluginJsonError {}
