//! Synthetic HTTP content exercises the real controller and local policy refusals.
//!
//! This helper owns no workspace/Preview/root authority and permits only its
//! fixed fixture origin. A successful PNG is transport/geometry evidence;
//! native decoding, paint, first-frame policy and engine qualification stay separate.
use super::candidate_policy::PolicyOrigin;
use super::{
    Client, Events, SHUTDOWN_TIMEOUT,
    candidate_network::{ProbeError, Routes},
};
use crate::page::{Page, PageError, ViewportFrame};
use crate::targets::{TargetError, Targets};
use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Request, StatusCode, header},
    response::{IntoResponse as _, Response},
};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::net::TcpListener;
use tokio_util::sync::{CancellationToken, DropGuard};
use vessel_proto::WorkspaceId;
use vessel_proto::browser::{
    CssCoordinate, CssPoint, HumanInput, ImageSize, InputText, KeyModifiers, MouseButton,
    PointerAction, RequiredInput, ScreenshotExtent, TextCompleteness, ViewportSide, ViewportSize,
    WaitBudget, WaitFor, WaitMatch,
};
use vessel_proto::conversation::FiniteNumber;

const HTML: &str = r#"<!doctype html><meta charset=utf-8><title>Rust fixture</title>
<style>html {scrollbar-width:none} body {display:flow-root; margin:0; height:1200px}
#probe {position:absolute; left:20px; top:100px; width:120px; height:24px}</style>
<h1>Owned Rust page</h1><label for=probe>Fixture input</label>
<input id=probe onpointerdown="this.dataset.trusted = event.isTrusted">
<button>Fixture button</button><p>Owned Rust page ready</p>"#;
const WIDTH: u32 = 390;
const HEIGHT: u32 = 844;
const FULL_HEIGHT: u32 = 1200;
const INPUT_X: f64 = 32.0;
const INPUT_Y: f64 = 110.0;
const INPUT_TEXT: &str = "typed in the fixture";
const FIXTURE_COOKIE: &str = "vessel-fixture-storage=present";
pub(super) const PERSISTED_ROUTE: &str = "http://fixture.test/profile-marker";
pub(super) const PERSISTED_MARKER: &str = "Fixture profile persisted";
const DOWNLOAD_ROUTE: &str = "/fixture-download";
pub(super) const DOWNLOAD_NAME: &str = "vessel-browser-fixture-download.txt";

#[derive(Clone)]
struct FixtureState {
    routes: Routes,
    sent_attachment: Arc<AtomicBool>,
    policy: Arc<PolicyOrigin>,
}

pub(super) struct FixtureProxy {
    stop: CancellationToken,
    guard: DropGuard,
    actor: tokio::task::JoinHandle<Result<(), std::io::Error>>,
    sent_attachment: Arc<AtomicBool>,
}
impl FixtureProxy {
    pub(super) fn start(listener: TcpListener, routes: Routes, policy: Arc<PolicyOrigin>) -> Self {
        let stop = CancellationToken::new();
        let guard = stop.clone().drop_guard();
        let closing = stop.clone();
        let sent_attachment = Arc::new(AtomicBool::new(false));
        let state = FixtureState {
            routes,
            sent_attachment: Arc::clone(&sent_attachment),
            policy,
        };
        let actor = tokio::spawn(async move {
            axum::serve(listener, Router::new().fallback(document).with_state(state))
                .with_graceful_shutdown(closing.cancelled_owned())
                .await
        });
        Self {
            stop,
            guard,
            actor,
            sent_attachment,
        }
    }
    pub(super) async fn finish(self) {
        let Self {
            stop,
            guard,
            actor,
            sent_attachment,
        } = self;
        stop.cancel();
        tokio::time::timeout(SHUTDOWN_TIMEOUT, actor)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert!(
            sent_attachment.load(Ordering::SeqCst),
            "the browser never requested the attachment"
        );
        drop(guard);
    }
}
async fn document(State(state): State<FixtureState>, request: Request<Body>) -> Response {
    let (parts, body) = request.into_parts();
    drop(body);
    if parts.method != axum::http::Method::GET {
        return StatusCode::FORBIDDEN.into_response();
    }
    if let Some(refused) = state.routes.try_refusal(&parts.uri).await {
        return match refused {
            Ok(marker) => (
                [
                    (header::CACHE_CONTROL, "no-store"),
                    (header::ACCESS_CONTROL_ALLOW_ORIGIN, state.policy.origin()),
                ],
                marker,
            )
                .into_response(),
            Err(error) => error.into_response(),
        };
    }
    if let Some(response) = state.policy.serve(&parts.uri, state.routes) {
        return response;
    }
    if parts.uri.host() != Some("fixture.test") {
        return StatusCode::FORBIDDEN.into_response();
    }
    if let Some(target) = state.routes.redirect(parts.uri.path()) {
        return (StatusCode::FOUND, [(header::LOCATION, target)]).into_response();
    }
    if parts.uri.path() == DOWNLOAD_ROUTE {
        state.sent_attachment.store(true, Ordering::SeqCst);
        let attachment = format!("attachment; filename={DOWNLOAD_NAME}");
        return (
            [
                (header::CONTENT_TYPE, "application/octet-stream"),
                (header::CACHE_CONTROL, "no-store"),
                (header::CONTENT_DISPOSITION, attachment.as_str()),
            ],
            "fixture attachment",
        )
            .into_response();
    }
    if parts.uri.path() == "/favicon.ico" {
        return StatusCode::NO_CONTENT.into_response();
    }
    if parts.uri.path() == "/profile-marker" {
        let received = parts
            .headers
            .get(header::COOKIE)
            .and_then(|entry| entry.to_str().ok());
        return if received.is_some_and(|cookies| {
            cookies
                .split(';')
                .any(|cookie| cookie.trim() == FIXTURE_COOKIE)
        }) {
            ([(header::CACHE_CONTROL, "no-store")], PERSISTED_MARKER).into_response()
        } else {
            StatusCode::PRECONDITION_FAILED.into_response()
        };
    }
    let storage_cookie = format!("{FIXTURE_COOKIE}; Max-Age=600; Path=/; SameSite=Lax");
    (
        [
            (header::CONTENT_TYPE, "text/html; charset=utf-8"),
            (header::CACHE_CONTROL, "no-store"),
            (header::SET_COOKIE, storage_cookie.as_str()),
        ],
        HTML,
    )
        .into_response()
}

