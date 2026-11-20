//! Protocol-controller fixtures; their invalid PNG must never be advertised as paint.
use super::*;
use crate::cdp::Endpoint;
use futures_util::{SinkExt as _, StreamExt as _};
use serde::Deserialize;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::Message;
use vessel_proto::browser::{BrowserHistory, ScreenshotExtent, ViewportSide};
const FIXTURE_DEADLINE: Duration = Duration::from_secs(20);

pub(super) fn viewport(width: u32, height: u32) -> ViewportSize {
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
        let snapshot = page.snapshot().await.unwrap();
        assert_eq!(
            snapshot.document.address.as_str(),
            "https://example.test/snapshot-document"
        );
        assert_eq!(snapshot.document.title.as_str(), "Own traversal title");
        assert!(
            snapshot
                .content
                .text
                .as_str()
                .contains("button \"Own protocol fixture\"")
        );
        let old: vessel_proto::browser::ElementRef = "@e1".parse().unwrap();
        assert!(page.references.resolve(&old).is_ok());
        assert!(matches!(page.snapshot().await, Err(PageError::Navigation)));
        assert!(page.references.resolve(&old).is_err());
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
        human_input(&mut page).await;
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
        let before = page.revision();
        let function = RequiredInput::try_from("async () => ({owned: 7})".to_owned()).unwrap();
        let observed = page.evaluate_function(&function).await.unwrap();
        assert_eq!(observed.text.as_str(), r#"{"owned":7}"#);
        assert_eq!(
            observed.completeness,
            vessel_proto::browser::TextCompleteness::Whole
        );
        assert!(page.revision() > before);
        capture::tests::cases(&mut page).await;
        assert!(page.revision() > BrowserRevision::ZERO);
        assert!(!format!("{page:?}").contains("private-target"));
        assert!(matches!(
            page.capture_extent(ScreenshotExtent::Viewport).await,
            Err(PageError::Cdp(_))
        ));
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
    let mut frame_queries = 0_u8;
    let mut metrics = 0_u8;
    let mut captures = 0_u8;
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
            "Page.getFrameTree" => {
                frame_queries = frame_queries.checked_add(1).unwrap();
                frame_result(frame_queries, captures)
            }
            "Page.createIsolatedWorld" => {
                assert_eq!(call["params"]["grantUniveralAccess"], false);
                serde_json::json!({"executionContextId":7})
            }
            "Runtime.callFunctionOn" => snapshot_result(&call),
            "Runtime.evaluate" => {
                assert_eq!(call["params"]["userGesture"], false);
                if call["params"]["expression"] == "1+2" {
                    serde_json::json!({"result":{"value":3}})
                } else {
                    assert!(
                        call["params"]["expression"]
                            .as_str()
                            .unwrap()
                            .contains("async () => ({owned: 7})")
                    );
                    serde_json::json!({"result":{"value":{"kind":"value","text":"{\"owned\":7}","completeness":"whole"}}})
                }
            }
            "Page.getLayoutMetrics" => {
                metrics = metrics.checked_add(1).unwrap();
                if metrics == 1 {
                    assert_eq!(captures, 0);
                    serde_json::json!({"cssContentSize":{"width":8192,"height":8192}})
                } else {
                    if metrics == 2 {
                        assert_eq!(captures, 0);
                    }
                    serde_json::json!({"cssContentSize":{"width":390.25,"height":1700.01}})
                }
            }
            "Page.captureScreenshot" => {
                if call["params"].get("clip").is_some() {
                    captures = captures.checked_add(1).unwrap();
                    assert_eq!(call["params"]["captureBeyondViewport"], true);
                    assert_eq!(
                        call["params"]["clip"],
                        serde_json::json!({"x":0,"y":0,"width":391,"height":1701,"scale":1})
                    );
                    // Deliberately HEADER-ONLY: protocol/geometry fixture, not pixels/paint.
                    let encoded = if captures == 2 {
                        "iVBORw0KGgoAAAANSUhEUgAAAAEAAAAB"
                    } else {
                        "iVBORw0KGgoAAAANSUhEUgAAAYcAAAal"
                    };
                    serde_json::json!({"data":encoded})
                } else {
                    serde_json::json!({"data":"bm90LXBuZw=="})
                }
            }
            "Input.dispatchKeyEvent"
            | "Input.dispatchMouseEvent"
            | "Input.insertText"
            | "Page.enable"
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

async fn human_input(page: &mut Page) {
    use vessel_proto::browser::{HumanInput, InputText};
    for input in [
        serde_json::json!({"kind":"pointer","position":{"x":10,"y":20},"action":{"kind":"press","button":"left"},"modifiers":{}}),
        serde_json::json!({"kind":"wheel","position":{"x":10,"y":20},"delta_x":0.5,"delta_y":12}),
        serde_json::json!({"kind":"release_all"}),
    ] {
        page.human_input(serde_json::from_value(input).unwrap())
            .await
            .unwrap();
    }
    page.human_input(HumanInput::Text {
        text: InputText::try_from("human-input-canary".to_owned()).unwrap(),
    })
    .await
    .unwrap();
    for key in ["Control", "a"] {
        page.human_input(HumanInput::Key {
            key: RequiredInput::try_from(key.to_owned()).unwrap(),
            action: vessel_proto::browser::KeyAction::Down,
            modifiers: vessel_proto::browser::KeyModifiers::default(),
        })
        .await
        .unwrap();
    }
    page.human_input(HumanInput::ReleaseAll).await.unwrap();
    let invalid:HumanInput=serde_json::from_value(serde_json::json!({"kind":"pointer","position":{"x":1000,"y":0},"action":{"kind":"move"},"modifiers":{}})).unwrap();
    assert!(matches!(
        page.human_input(invalid).await,
        Err(PageError::Surface)
    ));
    let key: HumanInput = serde_json::from_value(
        serde_json::json!({"kind":"key","key":"UnimplementedKey","action":"down","modifiers":{}}),
    )
    .unwrap();
    assert!(matches!(
        page.human_input(key).await,
        Err(PageError::Keyboard(requests::KeyboardError::Unsupported))
    ));
}

fn snapshot_result(call: &serde_json::Value) -> serde_json::Value {
    use serde_json::json;
    assert_eq!(call["params"]["executionContextId"], 7);
    assert_eq!(
        call["params"]["functionDeclaration"],
        crate::snapshot::SOURCE
    );
    assert_eq!(call["params"]["userGesture"], false);
    let first = call["params"]["arguments"][0]["value"]["firstRef"]
        .as_str()
        .unwrap();
    json!({"result":{"value":{"document":{"address":"https://example.test/snapshot-document","title":"Own traversal title"},"nodes":[{
        "kind":"element","depth":0,"role":"button","name":"Own protocol fixture",
        "reference":format!("@e{first}"),
        "fingerprint":{"tag":"button","id":"","input_type":"","name":"","value":""},
        "state":{"checked":null,"expanded":null,"selected":null,"disabled":false,"readonly":false,"level":null}
    }],"visited":1,"frames":0,"truncated":false}}})
}

#[tokio::test]
async fn revision_exhaustion_requires_target_close_instead_of_more_commands() {
    tokio::time::timeout(FIXTURE_DEADLINE, async {
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
            .await
            .unwrap();
        let endpoint = Endpoint::from_launcher(&format!(
            "ws://{}/devtools/browser/exhaustion-fixture",
            listener.local_addr().unwrap()
        ))
        .unwrap();
        let peer = tokio::spawn(peer(listener));
        let (client, _events) = Client::connect(&endpoint).await.unwrap();
        let mut page = Page::open(
            client.clone(),
            "https://example.test/".parse().unwrap(),
            viewport(390, 844),
        )
        .await
        .unwrap();
        page.revision = serde_json::from_str(&u64::MAX.to_string()).unwrap();
        assert!(matches!(page.reload().await, Err(PageError::Exhausted)));
        assert!(matches!(page.stop().await, Err(PageError::Uncertain)));
        page.close().await.unwrap();
        client.shutdown().await.unwrap();
        peer.await.unwrap();
    })
    .await
    .unwrap();
}

fn frame_result(queries: u8, captures: u8) -> serde_json::Value {
    let loader = if queries == 4 || captures == 3 {
        "changed-loader"
    } else {
        "first-loader"
    };
    serde_json::json!({"frameTree":{"frame":{"id":"own-frame","loaderId":loader}}})
}
