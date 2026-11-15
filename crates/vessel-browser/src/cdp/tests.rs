//! Scripted private debugger peers test transport only; no fake engine/frame is advertised.

use std::time::Duration;

use futures_util::{SinkExt as _, StreamExt as _};
use serde::Deserialize;
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::Message;
use vessel_proto::browser::{BrowserEffect, InputText};

use super::identity::{CommandId, SessionId, TargetId};
use super::requests::{Attach, InsertText, Version};
use super::{CdpError, Client, Endpoint};

const FIXTURE_DEADLINE: Duration = Duration::from_secs(20);
const EXCESS_EVENT_COUNT: usize = 33;

#[derive(Deserialize)]
struct CommandIdentity {
    id: CommandId,
}

#[derive(Deserialize)]
struct NumberResult {
    value: u8,
}

#[test]
fn sealed_request_shapes_match_the_foreign_protocol_and_bound_decoding() {
    use super::requests::*;
    use vessel_proto::browser::{RequiredInput, ViewportSide, ViewportSize};
    fn method<R: Request>(request: &R, expected: &str) {
        assert_eq!(
            serde_json::to_string(&R::METHOD).unwrap(),
            format!("\"{expected}\"")
        );
        assert!(serde_json::to_string(request).is_ok());
    }
    method(&CloseBrowser {}, "Browser.close");
    method(&EnablePage {}, "Page.enable");
    method(&EnableRuntime {}, "Runtime.enable");
    method(&EnableNetwork {}, "Network.enable");
    method(&Stop {}, "Page.stopLoading");
    method(&Reload {}, "Page.reload");
    method(&History {}, "Page.getNavigationHistory");
    method(&Capture {}, "Page.captureScreenshot");
    method(&ClearViewport {}, "Emulation.clearDeviceMetricsOverride");
    let created: Created =
        serde_json::from_str(r#"{"targetId":"private-target-fixture"}"#).unwrap();
    method(
        &CloseTarget {
            target_id: created.target_id,
        },
        "Target.closeTarget",
    );
    let closed: Closed = serde_json::from_str(r#"{"success":false}"#).unwrap();
    assert!(!closed.success);
    method(
        &CreateTarget {
            url: "https://example.test/".parse().unwrap(),
            background: true,
        },
        "Target.createTarget",
    );
    method(
        &Navigate {
            url: "https://example.test/path".parse().unwrap(),
        },
        "Page.navigate",
    );
    let navigation: Navigated = serde_json::from_str(r#"{"errorText":"fixture-refusal"}"#).unwrap();
    assert!(navigation.error_text.is_some());
    let history: NavigationHistory =
        serde_json::from_str(r#"{"currentIndex":0,"entries":[{"id":7,"url":"about:blank"}]}"#)
            .unwrap();
    assert_eq!(history.current_index.position(), Some(0));
    let entry = history.entries.into_iter().next().unwrap();
    assert_eq!(entry.url.as_str(), "about:blank");
    method(
        &NavigateHistory { entry_id: entry.id },
        "Page.navigateToHistoryEntry",
    );
    let negative: HistoryIndex = serde_json::from_str("-1").unwrap();
    assert_eq!(negative.position(), None);
    let evaluate = Evaluate::new(RequiredInput::try_from("1+1".to_owned()).unwrap());
    assert_eq!(
        serde_json::to_value(&evaluate).unwrap(),
        serde_json::json!({"expression":"1+1","returnByValue":true,"awaitPromise":true,"userGesture":false})
    );
    method(&evaluate, "Runtime.evaluate");
    let evaluated: Evaluated = serde_json::from_str(r#"{"result":{"value":2}}"#).unwrap();
    assert_eq!(evaluated.remote_object::<NumberResult>().unwrap().value, 2);
    let exception: Evaluated =
        serde_json::from_str(r#"{"result":{},"exceptionDetails":{"text":"private exception"}}"#)
            .unwrap();
    assert!(matches!(
        exception.remote_object::<NumberResult>(),
        Err(EvaluationError::Exception)
    ));
    assert!(!format!("{exception:?}").contains("private exception"));
    // Invalid bytes must fail before becoming any frame or image fixture.
    assert!(
        serde_json::from_str::<Captured>(r#"{"data":"bm90LXBuZw=="}"#)
            .map(|capture| capture.data.bytes().len())
            .is_err()
    );
    let viewport = SetViewport::new(ViewportSize {
        width: ViewportSide::try_from(390).unwrap(),
        height: ViewportSide::try_from(844).unwrap(),
    });
    assert_eq!(
        serde_json::to_value(&viewport).unwrap(),
        serde_json::json!({"width":390,"height":844,"deviceScaleFactor":1,"mobile":false})
    );
    method(&viewport, "Emulation.setDeviceMetricsOverride");
}

#[tokio::test]
async fn essential_events_keep_private_session_identity_without_raw_payload_access() {
    use super::wire::EventKind;
    tokio::time::timeout(FIXTURE_DEADLINE,async {
        let (listener,endpoint)=listener().await;
        let peer=tokio::spawn(async move {
            let (socket,_)=listener.accept().await.unwrap();
            let mut socket=tokio_tungstenite::accept_async(socket).await.unwrap();
            socket.send(Message::Text(r#"{"method":"Page.loadEventFired","sessionId":"private-session-canary","params":{"timestamp":1}}"#.into())).await.unwrap();
            while let Some(Ok(message))=socket.next().await {if message.is_close(){break;}}
        });
        let (client,mut events)=Client::connect(&endpoint).await.unwrap();
        let event=events.next().await.unwrap();
        assert_eq!(event.kind(),EventKind::Loaded);
        assert!(event.session().is_some());
        assert!(!format!("{event:?}").contains("private-session-canary"));
        client.shutdown().await.unwrap();
        peer.await.unwrap();
    }).await.unwrap();
}

#[derive(Deserialize)]
struct Call {
    id: CommandId,
    method: FakeMethod,
}
#[derive(Deserialize)]
enum FakeMethod {
    #[serde(rename = "Browser.getVersion")]
    Version,
    #[serde(rename = "Target.attachToTarget")]
    Attach,
}

async fn listener() -> (TcpListener, Endpoint) {
    let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
        .await
        .unwrap();
    let endpoint = Endpoint::from_launcher(&format!(
        "ws://{}/devtools/browser/transport-fixture",
        listener.local_addr().unwrap()
    ))
    .unwrap();
    (listener, endpoint)
}

#[tokio::test]
async fn out_of_order_responses_are_correlated_to_their_own_typed_requests() {
    tokio::time::timeout(FIXTURE_DEADLINE,async {
        let (listener,endpoint)=listener().await;
        let peer=tokio::spawn(async move {
            let (socket,_)=listener.accept().await.unwrap();
            let mut socket=tokio_tungstenite::accept_async(socket).await.unwrap();
            let mut calls=Vec::new();
            while calls.len()<2 {
                let frame=socket.next().await.unwrap().unwrap();
                let call:Call=serde_json::from_str(frame.to_text().unwrap()).unwrap();
                calls.push(call);
            }
            for call in calls.into_iter().rev() {
                let result=match call.method {
                    FakeMethod::Version=>serde_json::json!({"protocolVersion":"fixture","product":"not-a-qualified-browser","revision":"fixture","userAgent":"fixture","jsVersion":"fixture"}),
                    FakeMethod::Attach=>serde_json::json!({"sessionId":"private-session-fixture"}),
                };
                let message=serde_json::json!({"id":call.id,"result":result});
                socket.send(Message::Text(message.to_string().into())).await.unwrap();
            }
            while let Some(Ok(message))=socket.next().await {
                if message.is_close() {break;}
            }
        });
        let (client,_events)=Client::connect(&endpoint).await.unwrap();
        let target=TargetId::try_from("private-target-fixture".to_owned()).unwrap();
        let (version,attached)=tokio::join!(client.execute(None,Version{}),client.execute(None,Attach::new(target)));
        let version=version.unwrap();
        assert_eq!(version.product.as_str(),"not-a-qualified-browser");
        for report in [version.protocol_version,version.revision,version.user_agent,version.js_version] {
            assert_eq!(report.as_str(),"fixture");
        }
        assert!(!format!("{:?}",attached.unwrap().session_id).contains("private-session-fixture"));
        client.shutdown().await.unwrap();
        peer.await.unwrap();
    }).await.unwrap();
}

#[tokio::test]
async fn losing_a_reply_after_delivery_cannot_authorize_mutation_replay() {
    tokio::time::timeout(FIXTURE_DEADLINE, async {
        let (listener, endpoint) = listener().await;
        let peer = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            let mut socket = tokio_tungstenite::accept_async(socket).await.unwrap();
            let _delivered = socket.next().await.unwrap().unwrap();
            socket.close(None).await.unwrap();
        });
        let (client, _events) = Client::connect(&endpoint).await.unwrap();
        let error = client
            .execute(
                None,
                InsertText {
                    text: InputText::try_from("typing-canary".to_owned()).unwrap(),
                },
            )
            .await
            .unwrap_err();
        assert!(matches!(
            error,
            CdpError::Disconnected {
                effect: BrowserEffect::Uncertain
            }
        ));
        assert!(!format!("{error:?}").contains("typing-canary"));
        client.shutdown().await.unwrap();
        peer.await.unwrap();
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn essential_signal_overflow_fails_the_connection_without_silently_skipping_state() {
    tokio::time::timeout(FIXTURE_DEADLINE, async {
        let (listener, endpoint) = listener().await;
        let peer = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            let mut socket = tokio_tungstenite::accept_async(socket).await.unwrap();
            for _ in 0..EXCESS_EVENT_COUNT {
                socket
                    .send(Message::Text(
                        r#"{"method":"Page.loadEventFired","params":{}}"#.into(),
                    ))
                    .await
                    .unwrap();
            }
        });
        let (client, _events) = Client::connect(&endpoint).await.unwrap();
        client.wait_closed().await;
        assert!(matches!(client.shutdown().await, Err(CdpError::Busy)));
        peer.await.unwrap();
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn a_reply_from_another_flattened_session_fails_closed() {
    tokio::time::timeout(FIXTURE_DEADLINE, async {
        let (listener, endpoint) = listener().await;
        let peer = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            let mut socket = tokio_tungstenite::accept_async(socket).await.unwrap();
            let frame = socket.next().await.unwrap().unwrap();
            let call: CommandIdentity = serde_json::from_str(frame.to_text().unwrap()).unwrap();
            let reply = serde_json::json!({"id":call.id,"sessionId":"another-target","result":{}});
            socket
                .send(Message::Text(reply.to_string().into()))
                .await
                .unwrap();
            while let Some(Ok(message)) = socket.next().await {
                if message.is_close() {
                    break;
                }
            }
        });
        let (client, _events) = Client::connect(&endpoint).await.unwrap();
        let session = SessionId::try_from("selected-target".to_owned()).unwrap();
        let result = client
            .execute(
                Some(session),
                InsertText {
                    text: InputText::try_from("session-canary".to_owned()).unwrap(),
                },
            )
            .await;
        assert!(matches!(result, Err(CdpError::Protocol)));
        assert!(matches!(client.shutdown().await, Err(CdpError::Protocol)));
        peer.await.unwrap();
    })
    .await
    .unwrap();
}
