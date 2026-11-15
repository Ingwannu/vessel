//! Staged bounded schema validation/retry orchestration; no native call is invented.
//! W22 owns create/initial-turn schema transport. Only this adapter's schema leaf
//! may use Value, and only jsonschema interprets it. Caller errors abort retries.
mod response;
mod schema;
use schema::{ResponseError, Schema};
use std::future::Future;
use vessel_proto::RawJson;
const MAX_RETRIES: usize = 2;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Attempt {
    Initial,
    FirstRetry,
    SecondRetry,
}
impl Attempt {
    const ALL: [Self; MAX_RETRIES.saturating_add(1)] =
        [Self::Initial, Self::FirstRetry, Self::SecondRetry];
    fn retry_instruction(self) -> Option<&'static str> {
        match self {
            Self::Initial => None,
            Self::FirstRetry | Self::SecondRetry => Some(
                "Previous response was invalid. Respond again with JSON only that matches the schema.",
            ),
        }
    }
}
#[derive(Debug, thiserror::Error)]
enum GenerationError<E> {
    #[error("structured output caller failed")]
    Caller(#[source] E),
    #[error("structured response exceeds the CLI byte budget")]
    TooLarge,
    #[error("structured response exceeds the CLI fence processing budget")]
    FenceBudget,
    #[error("agent response did not match the required output schema after two retries")]
    Exhausted,
}
async fn generate<'a, F, Fut, E>(
    schema: &'a Schema,
    mut caller: F,
) -> Result<RawJson, GenerationError<E>>
where
    F: FnMut(Attempt, &'a RawJson) -> Fut,
    Fut: Future<Output = Result<String, E>>,
{
    for attempt in Attempt::ALL {
        let text = caller(attempt, schema.document())
            .await
            .map_err(GenerationError::Caller)?;
        let raw = match response::decode(&text) {
            Ok(raw) => raw,
            Err(response::DecodeError::TooLarge) => return Err(GenerationError::TooLarge),
            Err(response::DecodeError::FenceBudget) => return Err(GenerationError::FenceBudget),
            Err(response::DecodeError::Invalid) => continue,
        };
        match schema.validate(raw) {
            Ok(output) => return Ok(output),
            Err(ResponseError::Mismatch) => {}
            Err(ResponseError::TooLarge) => return Err(GenerationError::TooLarge),
        }
    }
    Err(GenerationError::Exhausted)
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::RefCell, io};
    fn schema() -> Schema {
        Schema::compile(RawJson::from_text(r#"{"type":"object","required":["ok"]}"#).unwrap())
            .unwrap()
    }
    #[tokio::test]
    async fn structured_retry_uses_two_retries_and_preserves_schema() {
        let schema = schema();
        let calls = RefCell::new(Vec::new());
        let result = generate(&schema, |attempt, document| {
            assert_eq!(document, schema.document());
            calls.borrow_mut().push(attempt);
            let response = match attempt {
                Attempt::Initial => "invalid",
                Attempt::FirstRetry => "{}",
                Attempt::SecondRetry => "```json\n{\"ok\":true}\n```",
            };
            assert_eq!(
                attempt.retry_instruction().is_some(),
                attempt != Attempt::Initial
            );
            std::future::ready(Ok::<_, io::Error>(response.to_owned()))
        })
        .await
        .unwrap();
        assert_eq!(calls.into_inner(), Attempt::ALL);
        assert_eq!(result.as_text(), r#"{"ok":true}"#);
    }
    #[tokio::test]
    async fn structured_retry_exhaustion_is_bounded_and_caller_refusal_is_not_retried() {
        let schema = schema();
        let calls = RefCell::new(Vec::new());
        let result = generate(&schema, |attempt, _| {
            calls.borrow_mut().push(attempt);
            std::future::ready(Ok::<_, io::Error>("{}".to_owned()))
        })
        .await;
        assert!(matches!(result, Err(GenerationError::Exhausted)));
        assert_eq!(calls.into_inner().len(), MAX_RETRIES.saturating_add(1));
        let calls = RefCell::new(Vec::new());
        let result = generate(&schema, |attempt, _| {
            calls.borrow_mut().push(attempt);
            std::future::ready(Err::<String, _>(io::Error::other(
                "permission/timeout abort",
            )))
        })
        .await;
        assert!(matches!(result, Err(GenerationError::Caller(_))));
        assert_eq!(calls.into_inner(), [Attempt::Initial]);
    }
}
