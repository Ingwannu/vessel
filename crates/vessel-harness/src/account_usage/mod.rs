//! Typed additional vendor response boundaries; no credential or network I/O here.
//! Existing Claude/Codex parsers remain the single owners of their schemas.
mod common;
mod decode;
pub(crate) use common::Number;
pub use decode::AccountUsageDecodeFailure;
mod copilot;
mod cursor;
mod grok;
mod kimi;
mod minimax;
mod opencode_go;
mod zai;
use vessel_proto::{
    limits::InvalidLimitText,
    usage_reports::{
        AvailableUsageReport, BuiltinUsageSource, InvalidUsageCollection, UsageProblem,
    },
};
/// A whole vendor body is bounded before parsing any collection or metadata.
pub const MAX_USAGE_VENDOR_BYTES: usize = 1_048_576;
/// A parsed response or an explicit subscription refusal; neither guesses an allowance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountUsageOutcome {
    /// A sanitized report, including explicit unknowns and completeness.
    Available(AvailableUsageReport),
    /// The vendor explicitly reports no current usable subscription.
    Unavailable(UsageProblem),
}
/// Private parser causes stay available without echoing provider response bodies.
#[derive(Debug, Clone, thiserror::Error)]
pub enum AccountUsageError {
    /// The fixed body budget is not a truncation boundary.
    #[error("the account usage response exceeds its supported size")]
    TooLarge,
    /// This entry point never claims another parser's sources.
    #[error("this account source parser is not available")]
    Unsupported,
    /// A provider application error has no reviewed public classification.
    #[error("the account usage response changed or reports an unsupported failure")]
    ChangedResponse,
    /// Source format changed; no raw JSON is placed in the display string.
    #[error("the account usage response could not be decoded")]
    Decode(#[source] AccountUsageDecodeFailure),
    /// Metadata is refused as a whole rather than shortened into another identity.
    #[error("account usage metadata exceeds its supported shape")]
    Text(#[from] InvalidLimitText),
    /// Count or identity bounds apply to the entire report.
    #[error("account usage entries exceed their supported bounds")]
    Collection(#[from] InvalidUsageCollection),
}
/// Decode supported additional vendors with synthetic or already bounded HTTP bytes.
/// # Errors
/// Unsupported sources, oversized bodies, changed formats or invalid report metadata.
pub fn decode_account_usage(
    source: BuiltinUsageSource,
    body: &[u8],
) -> Result<AccountUsageOutcome, AccountUsageError> {
    if body.len() > MAX_USAGE_VENDOR_BYTES {
        return Err(AccountUsageError::TooLarge);
    }
    match source {
        BuiltinUsageSource::Copilot => copilot::decode(body),
        BuiltinUsageSource::Cursor => cursor::decode(body),
        BuiltinUsageSource::Grok => grok::decode(body),
        BuiltinUsageSource::Kimi => kimi::decode(body),
        BuiltinUsageSource::MiniMax => minimax::decode(body),
        BuiltinUsageSource::OpenCodeGo => opencode_go::decode(body),
        BuiltinUsageSource::Zai => zai::decode(body),
        BuiltinUsageSource::Claude | BuiltinUsageSource::Codex => {
            Err(AccountUsageError::Unsupported)
        }
    }
}
