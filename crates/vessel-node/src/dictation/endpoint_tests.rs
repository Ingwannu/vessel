//! Loopback HTTP only; assertions never print credentials, bodies or audio.

use super::super::{audio::PROVIDER_HZ, roots::Factory};
use super::*;
use axum::{
    Router,
    body::Body,
    http::{HeaderMap, Response, StatusCode, Uri},
    routing::post,
};
use bytes::Bytes;
use tokio::{net::TcpListener, task::JoinHandle};
use tokio_util::sync::CancellationToken;
use vessel_proto::dictation::DictationPcm;

const KEY: &str = "synthetic-private-credential";

/// Job tests reuse the actual HTTP fixture instead of copying server policy.
pub(in crate::dictation) struct Server {
    base: EndpointBase,
    stop: CancellationToken,
    task: Option<JoinHandle<Result<(), std::io::Error>>>,
}
impl Drop for Server {
    fn drop(&mut self) {
        self.stop.cancel();
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}
impl Server {
    /// Bind an isolated loopback route for the real endpoint and job clients.
    pub(in crate::dictation) async fn start(router: Router) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let stop = CancellationToken::new();
        let closing = stop.clone();
        let task = tokio::spawn(async move {
            axum::serve(listener, router)
                .with_graceful_shutdown(closing.cancelled_owned())
                .await
        });
        Self {
            base: format!("http://{address}/v1").parse().unwrap(),
            stop,
            task: Some(task),
        }
    }
    /// Exercise actual specification validation and client preparation in callers.
    pub(in crate::dictation) async fn endpoint(&self) -> Endpoint {
        let spec = Specification::new(
            &self.base,
            EndpointModel::whisper(),
            Language::english(),
            &ResolvedValue::new(KEY.to_owned()),
        )
        .unwrap();
        spec.connect(&Factory::default()).await.unwrap()
    }
    /// Join teardown so a completed test cannot retain its listener task.
    pub(in crate::dictation) async fn finish(mut self) {
        self.stop.cancel();
        self.task.take().unwrap().await.unwrap().unwrap();
    }
}

async fn within(test: impl Future<Output = ()>) {
    // Includes one real request budget plus setup; timer tests advance a clock.
    const TEST_LIMIT: Duration = Duration::from_secs(120);
    vessel_testkit::within(TEST_LIMIT, test).await.unwrap();
}

fn segment() -> Segment {
    Segment::from_test_pcm(DictationPcm::new(1_000_i16.to_le_bytes().to_vec()).unwrap())
}

