//! Serde can quote vendor input, so the retained cause must stop at this edge.
use std::{fmt, sync::Arc};

/// Input-free diagnostics backed by the original, privately retained JSON error.
///
/// There is deliberately no raw accessor or public source: ordinary error-chain
/// logging must not recover a response that serde included in its own message.
#[derive(Clone)]
pub struct AccountUsageDecodeFailure {
    cause: Arc<serde_json::Error>,
}

impl AccountUsageDecodeFailure {
    pub(super) fn new(cause: serde_json::Error) -> Self {
        Self {
            cause: Arc::new(cause),
        }
    }
}

impl fmt::Display for AccountUsageDecodeFailure {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            out,
            "account usage JSON {:?} error at line {}, column {}",
            self.cause.classify(),
            self.cause.line(),
            self.cause.column()
        )
    }
}

impl fmt::Debug for AccountUsageDecodeFailure {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, out)
    }
}

impl std::error::Error for AccountUsageDecodeFailure {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AccountUsageError, decode_account_usage};
    use std::error::Error as _;
    use vessel_proto::usage_reports::BuiltinUsageSource;

    #[test]
    fn the_actual_vendor_cause_is_retained_behind_the_safe_clone() {
        let marker = "PRIVATE-VENDOR-RETAINED-CANARY";
        let input = format!("\"{marker}\"");
        let AccountUsageError::Decode(failure) =
            decode_account_usage(BuiltinUsageSource::Copilot, input.as_bytes()).unwrap_err()
        else {
            panic!("expected the vendor decode boundary");
        };
        assert!(failure.cause.to_string().contains(marker));
        assert_eq!(failure.cause.classify(), serde_json::error::Category::Data);
        assert!(failure.source().is_none());
        let cloned = failure.clone();
        assert!(Arc::ptr_eq(&failure.cause, &cloned.cause));
        assert!(!format!("{cloned:?}").contains(marker));
    }
}
