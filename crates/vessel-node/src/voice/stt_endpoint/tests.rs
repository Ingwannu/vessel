use super::*;
use crate::voice::{
    fixture::{SYNTHETIC_KEY, Server},
    input_buffer::InputBuffer,
};
use axum::{
    Router,
    body::{Body, Bytes},
    http::{HeaderMap, Response, StatusCode},
    routing::post,
};
use vessel_proto::voice::{CapturePcm, Confidence};

fn utterance() -> Utterance {
    let mut buffer = InputBuffer::default();
    buffer
        .append(&CapturePcm::new(vec![0; 320]).unwrap())
        .unwrap();
    buffer.confirmed().unwrap();
    buffer.seal().unwrap()
}
async fn endpoint(server: &Server, model: &str) -> Endpoint {
    Endpoint::connect(
        &server.base,
        model.parse().unwrap(),
        Language::english(),
        &ResolvedValue::new(SYNTHETIC_KEY.to_owned()),
    )
    .await
    .unwrap()
}

#[tokio::test]
async fn voice_form_and_finite_logprob_policy_drive_the_actual_loopback_http_client() {
    let server = Server::start(Router::new().route("/v1/audio/transcriptions", post(|headers: HeaderMap, body: Bytes| async move {
        assert!(headers.get("authorization").unwrap().as_bytes() == format!("Bearer {SYNTHETIC_KEY}").as_bytes(), "wrong fixture credential");
        assert!(!body.windows(SYNTHETIC_KEY.len()).any(|bytes| bytes == SYNTHETIC_KEY.as_bytes()));
        assert!(body.windows(b"include[]".len()).any(|bytes| bytes == b"include[]"));
        assert!(!body.windows(b"name=\"prompt\"".len()).any(|bytes| bytes == b"name=\"prompt\""));
        Response::new(Body::from(r#"{"text":"private low confidence","logprobs":[{"token":"private","logprob":-4.0}]}"#))
    }))).await;
    let client = endpoint(&server, "gpt-4o-transcribe").await;
    assert!(!format!("{client:?}").contains(SYNTHETIC_KEY));
    assert!(!format!("{client:?}").contains("127.0.0.1"));
    let result = client
        .transcribe(&utterance(), &tokio_util::sync::CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(result, VoiceTranscript::Rejected);
    assert!(!format!("{result:?}").contains("private"));
    server.finish().await;
}

#[tokio::test]
async fn whisper_ignores_unrequested_scores_and_successful_text_debug_is_redacted() {
    let server = Server::start(Router::new().route(
        "/v1/audio/transcriptions",
        post(|body: Bytes| async move {
            assert!(
                !body
                    .windows(b"include[]".len())
                    .any(|bytes| bytes == b"include[]")
            );
            Response::new(Body::from(
                r#"{"text":"private accepted text","logprobs":[{"token":"x","logprob":-8.0}]}"#,
            ))
        }),
    ))
    .await;
    let result = endpoint(&server, "whisper-1")
        .await
        .transcribe(&utterance(), &tokio_util::sync::CancellationToken::new())
        .await
        .unwrap();
    assert!(matches!(
        result,
        VoiceTranscript::Accepted {
            confidence: Confidence::Unspecified,
            ..
        }
    ));
    assert!(!format!("{result:?}").contains("private"));
    server.finish().await;
}

#[tokio::test]
async fn rejected_and_oversized_foreign_responses_keep_no_diagnostic_payload() {
    for (status, size) in [
        (StatusCode::UNAUTHORIZED, 64),
        (StatusCode::OK, MAX_RESPONSE_BYTES.saturating_add(1)),
    ] {
        let server = Server::start(Router::new().route(
            "/v1/audio/transcriptions",
            post(move || async move {
                Response::builder()
                    .status(status)
                    .body(Body::from(vec![b'x'; size]))
                    .unwrap()
            }),
        ))
        .await;
        let error = endpoint(&server, "whisper-1")
            .await
            .transcribe(&utterance(), &tokio_util::sync::CancellationToken::new())
            .await
            .unwrap_err();
        assert!(matches!(
            error,
            SttError::Endpoint(EndpointError::Rejected | EndpointError::Capacity)
        ));
        assert!(!format!("{error:?}").contains(SYNTHETIC_KEY));
        server.finish().await;
    }
}