#[tokio::test]
async fn the_fixed_form_sends_auth_only_in_the_header_and_a_provider_rate_wav() {
    within(async {
        let router = Router::new().route(
            "/v1/audio/transcriptions",
            post(|headers: HeaderMap, uri: Uri, body: Bytes| async move {
                let authorization = headers.get("authorization").unwrap().to_str().unwrap();
                assert!(
                    authorization == format!("Bearer {KEY}"),
                    "authorization was not the selected secret"
                );
                assert!(
                    !uri.to_string().contains(KEY),
                    "credential entered request target"
                );
                assert!(
                    !body.windows(KEY.len()).any(|part| part == KEY.as_bytes()),
                    "credential entered request body"
                );
                for field in [
                    b"name=\"model\"".as_slice(),
                    b"whisper-1",
                    b"name=\"language\"",
                    b"name=\"prompt\"",
                    b"name=\"response_format\"",
                ] {
                    assert!(
                        body.windows(field.len()).any(|part| part == field),
                        "required transcription field absent"
                    );
                }
                let wav = body.windows(4).position(|part| part == b"RIFF").unwrap();
                let hz = u32::from_le_bytes(body[wav + 24..wav + 28].try_into().unwrap());
                assert_eq!(hz, PROVIDER_HZ);
                Response::new(Body::from(r#"{"text":"from-loopback"}"#))
            }),
        );
        let server = Server::start(router).await;
        let text = server
            .endpoint()
            .await
            .transcribe(&segment())
            .await
            .unwrap();
        assert!(text.as_str() == "from-loopback", "transcript changed");
        server.finish().await;
    })
    .await;
}

#[tokio::test]
async fn redirects_never_send_the_credential_to_the_location() {
    within(async {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let redirected = Arc::new(AtomicUsize::new(0));
        let seen = Arc::clone(&redirected);
        let router = Router::new()
            .route(
                "/v1/audio/transcriptions",
                post(|| async {
                    Response::builder()
                        .status(StatusCode::FOUND)
                        .header("location", "/stolen")
                        .body(Body::empty())
                        .unwrap()
                }),
            )
            .route(
                "/stolen",
                axum::routing::any(move || {
                    let seen = Arc::clone(&seen);
                    async move {
                        let _previous = seen.fetch_add(1, Ordering::SeqCst);
                        Response::new(Body::from(r#"{"text":"unexpected"}"#))
                    }
                }),
            );
        let server = Server::start(router).await;
        let result = server.endpoint().await.transcribe(&segment()).await;
        assert!(matches!(
            result,
            Err(ProviderError::Decode(
                provider_json::DecodeError::StatusBody {
                    status: StatusCode::FOUND,
                    ..
                }
            ))
        ));
        assert_eq!(redirected.load(Ordering::SeqCst), 0);
        server.finish().await;
    })
    .await;
}

#[tokio::test]
async fn a_chunked_response_is_capped_without_trusting_content_length() {
    within(async {
        let router = Router::new().route(
            "/v1/audio/transcriptions",
            post(|| async {
                let chunks = [
                    Ok::<_, std::io::Error>(Bytes::from(vec![b'x'; MAX_RESPONSE_BYTES])),
                    Ok(Bytes::from_static(b"x")),
                ];
                Response::new(Body::from_stream(futures_util::stream::iter(chunks)))
            }),
        );
        let server = Server::start(router).await;
        let result = server.endpoint().await.transcribe(&segment()).await;
        assert!(matches!(result, Err(ProviderError::ResponseTooLarge)));
        server.finish().await;
    })
    .await;
}

#[test]
fn invalid_or_empty_credentials_are_refused_before_roots_or_network() {
    for (credential, absent) in [("  ", true), ("synthetic\ncredential", false)] {
        let error = Specification::new(
            &EndpointBase::openai(),
            EndpointModel::whisper(),
            Language::english(),
            &ResolvedValue::new(credential.to_owned()),
        )
        .err()
        .unwrap();
        if absent {
            assert!(matches!(error, ProviderError::KeyAbsent));
        } else {
            assert!(matches!(error, ProviderError::Header(_)));
        }
        assert!(!error.to_string().contains(credential));
        assert!(!format!("{error:?}").contains(credential));
    }
}

#[tokio::test]
async fn the_request_budget_also_covers_waiting_for_a_response_body() {
    use std::sync::atomic::{AtomicU32, Ordering};
    within(async {
        let calls = Arc::new(AtomicU32::new(0));
        let counted = Arc::clone(&calls);
        let entered = Arc::new(tokio::sync::Notify::new());
        let release = Arc::new(tokio::sync::Notify::new());
        let arrived = Arc::clone(&entered);
        let releasing = Arc::clone(&release);
        let router = Router::new().route(
            "/v1/audio/transcriptions",
            post(move || {
                let arrived = Arc::clone(&arrived);
                let counted = Arc::clone(&counted);
                let releasing = Arc::clone(&releasing);
                async move {
                    let _previous = counted.fetch_add(1, Ordering::SeqCst);
                    let chunks = futures_util::stream::once(async move {
                        arrived.notify_one();
                        releasing.notified().await;
                        Ok::<_, std::io::Error>(Bytes::from_static(br#"{"text":"late"}"#))
                    });
                    Response::new(Body::from_stream(chunks))
                }
            }),
        );
        let server = Server::start(router).await;
        let headers_received = Arc::new(tokio::sync::Notify::new());
        let mut endpoint = server.endpoint().await;
        endpoint.headers_received = Some(Arc::clone(&headers_received));
        let audio = segment();
        let work = tokio::spawn(async move { endpoint.transcribe(&audio).await });
        entered.notified().await;
        headers_received.notified().await;
        tokio::time::pause();
        tokio::time::advance(REQUEST).await;
        let error = tokio::time::timeout(REQUEST, work)
            .await
            .unwrap()
            .unwrap()
            .unwrap_err();
        tokio::time::resume();
        assert!(matches!(error, ProviderError::Http(source) if source.is_timeout()));
        assert_eq!(calls.load(Ordering::SeqCst), MAX_UPLOAD_ATTEMPTS);
        release.notify_one();
        server.finish().await;
    })
    .await;
}

#[tokio::test]
async fn transport_diagnostics_drop_the_url_without_losing_the_typed_cause() {
    within(async {
    let server = Server::start(Router::new()).await;
    let endpoint = server.endpoint().await;
    // Unsupported schemes are rejected by reqwest before any connection work.
    let url = Url::parse(&format!("file:///{KEY}")).unwrap();
    let request = reqwest::Request::new(reqwest::Method::GET, url);
    let source = endpoint.http.execute(request).await.unwrap_err();
    assert!(source.url().is_some());
    let guarded = ProviderError::Http(source);
    assert!(!format!("{guarded:?}").contains(KEY));
    let ProviderError::Http(source) = guarded else {
        panic!("the guarded transport cause changed category");
    };
    let error = ProviderError::transport(source);
    assert!(!error.to_string().contains(KEY));
    assert!(!format!("{error:?}").contains(KEY));
    assert!(std::error::Error::source(&error).is_some());
    assert!(
        matches!(error, ProviderError::Http(source) if source.is_builder() && source.url().is_none())
    );
    server.finish().await;
    })
    .await;
}

#[tokio::test]
async fn transient_statuses_and_retry_hints_never_repeat_an_upload() {
    use std::sync::atomic::{AtomicU32, Ordering};
    within(async {
        for status in [StatusCode::SERVICE_UNAVAILABLE, StatusCode::TOO_MANY_REQUESTS] {
            let calls = Arc::new(AtomicU32::new(0));
            let observed = Arc::clone(&calls);
            let router = Router::new().route("/v1/audio/transcriptions", post(move || {
                let observed = Arc::clone(&observed);
                async move {
                    let attempt = observed.fetch_add(1, Ordering::SeqCst);
                    if attempt < MAX_UPLOAD_ATTEMPTS {
                        Response::builder().status(status)
                            .header("x-should-retry", "true")
                            .header("retry-after", "0")
                            .header("retry-after-ms", "0")
                            .body(Body::from(r#"{"error":{"message":"synthetic provider failure payload"}}"#))
                            .unwrap()
                    } else {
                        Response::new(Body::from(r#"{"text":"unexpected retry"}"#))
                    }
                }
            }));
            let server = Server::start(router).await;
            let error = server.endpoint().await.transcribe(&segment()).await.unwrap_err();
            assert!(matches!(error, ProviderError::Decode(provider_json::DecodeError::Status(code)) if code == status));
            assert!(!error.to_string().contains("synthetic provider failure payload"));
            assert!(!format!("{error:?}").contains("synthetic provider failure payload"));
            assert_eq!(calls.load(Ordering::SeqCst), MAX_UPLOAD_ATTEMPTS);
            server.finish().await;
        }
    }).await;
}

async fn consume_request(socket: &mut tokio::net::TcpStream) -> std::io::Result<()> {
    use tokio::io::{AsyncBufReadExt as _, AsyncReadExt as _, BufReader};
    const MAX_FAKE_HEADER_BYTES: usize = 16 * 1024;
    const MAX_FAKE_BODY_BYTES: usize = 64 * 1024;
    let mut input = BufReader::new(socket);
    let mut size = 0_usize;
    let mut length = None;
    loop {
        let mut line = String::new();
        let count = input.read_line(&mut line).await?;
        assert!(count > 0, "request headers ended early");
        size = size.checked_add(count).unwrap();
        assert!(
            size <= MAX_FAKE_HEADER_BYTES,
            "test request headers exceeded their bound"
        );
        if line == "\r\n" {
            break;
        }
        if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
            length = Some(value.trim().parse::<usize>().unwrap());
        }
    }
    let length = length.unwrap();
    assert!(
        length <= MAX_FAKE_BODY_BYTES,
        "test request exceeded its fixture bound"
    );
    let mut body = vec![0; length];
    let _read = input.read_exact(&mut body).await?;
    Ok(())
}

#[tokio::test]
async fn a_server_closing_after_upload_before_headers_does_not_cause_a_resend() {
    use std::sync::atomic::{AtomicU32, Ordering};
    within(async {
        let calls = Arc::new(AtomicU32::new(0));
        let observed = Arc::clone(&calls);
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}/v1", listener.local_addr().unwrap())
            .parse()
            .unwrap();
        let stop = CancellationToken::new();
        let closing = stop.clone();
        let task = tokio::spawn(async move {
            loop {
                tokio::select! {
                    () = closing.cancelled() => break,
                    accepted = listener.accept() => {
                        let (mut socket, _peer) = accepted?;
                        consume_request(&mut socket).await?;
                        let _previous = observed.fetch_add(1, Ordering::SeqCst);
                        // The body was accepted; closing without response headers
                        // models an ambiguous upload, not a pre-send validation error.
                        drop(socket);
                    }
                }
            }
            Ok(())
        });
        let server = Server {
            base,
            stop,
            task: Some(task),
        };
        let error = server
            .endpoint()
            .await
            .transcribe(&segment())
            .await
            .unwrap_err();
        assert!(matches!(error, ProviderError::Http(_)));
        server.finish().await;
        assert_eq!(calls.load(Ordering::SeqCst), MAX_UPLOAD_ATTEMPTS);
    })
    .await;
}

#[tokio::test]
async fn an_advertised_oversize_is_refused_without_waiting_for_body_bytes() {
    within(async {
        let router = Router::new().route(
            "/v1/audio/transcriptions",
            post(|| async {
                let body = futures_util::stream::pending::<Result<Bytes, std::io::Error>>();
                Response::builder()
                    .header(
                        "content-length",
                        MAX_RESPONSE_BYTES.saturating_add(1).to_string(),
                    )
                    .body(Body::from_stream(body))
                    .unwrap()
            }),
        );
        let server = Server::start(router).await;
        let result = server.endpoint().await.transcribe(&segment()).await;
        assert!(matches!(result, Err(ProviderError::ResponseTooLarge)));
        // This fake deliberately has a body that never completes.
        drop(server);
    })
    .await;
}
