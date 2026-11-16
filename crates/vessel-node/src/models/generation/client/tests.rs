//! Recorded exchange outcomes and virtual time; no sockets or credentials.

use super::*;
use serde_json::json;
use std::{
    error::Error,
    sync::{
        Arc,
        atomic::{AtomicU32, Ordering},
    },
};
use vessel_llm::{
    client::{Api, ApiKey},
    completion::CompletionError,
};

fn prepared() -> Prepared {
    Prepared::fixture(Api::AnthropicMessages, Bytes::from_static(b"{}"))
}

fn endpoint() -> Endpoint {
    Endpoint {
        api: Api::AnthropicMessages,
        base_url: "http://not-called.invalid/v1".parse().unwrap(),
        key: None,
    }
}

fn body(text: &str) -> Bytes {
    Bytes::from(serde_json::to_vec(&json!({
        "id":"msg_fixture", "type":"message", "role":"assistant", "model":"fixture-model",
        "content":[{"type":"text", "text":text}], "stop_reason":"end_turn", "stop_sequence":null,
        "usage":{"input_tokens":17,"output_tokens":7}
    })).unwrap())
}

fn recorded(reply: Result<Bytes, ir::Failure>) -> (Client, Arc<Recorded>) {
    let record = Arc::new(Recorded {
        reply,
        calls: AtomicU32::new(0),
    });
    (
        Client {
            transport: Transport::Recorded(Arc::clone(&record)),
        },
        record,
    )
}

#[test]
fn a_proxy_or_retrying_client_is_refused_before_use() {
    let wrong = HttpClient::with_roots(rustls::RootCertStore::empty(), Limits::default()).unwrap();
    assert!(matches!(Client::new(wrong), Err(ClientError::Policy)));
    let mut retrying = policy();
    retrying.retries = TITLE_ATTEMPTS;
    let wrong = HttpClient::with_roots(rustls::RootCertStore::empty(), retrying).unwrap();
    assert!(matches!(Client::new(wrong), Err(ClientError::Policy)));
    let http = HttpClient::with_roots(rustls::RootCertStore::empty(), policy()).unwrap();
    let client = Client::new(http).unwrap();
    assert_eq!(format!("{client:?}"), "MetadataClient(..)");
    assert_eq!(policy().connect, TITLE_CONNECT_LIMIT);
    assert_eq!(policy().max_body, MAX_COMPLETION_BYTES);
    assert_eq!(policy().retries.saturating_add(1), TITLE_ATTEMPTS);
}

#[tokio::test]
async fn one_recorded_call_keeps_accounting_and_normal_text() {
    let (client, record) = recorded(Ok(body(r#"{"title":"Repair fixtures"}"#)));
    let result = client.complete(&endpoint(), prepared()).await.unwrap();
    assert_eq!(record.calls.load(Ordering::SeqCst), TITLE_ATTEMPTS);
    assert_eq!(result.usage.input, 17);
    assert_eq!(result.usage.output, 7);
    assert_eq!(result.stop, Some(ir::StopReason::EndTurn));
    assert!(result.text.unwrap().as_str().contains("Repair fixtures"));
}

#[tokio::test]
async fn content_and_byte_refusals_do_not_retry_the_exchange() {
    let cases = [
        (body(""), CompletionError::Empty),
        (
            Bytes::from(vec![b'x'; MAX_COMPLETION_BYTES.saturating_add(1)]),
            CompletionError::TooLarge,
        ),
    ];
    for (bytes, expected) in cases {
        let (client, record) = recorded(Ok(bytes));
        let result = client.complete(&endpoint(), prepared()).await.unwrap();
        assert_eq!(record.calls.load(Ordering::SeqCst), TITLE_ATTEMPTS);
        assert_eq!(
            std::mem::discriminant(&result.text.unwrap_err()),
            std::mem::discriminant(&expected)
        );
    }
    let (client, record) = recorded(Ok(body("")));
    let result = client.complete(&endpoint(), prepared()).await.unwrap();
    assert_eq!(result.usage.input, 17);
    assert_eq!(result.usage.output, 7);
    assert_eq!(record.calls.load(Ordering::SeqCst), TITLE_ATTEMPTS);
}

#[tokio::test]
async fn provider_failure_is_typed_and_sealed_without_a_retry() {
    let (client, record) = recorded(Err(ir::Failure::new(
        ir::FailureClass::RateLimited,
        "private-provider-marker",
    )));
    let error = client.complete(&endpoint(), prepared()).await.unwrap_err();
    assert_eq!(record.calls.load(Ordering::SeqCst), TITLE_ATTEMPTS);
    let ClientError::Provider(cause) = &error else {
        panic!("expected provider failure");
    };
    assert_eq!(cause.class(), ir::FailureClass::RateLimited);
    assert!(!format!("{error} {error:?} {cause}").contains("private-provider-marker"));
    assert!(error.source().unwrap().source().is_none());
}

#[tokio::test(start_paused = true)]
async fn the_total_deadline_bounds_a_stalled_exchange() {
    let entered = Arc::new(tokio::sync::Notify::new());
    let client = Client {
        transport: Transport::Stalled(Arc::clone(&entered)),
    };
    let start = tokio::time::Instant::now();
    let call = tokio::spawn(async move { client.complete(&endpoint(), prepared()).await });
    entered.notified().await;
    let error = call.await.unwrap().unwrap_err();
    assert!(matches!(error, ClientError::Deadline(_)));
    assert_eq!(start.elapsed(), TITLE_CALL_LIMIT);
    assert!(
        error
            .to_string()
            .contains(&TITLE_CALL_LIMIT.as_secs().to_string())
    );
}

#[tokio::test]
async fn the_endpoint_key_checks_the_complete_body_without_logging_it() {
    let key = "synthetic-private-key-0123456789";
    let (client, record) = recorded(Ok(body(&format!("{{\"title\":\"{key}\"}}"))));
    let mut endpoint = endpoint();
    endpoint.key = Some(ApiKey::new(key.to_owned()));
    let result = client.complete(&endpoint, prepared()).await.unwrap();
    assert!(matches!(result.text, Err(CompletionError::QuotedKey)));
    assert!(!format!("{result:?}").contains(key));
    assert_eq!(record.calls.load(Ordering::SeqCst), TITLE_ATTEMPTS);
}

#[tokio::test]
async fn an_api_mismatch_is_refused_without_an_attempt() {
    let (client, record) = recorded(Ok(body("unused")));
    let mut endpoint = endpoint();
    endpoint.api = Api::OpenAiChat;
    let error = client.complete(&endpoint, prepared()).await.unwrap_err();
    assert!(matches!(error, ClientError::ApiMismatch));
    assert_eq!(record.calls.load(Ordering::SeqCst), 0);
}
