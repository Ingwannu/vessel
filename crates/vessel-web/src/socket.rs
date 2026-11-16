//! Mutually authenticate before starting the bridge. Tungstenite owns the
//! socket so a fresh 4 MiB parser is installed only after proof;
//! axum's WebSocket wrapper cannot change its configuration after upgrade.
//! An elapsed hello budget is not proof of a bad credential: it remains
//! retryable, while invalid proof and unexpected frames remain refusals.
use std::sync::Arc;

use axum::Extension;
use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse as _, Response};
use futures_util::{SinkExt as _, StreamExt as _};
use hyper::upgrade::Upgraded;
use hyper_util::rt::TokioIo;
use serde::Deserialize;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::{
    Message,
    handshake::server::create_response,
    protocol::{CloseFrame, Role, WebSocketConfig, frame::coding::CloseCode},
};

use crate::server::Ui;
use crate::source::NodeSource;
use crate::transport::Connection;
use crate::wire::{AuthMode, CLOSE_UNAUTHORIZED, Nonce, PresentedToken, ServerFrame};

pub(crate) type WebSocket = WebSocketStream<TokioIo<Upgraded>>;
const PREAUTH_FRAME: usize = 4096;
const MAX_FRAME: usize = 4 * 1024 * 1024;

pub(crate) async fn upgrade<S: NodeSource>(
    State(ui): State<Arc<Ui<S>>>,
    Extension(place): Extension<Arc<Connection>>,
    request: Request,
) -> Response {
    let (parts, _body) = request.into_parts();
    let mut request = hyper::Request::from_parts(parts, ());
    let Ok(response) = create_response(&request) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let upgrade = hyper::upgrade::on(&mut request);
    drop(tokio::spawn(async move {
        tokio::select! {
            () = ui.shutdown.cancelled() => {},
            () = async {
                if let Ok(Ok(io)) = tokio::time::timeout(ui.hello_deadline, upgrade).await {
                    let config = WebSocketConfig::default().max_message_size(Some(PREAUTH_FRAME)).max_frame_size(Some(PREAUTH_FRAME));
                    let socket = WebSocketStream::from_raw_socket(TokioIo::new(io), Role::Server, Some(config)).await;
                    run(Arc::clone(&ui), socket, place).await;
                }
            } => {},
        }
    }));
    response.map(|()| axum::body::Body::empty())
}

// Decode only the tiny hello shape before authentication, never node calls.
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum Hello {
    Hello {
        mode: AuthMode,
        nonce: Nonce,
        proof: PresentedToken,
    },
}

enum AuthFailure {
    Refused,
    TimedOut,
}

impl AuthFailure {
    fn frame(self) -> CloseFrame {
        let (code, reason) = match self {
            Self::Refused => (
                CLOSE_UNAUTHORIZED.into(),
                "the UI was restarted or authentication failed",
            ),
            Self::TimedOut => (CloseCode::Again, "UI authentication timed out"),
        };
        CloseFrame {
            code,
            reason: reason.into(),
        }
    }
}

async fn run<S: NodeSource>(ui: Arc<Ui<S>>, mut socket: WebSocket, place: Arc<Connection>) {
    let accepted = tokio::time::timeout(ui.hello_deadline, authenticate(&ui, &mut socket)).await;
    let failure = match accepted {
        Ok(true) => None,
        Ok(false) => Some(AuthFailure::Refused),
        Err(_elapsed) => Some(AuthFailure::TimedOut),
    };
    if let Some(failure) = failure {
        // A bounded best-effort reason; the socket is dropped either way.
        let _closed = tokio::time::timeout(
            std::time::Duration::from_millis(100),
            socket.send(Message::Close(Some(failure.frame()))),
        )
        .await;
        return;
    }
    place.authenticated();
    // The protocol forbids calls before welcome. Discard any pre-auth
    // read-ahead rather than carrying attacker-pipelined calls across the
    // boundary. A conforming peer waits for welcome, so its calls are
    // still in the transport (send does not read). tokio-tungstenite does
    // not expose the synchronous parser's set_config.
    let config = WebSocketConfig::default()
        .max_message_size(Some(MAX_FRAME))
        .max_frame_size(Some(MAX_FRAME));
    let socket =
        WebSocketStream::from_raw_socket(socket.into_inner(), Role::Server, Some(config)).await;
    crate::bridge::run(Arc::clone(&ui.registry), ui.keeper.clone(), socket).await;
}

async fn authenticate<S: NodeSource>(ui: &Ui<S>, socket: &mut WebSocket) -> bool {
    let Ok(random) = crate::LaunchToken::mint() else {
        return false;
    };
    let nonce = Nonce::from_random(random);
    let challenge = ServerFrame::Challenge {
        launch: ui.auth.launch.clone(),
        nonce: nonce.clone(),
    };
    if !send(socket, &challenge).await {
        return false;
    }
    loop {
        match socket.next().await {
            Some(Ok(Message::Text(text))) => {
                let Ok(Hello::Hello {
                    mode,
                    nonce: client,
                    proof,
                }) = serde_json::from_str::<Hello>(&text)
                else {
                    return false;
                };
                let Some(welcome) = ui.auth.welcome(mode, &nonce, &client, &proof) else {
                    return false;
                };
                return send(socket, &welcome).await;
            }
            Some(Ok(Message::Ping(_) | Message::Pong(_))) => {}
            _ => return false,
        }
    }
}

async fn send(socket: &mut WebSocket, frame: &ServerFrame) -> bool {
    match serde_json::to_string(frame) {
        Ok(text) => socket.send(Message::Text(text.into())).await.is_ok(),
        Err(_) => false,
    }
}
