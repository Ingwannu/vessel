//! Synthetic HTTP content exercises the real controller, never the product proxy.
//!
//! This helper owns no workspace/Preview/root authority and permits only its
//! fixed fixture origin. A successful PNG is transport/geometry evidence;
//! native decoding, paint, first-frame policy and engine qualification stay separate.
use super::{Client, SHUTDOWN_TIMEOUT};
use crate::page::{Page, PageError, ViewportFrame};
use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header},
    response::{IntoResponse as _, Response},
};
use tokio::net::TcpListener;
use tokio_util::sync::{CancellationToken, DropGuard};
use vessel_proto::browser::{
    CssCoordinate, CssPoint, HumanInput, ImageSize, InputText, KeyModifiers, MouseButton,
    PointerAction, RequiredInput, ScreenshotExtent, TextCompleteness, ViewportSide, ViewportSize,
    WaitBudget, WaitFor, WaitMatch,
};
use vessel_proto::conversation::FiniteNumber;

const HTML: &str = r#"<!doctype html><meta charset=utf-8><title>Rust fixture</title>
<style>body {margin:0; min-height:1200px} #probe {position:absolute; left:20px; top:100px; width:120px; height:24px}</style>
<h1>Owned Rust page</h1><label for=probe>Fixture input</label>
<input id=probe onpointerdown="this.dataset.trusted = event.isTrusted">
<button>Fixture button</button><p>Owned Rust page ready</p>"#;
const WIDTH: u32 = 390;
const HEIGHT: u32 = 844;
const FULL_HEIGHT: u32 = 1200;
const INPUT_X: f64 = 32.0;
const INPUT_Y: f64 = 110.0;
const INPUT_TEXT: &str = "typed in the fixture";

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
    let (parts, body) = request.into_parts();
    drop(body);
    if parts.method != axum::http::Method::GET || parts.uri.host() != Some("fixture.test") {
        return std::future::ready(StatusCode::FORBIDDEN.into_response());
    }
    if parts.uri.path() == "/favicon.ico" {
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
    full_png: ImageSize,
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
    let frame = page.viewport_frame().await?;
    let png = frame.image().size();
    exercise_input(&mut page, &frame).await?;
    let changed = page.viewport_frame().await?;
    assert_ne!(changed.image(), frame.image());
    assert!(matches!(
        page.input_from_frame(&frame, HumanInput::ReleaseAll).await,
        Err(PageError::StaleFrame)
    ));
    let full_png = page.capture_extent(ScreenshotExtent::Page).await?.size();
    page.close().await?;
    Ok(Observed {
        matched,
        snapshot,
        png,
        full_png,
    })
}

async fn exercise_input(page: &mut Page, frame: &ViewportFrame) -> Result<(), PageError> {
    let point = CssPoint {
        x: CssCoordinate::try_from(FiniteNumber::new(INPUT_X).unwrap()).unwrap(),
        y: CssCoordinate::try_from(FiniteNumber::new(INPUT_Y).unwrap()).unwrap(),
    };
    for action in [
        PointerAction::Press(MouseButton::Left),
        PointerAction::Release(MouseButton::Left),
    ] {
        page.input_from_frame(
            frame,
            HumanInput::Pointer {
                position: point,
                action,
                modifiers: KeyModifiers::default(),
            },
        )
        .await?;
    }
    page.input_from_frame(
        frame,
        HumanInput::Text {
            text: InputText::try_from(INPUT_TEXT.to_owned()).unwrap(),
        },
    )
    .await?;
    let inspect = RequiredInput::try_from(
        "() => { const input = document.querySelector('#probe'); return [input.value, input.dataset.trusted, input === document.activeElement]; }"
            .to_owned(),
    ).unwrap();
    let observed = page.evaluate_function(&inspect).await?;
    let expected = serde_json::to_string(&(INPUT_TEXT, "true", true)).unwrap();
    assert_eq!(observed.completeness, TextCompleteness::Whole);
    assert_eq!(observed.text.as_str(), expected);
    Ok(())
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
        assert_eq!(self.full_png.width().get(), WIDTH);
        assert_eq!(self.full_png.height().get(), FULL_HEIGHT);
    }
}
