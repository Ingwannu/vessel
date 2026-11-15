//! Scripted loopback responses with channel barriers; provider and node
//! share the same real credential cache. No product API override is added.
use super::super::*;
use axum::{
    Router,
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use std::{
    collections::VecDeque,
    sync::atomic::{AtomicUsize, Ordering},
    sync::{Mutex, PoisonError},
};
use tokio::sync::oneshot;
use vessel_forge::{Credentials, EnvTokens, GitHub, Token};

pub(super) const EMPTY: &str = r#"{"data":{"repository":{"pullRequests":{"nodes":[],"pageInfo":{"hasNextPage":false,"endCursor":null}}}}}"#;

pub(super) struct Reply {
    status: StatusCode,
    body: &'static str,
    seen: Option<oneshot::Sender<()>>,
    release: Option<oneshot::Receiver<()>>,
}
impl Reply {
    pub(super) fn immediate(status: StatusCode, body: &'static str) -> Self {
        Self {
            status,
            body,
            seen: None,
            release: None,
        }
    }
    pub(super) fn held() -> (Self, oneshot::Receiver<()>, oneshot::Sender<()>) {
        let (seen, started) = oneshot::channel();
        let (release, wait) = oneshot::channel();
        (
            Self {
                status: StatusCode::OK,
                body: EMPTY,
                seen: Some(seen),
                release: Some(wait),
            },
            started,
            release,
        )
    }
}
#[derive(Default)]
struct Script {
    replies: Mutex<VecDeque<Reply>>,
    calls: AtomicUsize,
}
pub(super) struct Stand {
    script: Arc<Script>,
    pub(super) credentials: Arc<Credentials>,
    pub(super) github: Arc<GitHub>,
    pub(super) hosts: Arc<Hosts>,
}
async fn graphql(State(script): State<Arc<Script>>) -> axum::response::Response {
    let reply = script
        .replies
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .pop_front();
    let Some(reply) = reply else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    let _previous = script.calls.fetch_add(1, Ordering::SeqCst);
    if let Some(seen) = reply.seen {
        seen.send(()).unwrap();
    }
    if let Some(wait) = reply.release {
        wait.await.unwrap();
    }
    (reply.status, reply.body).into_response()
}
impl Stand {
    pub(super) async fn start(fixture: &Fixture) -> Self {
        let script = Arc::new(Script::default());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let app = Router::new()
            .route("/graphql", post(graphql))
            .route("/user", get(|| async { r#"{"login":"fixture"}"# }))
            .with_state(Arc::clone(&script));
        let stop = fixture.stop.clone();
        drop(fixture.tasks.spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(stop.cancelled_owned())
                .await
                .unwrap();
        }));
        let credentials = Arc::new(Credentials::new(
            EnvTokens {
                github: Token::new("fixture-node-loopback"),
                enterprise: None,
            },
            None,
        ));
        let github = Arc::new(
            GitHub::at(
                Arc::clone(&credentials),
                format!("http://{address}/").parse().unwrap(),
            )
            .unwrap(),
        );
        let provider = Arc::clone(&github);
        let hosts = Arc::new(Hosts::for_provider_test(Arc::clone(&credentials), provider));
        Self {
            script,
            credentials,
            github,
            hosts,
        }
    }
    pub(super) fn queue(&self, reply: Reply) {
        self.script.replies.lock().unwrap().push_back(reply);
    }
    pub(super) fn calls(&self) -> usize {
        self.script.calls.load(Ordering::SeqCst)
    }
    pub(super) fn client(&self, fixture: &Fixture) -> Arc<ProjectPulls> {
        Arc::new(ProjectPulls::start(
            Arc::clone(&fixture.workspaces),
            Arc::clone(&self.hosts),
            &fixture.tasks,
            &fixture.stop,
        ))
    }
}
