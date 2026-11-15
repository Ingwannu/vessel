//! Cancellation exercises the real client against held loopback responses.
use super::*;
use crate::voice::fixture::{SYNTHETIC_KEY, Server};
use axum::{
    Router,
    body::{Body, Bytes},
    http::Response,
    routing::post,
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use tokio::sync::Notify;

async fn client(server: &Server) -> Endpoint {
    Endpoint::connect(
        &server.base,
        Operation::Synthesize,
        &ResolvedValue::new(SYNTHETIC_KEY.to_owned()),
    )
    .await
    .unwrap()
}
fn kind() -> header::HeaderValue {
    header::HeaderValue::from_static("application/json")
}

#[tokio::test]
async fn cancelled_before_post_does_not_send_headers_or_pcm() {
    let count = Arc::new(AtomicUsize::new(0));
    let calls = Arc::clone(&count);
    let server = Server::start(Router::new().route(
        "/v1/audio/speech",
        post(move || {
            let calls = Arc::clone(&calls);
            async move {
                let _old = calls.fetch_add(1, Ordering::Relaxed);
                Body::empty()
            }
        }),
    ))
    .await;
    let endpoint = client(&server).await;
    let stop = CancellationToken::new();
    stop.cancel();
    assert!(matches!(
        endpoint.post(vec![0; 8], kind(), &stop).await,
        Err(EndpointError::Cancelled)
    ));
    assert_eq!(count.load(Ordering::Relaxed), 0);
    server.finish().await;
}

#[tokio::test]
async fn cancellation_wakes_a_request_with_held_response_headers() {
    let entered = Arc::new(Notify::new());
    let release = CancellationToken::new();
    let (observed, held) = (Arc::clone(&entered), release.clone());
    let server = Server::start(Router::new().route(
        "/v1/audio/speech",
        post(move || {
            let (observed, held) = (Arc::clone(&observed), held.clone());
            async move {
                observed.notify_one();
                held.cancelled().await;
                Body::empty()
            }
        }),
    ))
    .await;
    let endpoint = client(&server).await;
    let stop = CancellationToken::new();
    let cancellation = stop.clone();
    let task = tokio::spawn(async move { endpoint.post(vec![0; 8], kind(), &cancellation).await });
    tokio::time::timeout(Duration::from_secs(5), entered.notified())
        .await
        .unwrap();
    stop.cancel();
    assert!(matches!(
        tokio::time::timeout(Duration::from_secs(5), task)
            .await
            .unwrap()
            .unwrap(),
        Err(EndpointError::Cancelled)
    ));
    release.cancel();
    server.finish().await;
}

#[tokio::test]
async fn cancellation_wakes_a_bounded_reader_with_a_held_response_body() {
    let entered = Arc::new(Notify::new());
    let release = CancellationToken::new();
    let (observed, held) = (Arc::clone(&entered), release.clone());
    let server = Server::start(Router::new().route(
        "/v1/audio/speech",
        post(move || {
            let (observed, held) = (Arc::clone(&observed), held.clone());
            async move {
                Response::new(Body::from_stream(futures_util::stream::once(async move {
                    observed.notify_one();
                    held.cancelled().await;
                    Ok::<_, std::convert::Infallible>(Bytes::from_static(&[0, 0]))
                })))
            }
        }),
    ))
    .await;
    let endpoint = client(&server).await;
    let stop = CancellationToken::new();
    let response = endpoint.post(Vec::new(), kind(), &stop).await.unwrap();
    let cancellation = stop.clone();
    let task = tokio::spawn(async move { read(response, 128, &cancellation).await });
    tokio::time::timeout(Duration::from_secs(5), entered.notified())
        .await
        .unwrap();
    stop.cancel();
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(5), task)
            .await
            .unwrap()
            .unwrap()
            .unwrap_err(),
        EndpointError::Cancelled
    );
    release.cancel();
    server.finish().await;
}
