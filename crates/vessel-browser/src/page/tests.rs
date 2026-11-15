//! Protocol-controller fixtures; their invalid PNG must never be advertised as paint.
use super::*;
use crate::cdp::Endpoint;
use futures_util::{SinkExt as _, StreamExt as _};
use serde::Deserialize;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::Message;
use vessel_proto::browser::{BrowserHistory, ViewportSide};
const FIXTURE_DEADLINE: Duration = Duration::from_secs(20);

fn viewport(width: u32, height: u32) -> ViewportSize {
    ViewportSize {
        width: ViewportSide::try_from(width).unwrap(),
        height: ViewportSide::try_from(height).unwrap(),
    }
}
#[test]
fn raster_admission_refuses_large_combined_dimensions_before_engine_allocation() {
    assert!(surface(viewport(8192, 8192)).is_err());
    assert!(surface(viewport(8192, 1024)).is_ok());
    assert!(surface(viewport(390, 844)).is_ok());
}

#[derive(Deserialize)]
struct EvaluatedValue {
    value: u8,
}

#[tokio::test]
async fn actual_target_requests_check_history_surface_and_uncertain_delivery() {
    tokio::time::timeout(FIXTURE_DEADLINE, async {
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
            .await
            .unwrap();
        let endpoint = Endpoint::from_launcher(&format!(
            "ws://{}/devtools/browser/page-fixture",
            listener.local_addr().unwrap()
        ))
        .unwrap();
        let peer = tokio::spawn(peer(listener));
        let (client, _events) = Client::connect(&endpoint).await.unwrap();
        let url: BrowserUrl = "https://example.test/".parse().unwrap();
        let mut page = Page::open(client.clone(), url.clone(), viewport(390, 844))
            .await
            .unwrap();
        assert_eq!(page.revision(), BrowserRevision::ZERO);
        assert_eq!(page.history().await.unwrap(), BrowserHistory::Both);
        page.traverse(HistoryDirection::Back).await.unwrap();
        page.traverse(HistoryDirection::Forward).await.unwrap();
        assert!(matches!(
            page.traverse(HistoryDirection::Back).await,
            Err(PageError::NoHistory)
        ));
        assert!(matches!(
            page.traverse(HistoryDirection::Forward).await,
            Err(PageError::Destination)
        ));
        page.navigate(url).await.unwrap();
        page.reload().await.unwrap();
        page.stop().await.unwrap();
        assert!(matches!(
            page.resize(viewport(8192, 8192)).await,
            Err(PageError::Surface)
        ));
        page.resize(viewport(1280, 720)).await.unwrap();
        let value = page
            .evaluate(RequiredInput::try_from("1+2".to_owned()).unwrap())
            .await
            .unwrap();
        assert_eq!(value.remote_object::<EvaluatedValue>().unwrap().value, 3);
        assert!(page.revision() > BrowserRevision::ZERO);
        assert!(!format!("{page:?}").contains("private-target"));
        assert!(matches!(page.capture().await, Err(PageError::Cdp(_))));
        assert!(matches!(page.stop().await, Err(PageError::Uncertain)));
        page.close().await.unwrap();
        client.shutdown().await.unwrap();
        peer.await.unwrap();
    })
    .await
    .unwrap();
}

async fn peer(listener: TcpListener) {
    let (socket, _) = listener.accept().await.unwrap();
    let mut socket = tokio_tungstenite::accept_async(socket).await.unwrap();
    let mut histories = 0_u8;
    while let Some(Ok(frame)) = socket.next().await {
        if frame.is_close() {
            break;
        }
        let call: serde_json::Value = serde_json::from_str(frame.to_text().unwrap()).unwrap();
        let method = call["method"].as_str().unwrap();
        let result = match method {
            "Target.createTarget" => serde_json::json!({"targetId":"private-target"}),
            "Target.attachToTarget" => serde_json::json!({"sessionId":"private-session"}),
            "Target.closeTarget" => serde_json::json!({"success":true}),
            "Page.getNavigationHistory" => {
                histories = histories.checked_add(1).unwrap();
                history(histories)
            }
            "Runtime.evaluate" => serde_json::json!({"result":{"value":3}}),
            "Page.captureScreenshot" => serde_json::json!({"data":"bm90LXBuZw=="}),
            "Page.enable"
            | "Runtime.enable"
            | "Network.enable"
            | "Emulation.setDeviceMetricsOverride"
            | "Page.navigateToHistoryEntry"
            | "Page.navigate"
            | "Page.reload"
            | "Page.stopLoading" => serde_json::json!({}),
            other => panic!("unexpected fixture command {other}"),
        };
        let mut response = serde_json::json!({"id":call["id"],"result":result});
        if let Some(session) = call.get("sessionId") {
            assert_eq!(session, "private-session");
            response["sessionId"] = session.clone();
        }
        socket
            .send(Message::Text(response.to_string().into()))
            .await
            .unwrap();
    }
}
fn history(count: u8) -> serde_json::Value {
    match count {
        1..=3 => {
            serde_json::json!({"currentIndex":1,"entries":[{"id":1,"url":"https://example.test/a"},{"id":2,"url":"https://example.test/b"},{"id":3,"url":"https://example.test/c"}]})
        }
        4 => {
            serde_json::json!({"currentIndex":0,"entries":[{"id":1,"url":"https://example.test/"}]})
        }
        _ => {
            serde_json::json!({"currentIndex":0,"entries":[{"id":1,"url":"https://example.test/"},{"id":2,"url":"file:///private-fixture"}]})
        }
    }
}
