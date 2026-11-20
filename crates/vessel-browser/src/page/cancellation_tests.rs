//! Delayed typed protocol replies, not a browser runtime or profile authority.
use super::*;
use crate::cdp::Endpoint;
use futures_util::{SinkExt as _, StreamExt as _};
use std::time::Duration;
use tokio::{net::TcpListener, sync::oneshot};
use tokio_tungstenite::tungstenite::Message;
const FIXTURE_DEADLINE: Duration = Duration::from_secs(20);
#[derive(Clone, Copy)]
enum Held {
    Reload,
    Close,
}
struct Fixture {
    page: Page,
    client: Client,
    arrived: oneshot::Receiver<()>,
    release: oneshot::Sender<()>,
    acknowledged: oneshot::Receiver<()>,
    peer: tokio::task::JoinHandle<()>,
}
async fn fixture(held: Held) -> Fixture {
    let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
        .await
        .unwrap();
    let endpoint = Endpoint::from_launcher(&format!(
        "ws://{}/devtools/browser/cancel-fixture",
        listener.local_addr().unwrap()
    ))
    .unwrap();
    let (seen, arrived) = oneshot::channel();
    let (release, resume) = oneshot::channel();
    let (ack, acknowledged) = oneshot::channel();
    let peer = tokio::spawn(peer(listener, held, seen, resume, ack));
    let (client, _events) = Client::connect(&endpoint).await.unwrap();
    let page = Page::open(
        client.clone(),
        "https://example.test/".parse().unwrap(),
        tests::viewport(390, 844),
    )
    .await
    .unwrap();
    Fixture {
        page,
        client,
        arrived,
        release,
        acknowledged,
        peer,
    }
}
async fn peer(
    listener: TcpListener,
    held: Held,
    seen: oneshot::Sender<()>,
    resume: oneshot::Receiver<()>,
    acknowledged: oneshot::Sender<()>,
) {
    let (socket, _) = listener.accept().await.unwrap();
    let mut socket = tokio_tungstenite::accept_async(socket).await.unwrap();
    let mut barrier = Some((seen, resume));
    let mut acknowledged = Some(acknowledged);
    let mut closes = 0_u8;
    while let Some(Ok(frame)) = socket.next().await {
        if frame.is_close() {
            break;
        }
        let call: serde_json::Value = serde_json::from_str(frame.to_text().unwrap()).unwrap();
        let method = call["method"].as_str().unwrap();
        let held_method = match held {
            Held::Reload => "Page.reload",
            Held::Close => "Target.closeTarget",
        };
        if method == held_method
            && let Some((seen, resume)) = barrier.take()
        {
            seen.send(()).unwrap();
            resume.await.unwrap();
        }
        let result = match method {
            "Target.createTarget" => serde_json::json!({"targetId":"owned-target"}),
            "Target.attachToTarget" => serde_json::json!({"sessionId":"owned-session"}),
            "Target.closeTarget" => {
                closes = closes.checked_add(1).unwrap();
                assert_eq!(closes, 1, "duplicate backend close");
                serde_json::json!({"success":true})
            }
            "Page.enable"
            | "Runtime.enable"
            | "Network.enable"
            | "Emulation.setDeviceMetricsOverride"
            | "Page.reload" => serde_json::json!({}),
            method => panic!("unexpected effect after cancellation: {method}"),
        };
        let mut response = serde_json::json!({"id":call["id"],"result":result});
        if let Some(session) = call.get("sessionId") {
            response["sessionId"] = session.clone();
        }
        socket
            .send(Message::Text(response.to_string().into()))
            .await
            .unwrap();
        if method == held_method
            && let Some(acknowledged) = acknowledged.take()
        {
            acknowledged.send(()).unwrap();
        }
    }
}
#[tokio::test]
async fn dropped_reload_stays_uncertain_until_retained_owner_closes_target() {
    tokio::time::timeout(FIXTURE_DEADLINE, async {
        let mut f = fixture(Held::Reload).await;
        {
            let mut reload = Box::pin(f.page.reload());
            tokio::select! {
                biased;
                result=&mut reload=>panic!("held reload completed early: {result:?}"),
                seen=&mut f.arrived=>seen.unwrap(),
            }
        }
        assert!(matches!(f.page.stop().await, Err(PageError::Uncertain)));
        f.release.send(()).unwrap();
        f.page.close().await.unwrap();
        f.page.revision = serde_json::from_str(&u64::MAX.to_string()).unwrap();
        assert!(matches!(f.page.invalidate(), Err(PageError::Closed)));
        assert_eq!(f.page.delivery, DeliveryState::Closed);
        f.page.close().await.unwrap();
        assert!(matches!(f.page.stop().await, Err(PageError::Closed)));
        assert!(matches!(f.page.snapshot().await, Err(PageError::Closed)));
        assert!(matches!(
            f.page
                .human_input(vessel_proto::browser::HumanInput::ReleaseAll)
                .await,
            Err(PageError::Closed)
        ));
        f.client.shutdown().await.unwrap();
        f.peer.await.unwrap();
    })
    .await
    .unwrap();
}
#[tokio::test]
async fn dropped_close_keeps_the_owned_target_and_does_not_invent_an_ack() {
    tokio::time::timeout(FIXTURE_DEADLINE, async {
        let mut f = fixture(Held::Close).await;
        {
            let mut close = Box::pin(f.page.close());
            tokio::select! {
                biased;
                result=&mut close=>panic!("held close completed early: {result:?}"),
                seen=&mut f.arrived=>seen.unwrap(),
            }
        }
        assert_eq!(
            f.page.target,
            TargetId::try_from("owned-target".to_owned()).unwrap()
        );
        assert_eq!(f.page.delivery, DeliveryState::Uncertain);
        assert!(matches!(f.page.stop().await, Err(PageError::Uncertain)));
        f.release.send(()).unwrap();
        f.acknowledged.await.unwrap();
        assert_eq!(f.page.delivery, DeliveryState::Uncertain);
        // The late backend ACK was for the cancelled caller; the owner cannot
        // claim it consumed that ACK. Engine close settles physical lifetime.
        f.client.shutdown().await.unwrap();
        f.peer.await.unwrap();
    })
    .await
    .unwrap();
}
