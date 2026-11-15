//! Isolated loopback speech fixtures; no real provider, credentials or audio device.

use super::*;
use axum::{
    Router,
    body::{Body, Bytes},
    http::{HeaderMap, Response, StatusCode},
    routing::{get, post},
};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::voice::fixture::{SYNTHETIC_KEY, Server};

impl Server {
    async fn endpoint(&self) -> Endpoint {
        Endpoint::connect(
            &self.base,
            None,
            None,
            &ResolvedValue::new(SYNTHETIC_KEY.to_owned()),
        )
        .await
        .unwrap()
    }
}

fn text() -> SpeechText {
    SpeechText::try_from("fixture utterance".to_owned()).unwrap()
}

#[tokio::test]
async fn default_request_and_pcm_response_keep_key_in_header_and_redact_debug() {
    let router = Router::new().route(
        "/v1/audio/speech",
        post(|headers: HeaderMap, body: Bytes| async move {
            assert!(
                headers.get(header::AUTHORIZATION).unwrap().as_bytes()
                    == format!("Bearer {SYNTHETIC_KEY}").as_bytes(),
                "wrong fixture credential"
            );
            assert!(
                !body
                    .windows(SYNTHETIC_KEY.len())
                    .any(|part| part == SYNTHETIC_KEY.as_bytes())
            );
            let fields: serde_json::Value = serde_json::from_slice(&body).unwrap();
            assert_eq!(fields.get("model").unwrap(), "tts-1");
            assert_eq!(fields.get("voice").unwrap(), "alloy");
            assert_eq!(fields.get("response_format").unwrap(), "pcm");
            Response::builder()
                .header(header::CONTENT_TYPE, "audio/pcm")
                .body(Body::from(vec![0, 1, 0, 2]))
                .unwrap()
        }),
    );
    let server = Server::start(router).await;
    let endpoint = server.endpoint().await;
    assert!(!format!("{endpoint:?}").contains(SYNTHETIC_KEY));
    assert!(!format!("{endpoint:?}").contains("127.0.0.1"));
    let audio = endpoint
        .synthesize(&text(), &CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(audio.format.rate().get(), 24_000);
    assert_eq!(audio.pcm.as_slice(), [0, 1, 0, 2]);
    assert!(!format!("{audio:?}").contains("[0, 1"));
    server.finish().await;
}

#[tokio::test]
async fn redirects_and_rejections_never_retry_or_reach_a_destination() {
    let requests = Arc::new(AtomicUsize::new(0));
    let seen = Arc::clone(&requests);
    let destinations = Arc::new(AtomicUsize::new(0));
    let destination = Arc::clone(&destinations);
    let router = Router::new()
        .route(
            "/v1/audio/speech",
            post(move || {
                let seen = Arc::clone(&seen);
                async move {
                    let _previous = seen.fetch_add(1, Ordering::AcqRel);
                    Response::builder()
                        .status(StatusCode::TEMPORARY_REDIRECT)
                        .header(header::LOCATION, "/private-destination")
                        .body(Body::from("private provider response"))
                        .unwrap()
                }
            }),
        )
        .route(
            "/private-destination",
            get(move || {
                let destination = Arc::clone(&destination);
                async move {
                    let _previous = destination.fetch_add(1, Ordering::AcqRel);
                    StatusCode::OK
                }
            }),
        );
    let server = Server::start(router).await;
    let error = server
        .endpoint()
        .await
        .synthesize(&text(), &CancellationToken::new())
        .await
        .unwrap_err();
    assert_eq!(error, TtsError::Rejected);
    assert_eq!(requests.load(Ordering::Acquire), 1);
    assert_eq!(destinations.load(Ordering::Acquire), 0);
    assert!(!format!("{error:?}").contains("private"));
    server.finish().await;
}

#[tokio::test]
async fn malformed_and_oversized_response_bodies_cannot_become_playback() {
    for (body, media, wanted) in [
        (Vec::new(), "audio/pcm", TtsError::Format),
        (vec![0; 3], "audio/pcm", TtsError::Format),
        (
            b"private response".to_vec(),
            "application/json",
            TtsError::Format,
        ),
        (
            vec![0; MAX_PLAYBACK_BYTES.saturating_add(2)],
            "audio/pcm",
            TtsError::Capacity,
        ),
    ] {
        let router = Router::new().route(
            "/v1/audio/speech",
            post(move || {
                let body = body.clone();
                async move {
                    Response::builder()
                        .header(header::CONTENT_TYPE, media)
                        .body(Body::from(body))
                        .unwrap()
                }
            }),
        );
        let server = Server::start(router).await;
        assert_eq!(
            server
                .endpoint()
                .await
                .synthesize(&text(), &CancellationToken::new())
                .await
                .unwrap_err(),
            wanted
        );
        server.finish().await;
    }
}

#[tokio::test]
async fn bad_credentials_and_oversized_segments_refuse_before_network_work() {
    for key in ["", "invalid\r\nprivate"] {
        assert_eq!(
            Endpoint::connect(
                &EndpointBase::openai(),
                None,
                None,
                &ResolvedValue::new(key.to_owned())
            )
            .await
            .unwrap_err(),
            TtsError::Credential
        );
    }
    let requests = Arc::new(AtomicUsize::new(0));
    let seen = Arc::clone(&requests);
    let router = Router::new().route(
        "/v1/audio/speech",
        post(move || {
            let seen = Arc::clone(&seen);
            async move {
                let _previous = seen.fetch_add(1, Ordering::AcqRel);
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }),
    );
    let server = Server::start(router).await;
    let long = SpeechText::try_from("x".repeat(MAX_SEGMENT_UNITS.saturating_add(1))).unwrap();
    assert_eq!(
        server
            .endpoint()
            .await
            .synthesize(&long, &CancellationToken::new())
            .await
            .unwrap_err(),
        TtsError::Segment
    );
    assert_eq!(requests.load(Ordering::Acquire), 0);
    server.finish().await;
}

#[test]
fn custom_model_and_voice_remain_distinct_at_the_request_edge() {
    let model = "custom-synthesis".parse::<TtsModel>().unwrap();
    let voice = "custom-speaker".parse::<TtsVoice>().unwrap();
    let request = Request {
        model: &model,
        voice: &voice,
        input: "synthetic fixture",
        response_format: ResponseFormat::Pcm,
    };
    let wire = serde_json::to_string(&request).unwrap();
    assert!(wire.contains("\"model\":\"custom-synthesis\""));
    assert!(wire.contains("\"voice\":\"custom-speaker\""));
    assert!(!format!("{model:?} {voice:?}").contains("custom-"));
}
