//! Transient provider context. It travels with one prompt command and is
//! never emitted as the user's message or kept for a later prompt.

use std::fmt;
use std::sync::Arc;

use agent_client_protocol::v1::schema as acp;

use crate::InputError;

/// Node-prepared, bounded context for one side question (ADR 0045).
#[derive(Clone)]
pub struct PromptContext(Arc<str>);

impl PromptContext {
    /// Parse at the trusted caller's edge. The driver additionally bounds
    /// the actual serialized provider request before sending anything.
    ///
    /// # Errors
    /// Blank context or more than the side snapshot's UTF-8 byte bound.
    pub fn new(text: String) -> Result<Self, InputError> {
        if text.trim().is_empty() || text.len() > vessel_proto::side_questions::SNAPSHOT_BYTES {
            return Err(InputError::ContextLimit);
        }
        Ok(Self(Arc::from(text)))
    }

    /// The provider's quoted context; not a user message.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for PromptContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PromptContext")
            .field("bytes", &self.0.len())
            .finish()
    }
}

/// The transport adds these JSON-RPC fields to `PromptRequest`. Use its
/// reserved id rather than assuming a UUID width; this must track the SDK
/// stdio envelope if that protocol changes. No content is emitted on refusal.
pub(crate) fn check_request(
    request: &acp::PromptRequest,
    id: &acp::RequestId,
) -> Result<(), InputError> {
    let envelope = acp::JsonRpcMessage::wrap(acp::Request {
        id: id.clone(),
        method: Arc::from("session/prompt"),
        params: Some(request),
    });
    if serde_json::to_vec(&envelope)
        .map_err(InputError::InvalidContent)?
        .len()
        > vessel_proto::side_questions::PROMPT_BYTES
    {
        return Err(InputError::ContextLimit);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_is_bounded_in_bytes_and_debug_does_not_reveal_it() {
        assert!(PromptContext::new("한".repeat(8_192)).is_ok());
        assert!(PromptContext::new("한".repeat(8_193)).is_err());
        assert!(PromptContext::new(" ".to_owned()).is_err());
        assert_eq!(
            format!("{:?}", PromptContext::new("private".to_owned()).unwrap()),
            "PromptContext { bytes: 7 }"
        );
    }
    #[test]
    fn full_envelope_escaping_and_utf8_count_at_the_limit() {
        let id = acp::RequestId::Str("request-id-with-\"escaping\"".to_owned());
        let make = |text: String| {
            acp::PromptRequest::new(
                acp::SessionId::new("session-id"),
                vec![acp::ContentBlock::Text(acp::TextContent::new(text))],
            )
        };
        let prefix = "한\n\"".repeat(100);
        let base = make(prefix.clone());
        let envelope = acp::JsonRpcMessage::wrap(acp::Request {
            id: id.clone(),
            method: Arc::from("session/prompt"),
            params: Some(&base),
        });
        let room = vessel_proto::side_questions::PROMPT_BYTES
            - serde_json::to_vec(&envelope).unwrap().len();
        let exact = format!("{prefix}{}", "x".repeat(room));
        assert!(check_request(&make(exact.clone()), &id).is_ok());
        assert!(matches!(
            check_request(&make(format!("{exact}x")), &id),
            Err(InputError::ContextLimit)
        ));
        assert!(matches!(
            check_request(&make(format!("{exact}한")), &id),
            Err(InputError::ContextLimit)
        ));
    }
}