pub(super) struct Observed {
    matched: WaitMatch,
    snapshot: crate::snapshot::Snapshot,
    png: ImageSize,
    full_png: ImageSize,
    compositor_png: ImageSize,
}

#[derive(Debug, thiserror::Error)]
pub(super) enum ExerciseError {
    #[error(transparent)]
    Target(#[from] TargetError),
    #[error(transparent)]
    Page(#[from] PageError),
    #[error(transparent)]
    Policy(#[from] ProbeError),
    #[error("the fixture's download navigation unexpectedly completed")]
    DownloadAccepted,
}

pub(super) async fn exercise(
    client: &Client,
    events: &mut Events,
    routes: Routes,
    policy: &PolicyOrigin,
) -> Result<Observed, ExerciseError> {
    let viewport = ViewportSize {
        width: ViewportSide::try_from(WIDTH).unwrap(),
        height: ViewportSide::try_from(HEIGHT).unwrap(),
    };
    let workspace = WorkspaceId::random();
    let mut targets = Targets::default();
    let browser = targets
        .open(
            client,
            workspace,
            "http://fixture.test/".parse().unwrap(),
            viewport,
        )
        .await?;
    assert!(matches!(
        targets.owned(WorkspaceId::random(), &browser),
        Err(TargetError::Missing)
    ));
    let page = targets.owned(workspace, &browser)?;
    // The final marker follows the controls, so a partially parsed document is
    // not mistaken for a ready snapshot/capture fixture. This is not paint proof.
    let matched = page
        .wait(
            WaitFor::Text(RequiredInput::try_from("Owned Rust page ready".to_owned()).unwrap()),
            WaitBudget::Default,
        )
        .await?;
    let snapshot = page.snapshot().await?;
    let mut frames = page.subscribe_frames();
    page.refresh_visible_frame().await?;
    let frame = frames.borrow_and_update().clone().unwrap();
    let png = frame.image().size();
    exercise_input(page, &frame).await?;
    assert!(
        frames.borrow().is_none(),
        "old document generation stayed visible"
    );
    let changed = page.viewport_frame().await?;
    assert_ne!(changed.image(), frame.image());
    assert!(matches!(
        page.input_from_frame(&frame, HumanInput::ReleaseAll).await,
        Err(PageError::StaleFrame)
    ));
    let full_png = page.capture_extent(ScreenshotExtent::Page).await?.size();
    let compositor_png = page.probe_compositor(events).await?.size();
    observe_policy(page, events, routes, policy).await?;
    targets.close(workspace, &browser).await?;
    assert!(matches!(
        targets.owned(workspace, &browser),
        Err(TargetError::Missing)
    ));
    Ok(Observed {
        matched,
        snapshot,
        png,
        full_png,
        compositor_png,
    })
}

async fn observe_policy(
    page: &mut Page,
    events: &mut Events,
    routes: Routes,
    policy: &PolicyOrigin,
) -> Result<(), ExerciseError> {
    // Keep the diagnostic's raw event queue serviced just as the actor does
    // between commands. No event from a prior route satisfies a page marker.
    let probe = async {
        routes.verify_browser(page).await?;
        policy.verify(page).await?;
        let attachment = format!("http://fixture.test{DOWNLOAD_ROUTE}")
            .parse()
            .unwrap();
        match page.navigate(attachment).await {
            Err(PageError::Navigation) => Ok(()),
            Err(error) => Err(error.into()),
            Ok(()) => Err(ExerciseError::DownloadAccepted),
        }
    };
    tokio::pin!(probe);
    loop {
        tokio::select! {
            result = &mut probe => return result,
            event = events.next() => {
                if event.is_none() {
                    return Err(PageError::Cdp(super::CdpError::Closed).into());
                }
            }
        }
    }
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
        let dimensions = [self.png, self.full_png, self.compositor_png]
            .map(|image| (image.width().get(), image.height().get()));
        assert_eq!(
            dimensions,
            [(WIDTH, HEIGHT), (WIDTH, FULL_HEIGHT), (WIDTH, HEIGHT)],
            "viewport, full-page, and compositor PNG dimensions"
        );
    }
}
