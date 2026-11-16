//! Synthetic HTTP content exercises the real controller, never the product proxy.
//!
//! This helper owns no workspace/Preview/root authority and permits only its
//! fixed fixture origin. A successful PNG is transport/geometry evidence;
//! native decoding, paint, first-frame policy and engine qualification stay separate.
use super::{Client, SHUTDOWN_TIMEOUT};
use crate::page::{Page, PageError};
use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header},
    response::{IntoResponse as _, Response},
};
use tokio::net::TcpListener;
use tokio_util::sync::{CancellationToken, DropGuard};
use vessel_proto::browser::{
    ImageSize, RequiredInput, ViewportSide, ViewportSize, WaitBudget, WaitFor, WaitMatch,
};

const HTML: &str = "<!doctype html><meta charset=utf-8><title>Rust fixture</title><h1>Owned Rust page</h1><label for=probe>Fixture input</label><input id=probe><button>Fixture button</button><p>Owned Rust page ready</p>";
const WIDTH: u32 = 390;
const HEIGHT: u32 = 844;

pub(super) struct FixtureProxy {
    stop: CancellationToken,
    guard: DropGuard,
    actor: tokio::task::JoinHandle<Result<(), std::io::Error>>,
}
impl FixtureProxy {
    pub(super) fn start(listener: TcpListener) -> Self {
        let stop = CancellationToken::new();
        let guard = stop.clone().drop_guard();
        let closing = stop.clone();
        let actor = tokio::spawn(async move {
            axum::serve(listener, Router::new().fallback(document))
                .with_graceful_shutdown(closing.cancelled_owned())
                .await
        });
        Self { stop, guard, actor }
    }
    pub(super) async fn finish(self) {
        let Self { stop, guard, actor } = self;
        stop.cancel();
        tokio::time::timeout(SHUTDOWN_TIMEOUT, actor)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        drop(guard);
    }
}
fn document(request: Request<Body>) -> std::future::Ready<Response> {
    if request.method() != axum::http::Method::GET || request.uri().host() != Some("fixture.test") {
        return std::future::ready(StatusCode::FORBIDDEN.into_response());
    }
    if request.uri().path() == "/favicon.ico" {
        return std::future::ready(StatusCode::NO_CONTENT.into_response());
    }
    std::future::ready(
        (
            [
                (header::CONTENT_TYPE, "text/html; charset=utf-8"),
                (header::CACHE_CONTROL, "no-store"),
            ],
            HTML,
        )
            .into_response(),
    )
}

pub(super) struct Observed {
    matched: WaitMatch,
    snapshot: crate::snapshot::Snapshot,
    png: ImageSize,
}
pub(super) async fn exercise(client: &Client) -> Result<Observed, PageError> {
    let viewport = ViewportSize {
        width: ViewportSide::try_from(WIDTH).unwrap(),
        height: ViewportSide::try_from(HEIGHT).unwrap(),
    };
    let mut page = Page::open(
        client.clone(),
        "http://fixture.test/".parse().unwrap(),
        viewport,
    )
    .await?;
    // The final marker follows the controls, so a partially parsed document is
    // not mistaken for a ready snapshot/capture fixture. This is not paint proof.
    let matched = page
        .wait(
            WaitFor::Text(RequiredInput::try_from("Owned Rust page ready".to_owned()).unwrap()),
            WaitBudget::Default,
        )
        .await?;
    let snapshot = page.snapshot().await?;
    let png = page.capture().await?.size();
    page.close().await?;
    Ok(Observed {
        matched,
        snapshot,
        png,
    })
}
impl Observed {
    pub(super) fn verify(&self) {
        assert_eq!(self.matched, WaitMatch::Text);
        assert_eq!(
            self.snapshot.document.address.as_str(),
            "http://fixture.test/"
        );
        assert_eq!(self.snapshot.document.title.as_str(), "Rust fixture");
        assert!(
            self.snapshot
                .content
                .text
                .as_str()
                .contains("Fixture input")
        );
        assert!(
            self.snapshot
                .content
                .text
                .as_str()
                .contains("Fixture button")
        );
        assert_eq!(self.png.width().get(), WIDTH);
        assert_eq!(self.png.height().get(), HEIGHT);
    }
}
