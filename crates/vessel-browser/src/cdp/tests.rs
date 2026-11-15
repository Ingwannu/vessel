//! Scripted private debugger peers test transport only; no fake engine/frame is advertised.

use std::time::Duration;

use futures_util::{SinkExt as _, StreamExt as _};
use serde::Deserialize;
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::Message;
use vessel_proto::browser::{BrowserEffect, InputText};

use super::identity::{CommandId, TargetId};
use super::requests::{Attach, InsertText, Version};
use super::{Client, Endpoint, Error};

const FIXTURE_DEADLINE: Duration = Duration::from_secs(20);
const EXCESS_EVENT_COUNT: usize = 33;

#[derive(Deserialize)]
struct Call {
    id: CommandId,
    method: FakeMethod,
}
#[derive(Deserialize)]
enum FakeMethod {
    #[serde(rename="Browser.getVersion")] Version,
    #[serde(rename="Target.attachToTarget")] Attach,
}

async fn listener()->(TcpListener,Endpoint) {
    let listener=TcpListener::bind((std::net::Ipv4Addr::LOCALHOST,0)).await.unwrap();
    let endpoint=Endpoint::from_launcher(&format!("ws://{}/devtools/browser/transport-fixture",listener.local_addr().unwrap())).unwrap();
    (listener,endpoint)
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
        assert_eq!(version.unwrap().product.as_str(),"not-a-qualified-browser");
        assert!(!format!("{:?}",attached.unwrap().session_id).contains("private-session-fixture"));
        client.shutdown().await.unwrap();
        peer.await.unwrap();
    }).await.unwrap();
}

#[tokio::test]
async fn losing_a_reply_after_delivery_cannot_authorize_mutation_replay() {
    tokio::time::timeout(FIXTURE_DEADLINE,async {
        let (listener,endpoint)=listener().await;
        let peer=tokio::spawn(async move {
            let (socket,_)=listener.accept().await.unwrap();
            let mut socket=tokio_tungstenite::accept_async(socket).await.unwrap();
            let _delivered=socket.next().await.unwrap().unwrap();
            socket.close(None).await.unwrap();
        });
        let (client,_events)=Client::connect(&endpoint).await.unwrap();
        let error=client.execute(None,InsertText{text:InputText::try_from("typing-canary".to_owned()).unwrap()}).await.unwrap_err();
        assert!(matches!(error,Error::Disconnected{effect:BrowserEffect::Uncertain}));
        assert!(!format!("{error:?}").contains("typing-canary"));
        client.shutdown().await.unwrap();
        peer.await.unwrap();
    }).await.unwrap();
}

#[tokio::test]
async fn essential_signal_overflow_fails_the_connection_without_silently_skipping_state() {
    tokio::time::timeout(FIXTURE_DEADLINE,async {
        let (listener,endpoint)=listener().await;
        let peer=tokio::spawn(async move {
            let (socket,_)=listener.accept().await.unwrap();
            let mut socket=tokio_tungstenite::accept_async(socket).await.unwrap();
            for _ in 0..EXCESS_EVENT_COUNT {
                socket.send(Message::Text(r#"{"method":"Page.loadEventFired","params":{}}"#.into())).await.unwrap();
            }
        });
        let (client,_events)=Client::connect(&endpoint).await.unwrap();
        client.wait_closed().await;
        assert!(matches!(client.shutdown().await,Err(Error::Busy)));
        peer.await.unwrap();
    }).await.unwrap();
}
