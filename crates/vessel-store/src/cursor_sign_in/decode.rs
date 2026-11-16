//! Credential parser diagnostics retain their cause without reopening its input.
use std::fmt;

pub(super) struct CursorDecodeFailure {
    cause: serde_json::Error,
}

impl CursorDecodeFailure {
    pub(super) const fn new(cause: serde_json::Error) -> Self {
        Self { cause }
    }
}

impl fmt::Display for CursorDecodeFailure {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            out,
            "Cursor sign-in JSON {:?} error at line {}, column {}",
            self.cause.classify(),
            self.cause.line(),
            self.cause.column()
        )
    }
}

impl fmt::Debug for CursorDecodeFailure {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, out)
    }
}

// StoreError and io::Error expose this safe boundary, never serde's quoted input.
impl std::error::Error for CursorDecodeFailure {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error as _;

    #[test]
    fn the_actual_sqlite_legacy_cause_is_retained_privately() {
        let marker = "PRIVATE-CURSOR-RETAINED-CANARY";
        let body = format!("\"{marker}\"");
        let (_root, path) = super::super::tests::fixture(&body);
        let crate::StoreError::Io { source, .. } =
            crate::read_cursor_usage_sign_in(&path).unwrap_err()
        else {
            panic!("expected the legacy credential parser refusal");
        };
        let failure = source
            .get_ref()
            .unwrap()
            .downcast_ref::<CursorDecodeFailure>()
            .unwrap();
        assert!(failure.cause.to_string().contains(marker));
        assert_eq!(failure.cause.classify(), serde_json::error::Category::Data);
        assert!(failure.source().is_none());
        assert_eq!(source.kind(), std::io::ErrorKind::Other);
    }
}
