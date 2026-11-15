//! Preserve concrete private causes without making them public `Error` sources.
//! Follows the reviewed settings diagnostic boundary; provider documents need
//! category/line/column only, not another path-tracking decoder dependency.
use crate::StoreError;
use std::fmt;

/// Safe file/codec evidence. The original cause remains private: neither `Debug`
/// nor `Error::source` can expose strings quoted by serde or private file paths.
pub struct ProvidersFileFailure(Cause);

enum Cause {
    Storage(StoreError),
    Json(serde_json::Error),
}

impl From<StoreError> for ProvidersFileFailure {
    fn from(cause: StoreError) -> Self {
        Self(Cause::Storage(cause))
    }
}
impl From<serde_json::Error> for ProvidersFileFailure {
    fn from(cause: serde_json::Error) -> Self {
        Self(Cause::Json(cause))
    }
}
impl fmt::Display for ProvidersFileFailure {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            Cause::Storage(cause) => {
                write!(out, "provider file storage failure ({:?})", cause.code())
            }
            Cause::Json(cause) => write!(
                out,
                "provider JSON {:?} at line {} column {}",
                cause.classify(),
                cause.line(),
                cause.column()
            ),
        }
    }
}
impl fmt::Debug for ProvidersFileFailure {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, out)
    }
}
impl std::error::Error for ProvidersFileFailure {}
