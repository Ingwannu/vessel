//! Bypass SDK admission to prove the real server refuses before dispatch.
//! Only the fixture's temporary node and token are used by this connection.

use crate::support::{Fixture, client};
use futures_util::{SinkExt as _, StreamExt as _};
use tokio_tungstenite::tungstenite::{Message, client::IntoClientRequest as _};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use vessel_node::Node;
use vessel_proto::{
    Call, ClientFrame, Hello, Outcome, ProtocolRange, RequestId, ServerFrame, WireError,
};

pub(super) struct Raw(WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>);

impl Raw {
    pub(super) async fn connect(fixture: &Fixture, node: &Node) -> Self {
        let endpoint = format!("ws://{}/rpc", node.address().unwrap());
        let mut upgrade = endpoint.into_client_request().unwrap();
        let mut bearer = format!("Bearer {}", fixture.token().expose())
            .parse::<tokio_tungstenite::tungstenite::http::HeaderValue>()
            .unwrap();
        bearer.set_sensitive(true);
        let _previous = upgrade.headers_mut().insert("authorization", bearer);
        let (mut socket, _) = tokio_tungstenite::connect_async(upgrade).await.unwrap();
        let hello = ClientFrame::Hello(Hello {
            protocol: ProtocolRange::CURRENT,
            client: client(),
        });
        socket
            .send(Message::text(serde_json::to_string(&hello).unwrap()))
            .await
            .unwrap();
        let frame = socket.next().await.unwrap().unwrap().into_text().unwrap();
        let ServerFrame::Welcome(welcome) = serde_json::from_str(&frame).unwrap() else {
            panic!("expected a welcome");
        };
        assert_eq!(welcome.protocol, vessel_proto::PROTOCOL_VERSION);
        Self(socket)
    }

    pub(super) async fn refusal(&mut self, call: Call) -> WireError {
        let frame = ClientFrame::Request {
            id: RequestId::new(1),
            call,
        };
        self.0
            .send(Message::text(serde_json::to_string(&frame).unwrap()))
            .await
            .unwrap();
        let frame = self.0.next().await.unwrap().unwrap().into_text().unwrap();
        let ServerFrame::Response {
            id,
            outcome: Outcome::Err(error),
        } = serde_json::from_str(&frame).unwrap()
        else {
            panic!("expected a refusal");
        };
        assert_eq!(id, RequestId::new(1));
        error
    }

    pub(super) async fn close(mut self) {
        self.0.close(None).await.unwrap();
    }
}
