//! Task diagnostics retain the concrete panic/cancellation privately.
//! The public error chain exposes only its closed task outcome, not a payload.
use std::fmt;

/// A provider worker failed. A panic payload is retained for ownership/evidence,
/// but never appears in public Debug/Display or as an Error source.
pub struct ProviderTaskFailure(tokio::task::JoinError);

impl From<tokio::task::JoinError> for ProviderTaskFailure {
    fn from(cause: tokio::task::JoinError) -> Self {
        Self(cause)
    }
}
impl fmt::Display for ProviderTaskFailure {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_panic() {
            out.write_str("provider task panicked")
        } else if self.0.is_cancelled() {
            out.write_str("provider task cancelled")
        } else {
            out.write_str("provider task did not finish")
        }
    }
}
impl fmt::Debug for ProviderTaskFailure {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, out)
    }
}
impl std::error::Error for ProviderTaskFailure {}

pub(super) fn report_owner(failure: &ProviderTaskFailure) {
    tracing::error!(%failure, "provider owner ended unexpectedly");
}

#[cfg(test)]
mod tests;
