//! Opaque provider identifiers still need a bounded command-line decoding edge.

/// A CLI token is deliberately smaller than a prompt; IDs contain no controls.
const MAX_IDENTIFIER_CHARS: usize = 1024;

#[derive(Debug, thiserror::Error)]
#[error(
    "identifier must be nonempty, contain no control characters, and fit {MAX_IDENTIFIER_CHARS} characters"
)]
pub(super) struct InvalidIdentifier;
fn checked(text: &str) -> Result<&str, InvalidIdentifier> {
    if text.is_empty()
        || text.chars().count() > MAX_IDENTIFIER_CHARS
        || text.chars().any(char::is_control)
    {
        Err(InvalidIdentifier)
    } else {
        Ok(text)
    }
}
pub(super) fn option(text: &str) -> Result<vessel_proto::OptionValueId, InvalidIdentifier> {
    checked(text.trim()).map(vessel_proto::OptionValueId::new)
}
pub(super) fn model(text: &str) -> Result<vessel_proto::ModelId, InvalidIdentifier> {
    checked(text).map(vessel_proto::ModelId::new)
}
pub(super) fn conversation(
    text: &str,
) -> Result<vessel_proto::HarnessSessionId, InvalidIdentifier> {
    checked(text).map(vessel_proto::HarnessSessionId::new)
}
