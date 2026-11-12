//! A browser tab, as the page's bridge client drives it (ADR 0017): the
//! websocket to `vessel ui`, the mutual proof, then calls and follows.
//!
//! The socket is opened to the literal loopback address with the launch's
//! own `Host` and `Origin`, so the test does not depend on the system
//! resolving `*.localhost`.

use std::fmt::Write as _;

use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt as _, StreamExt as _};
use hmac::{Hmac, KeyInit as _, Mac as _};
use tokio::net::TcpStream;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest as _;
use vessel_proto::{Call, EventSeq, RawJson, Reply, SessionId};
use vessel_web::wire::{
    AuthMode, CallId, CallOutcome, ClientFrame, NodeKey, Nonce, PresentedToken, ServerFrame,
    StreamId,
};

use crate::fleet::Launch;

type Socket = WebSocketStream<TcpStream>;

/// The tab's sending half; frames come in on [`Incoming`].
pub(crate) struct Tab {
    sink: SplitSink<Socket, Message>,
    next_id: u64,
}

/// The tab's receiving half.
pub(crate) struct Incoming {
    stream: SplitStream<Socket>,
}

impl Tab {
    /// Open a tab on `launch` and redeem its bootstrap.
    pub(crate) async fn open(launch: &Launch) -> (Self, Incoming) {
        let tcp = TcpStream::connect(("127.0.0.1", launch.port))
            .await
            .unwrap();
        let mut request = format!("ws://{}/api/ws", launch.host)
            .into_client_request()
            .unwrap();
        let origin = format!("http://{}", launch.host);
        let _none = request
            .headers_mut()
            .insert("origin", origin.parse().unwrap());
        let (socket, _response) = tokio_tungstenite::client_async(request, tcp).await.unwrap();
        let (sink, stream) = socket.split();
        let mut tab = Self { sink, next_id: 0 };
        let mut incoming = Incoming { stream };
        let ServerFrame::Challenge { launch: id, nonce } = incoming.next().await else {
            panic!("expected a challenge");
        };
        let client = Nonce::try_from("b".repeat(64)).unwrap();
        let transcript = format!(
            "vessel-ui-v1\nclient\n{}\n{}\n{}\n",
            id.as_str(),
            nonce.as_str(),
            client.as_str()
        );
        tab.send(&ClientFrame::Hello {
            mode: AuthMode::Bootstrap,
            nonce: client,
            proof: PresentedToken::new(sign(&launch.bootstrap, &transcript)),
        })
        .await;
        let ServerFrame::Welcome { .. } = incoming.next().await else {
            panic!("expected a welcome");
        };
        (tab, incoming)
    }

    async fn send(&mut self, frame: &ClientFrame) {
        let text = serde_json::to_string(frame).unwrap();
        self.sink.send(Message::Text(text.into())).await.unwrap();
    }

    /// Send `call` to `node`; its id.
    pub(crate) async fn call(&mut self, node: &NodeKey, call: &Call) -> CallId {
        self.next_id = self.next_id.checked_add(1).unwrap();
        let id = CallId::new(self.next_id);
        let call = RawJson::from_serialize(call).unwrap();
        self.send(&ClientFrame::Call {
            id,
            node: node.clone(),
            call,
        })
        .await;
        id
    }

    /// Follow `session` on `node` from its first record as `stream`.
    pub(crate) async fn follow(&mut self, stream: StreamId, node: &NodeKey, session: SessionId) {
        self.send(&ClientFrame::Follow {
            stream,
            node: node.clone(),
            session,
            from: EventSeq::ZERO,
        })
        .await;
    }
}

impl Incoming {
    /// The next frame from the bridge.
    pub(crate) async fn next(&mut self) -> ServerFrame {
        loop {
            match self.stream.next().await {
                Some(Ok(Message::Text(text))) => return serde_json::from_str(&text).unwrap(),
                Some(Ok(Message::Ping(_) | Message::Pong(_))) => {}
                other => panic!("the bridge closed: {other:?}"),
            }
        }
    }

    /// Frames until the reply to `id`; the reply, or why there is none.
    /// Other frames are dropped (nothing is followed while a setup call
    /// waits).
    pub(crate) async fn reply(&mut self, id: CallId) -> Result<Reply, String> {
        loop {
            if let ServerFrame::Reply {
                id: answered,
                outcome,
            } = self.next().await
                && answered == id
            {
                return match outcome {
                    CallOutcome::Ok(reply) => Ok(*reply),
                    CallOutcome::Err(failure) => Err(format!("{failure:?}")),
                };
            }
        }
    }
}

/// HMAC-SHA256 of `message` under `key`'s ASCII hex, as lowercase hex.
fn sign(key: &str, message: &str) -> String {
    let mut mac = Hmac::<sha2::Sha256>::new_from_slice(key.as_bytes()).unwrap();
    mac.update(message.as_bytes());
    mac.finalize()
        .into_bytes()
        .iter()
        .fold(String::new(), |mut text, byte| {
            write!(text, "{byte:02x}").unwrap();
            text
        })
}
