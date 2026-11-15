//! Route-less invitations refuse before exposing a link, including to
//! older peers. A paired reader learns no additional setup state.
use crate::secure::{Relay, device_via, dialer, direct_route, relayed_with};
use crate::support::{Fixture, client, url, within};
use vessel_client::{
    Connector as _,
    secure::{RouteTimeouts, pair_over},
};
use vessel_proto::{ErrorCode, Scope};
use vessel_rpc::CallError;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn no_route_refuses_and_admin_scope_is_still_required_after_restart() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture.start().await;
        let owner = fixture.connect(&node).await;
        let Err(CallError::Remote(error)) = owner.invite_device(None, 600).await else {
            panic!("expected a no-route refusal");
        };
        assert_eq!(
            error,
            vessel_testkit::device_fixtures::unreachable_invitation()
        );
        assert!(owner.devices().await.unwrap().is_empty());
        node.stop().await.unwrap();

        let relay = Relay::start().await;
        let node = relayed_with(&fixture, &[&relay.url], |_| {}).await;
        let owner = fixture.connect(&node).await;
        let offer = owner
            .invite_with_scopes(None, 600, vec![Scope::Read])
            .await
            .unwrap()
            .link
            .decode()
            .unwrap();
        let (paired, _) = pair_over(&dialer(), &offer, "reader", RouteTimeouts::default())
            .await
            .unwrap();
        node.stop().await.unwrap();
        let node = fixture.start().await;
        let reader = device_via(dialer(), &paired, &[&direct_route(&node)], &[])
            .connect(client())
            .await
            .unwrap();
        let Err(CallError::Remote(error)) = reader.invite_device(None, 600).await else {
            panic!("expected an Admin scope refusal");
        };
        assert_eq!(error.code, ErrorCode::Forbidden);
        assert_eq!(error.detail, None);
        node.stop().await.unwrap();
        relay.stop().await;
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_old_connection_still_gets_actionable_no_route_refusal() {
    use futures_util::{SinkExt as _, StreamExt as _};
    use tokio_tungstenite::tungstenite::{Message, client::IntoClientRequest as _};
    use vessel_proto::{
        ClientFrame, Hello, InviteDevice, Outcome, ProtocolRange, ProtocolVersion, Request as _,
        RequestId, ServerFrame,
    };
    within(async {
        let fixture = Fixture::new();
        let node = fixture.start().await;
        let endpoint = url(&node).to_string();
        let mut upgrade = endpoint.into_client_request().unwrap();
        let mut bearer = format!("Bearer {}", fixture.token().expose())
            .parse::<tokio_tungstenite::tungstenite::http::HeaderValue>()
            .unwrap();
        bearer.set_sensitive(true);
        let _previous = upgrade.headers_mut().insert("authorization", bearer);
        let (mut socket, _) = tokio_tungstenite::connect_async(upgrade).await.unwrap();
        let version = ProtocolVersion::new(4);
        let hello = ClientFrame::Hello(Hello {
            protocol: ProtocolRange {
                min: version,
                max: version,
            },
            client: client(),
        });
        socket
            .send(Message::text(serde_json::to_string(&hello).unwrap()))
            .await
            .unwrap();
        let welcome = socket.next().await.unwrap().unwrap().into_text().unwrap();
        assert!(matches!(
            serde_json::from_str::<ServerFrame>(&welcome).unwrap(),
            ServerFrame::Welcome(_)
        ));
        let request = ClientFrame::Request {
            id: RequestId::new(1),
            call: InviteDevice {
                label: None,
                ttl_secs: 600,
                scopes: None,
            }
            .into_call(),
        };
        socket
            .send(Message::text(serde_json::to_string(&request).unwrap()))
            .await
            .unwrap();
        let reply = socket.next().await.unwrap().unwrap().into_text().unwrap();
        let Ok(ServerFrame::Response {
            outcome: Outcome::Err(error),
            ..
        }) = serde_json::from_str::<ServerFrame>(&reply)
        else {
            panic!("expected a no-route refusal");
        };
        assert_eq!(
            error,
            vessel_testkit::device_fixtures::unreachable_invitation()
        );
        node.stop().await.unwrap();
    })
    .await;
}
