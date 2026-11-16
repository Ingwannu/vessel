//! Controlled protocol replies prove polling/fencing, not a real DOM or engine.
use super::*;
use crate::cdp::Endpoint;
use futures_util::{SinkExt as _, StreamExt as _};
use std::time::Duration;
use tokio::{net::TcpListener, sync::oneshot};
use tokio_tungstenite::tungstenite::Message;
use vessel_proto::browser::{WaitBudget, WaitFor, WaitTimeout};

const FIXTURE_DEADLINE: Duration = Duration::from_secs(20);
#[derive(Clone, Copy)]
enum Scenario {
    Match,
    Navigate,
    Held,
    NoMatch,
}
struct Fixture {
    page: Page,
    client: Client,
    arrived: oneshot::Receiver<()>,
    release: oneshot::Sender<()>,
    acknowledged: oneshot::Receiver<()>,
    peer: tokio::task::JoinHandle<()>,
}
async fn fixture(scenario: Scenario) -> Fixture {
    let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
        .await
        .unwrap();
    let endpoint = Endpoint::from_launcher(&format!(
        "ws://{}/devtools/browser/wait-fixture",
        listener.local_addr().unwrap()
    ))
    .unwrap();
    let (seen, arrived) = oneshot::channel();
    let (release, resume) = oneshot::channel();
    let (ack, acknowledged) = oneshot::channel();
    let peer = tokio::spawn(peer(listener, scenario, seen, resume, ack));
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
async fn finish(mut fixture: Fixture) {
    fixture.page.close().await.unwrap();
    fixture.client.shutdown().await.unwrap();
    fixture.peer.await.unwrap();
}

#[tokio::test]
async fn wait_polls_until_observed_text_or_url_match_and_preserves_kind() {
    tokio::time::timeout(FIXTURE_DEADLINE, async {
        let mut fixture = fixture(Scenario::Match).await;
        let text = WaitFor::Text(RequiredInput::try_from("own needle".to_owned()).unwrap());
        assert_eq!(
            fixture.page.wait(text, WaitBudget::Default).await.unwrap(),
            wait::MatchedCondition::Text
        );
        let url = WaitFor::Url(RequiredInput::try_from("own substring".to_owned()).unwrap());
        assert_eq!(
            fixture.page.wait(url, WaitBudget::Default).await.unwrap(),
            wait::MatchedCondition::Url
        );
        finish(fixture).await;
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn matched_probe_from_a_replaced_loader_never_publishes_success() {
    tokio::time::timeout(FIXTURE_DEADLINE, async {
        let mut fixture = fixture(Scenario::Navigate).await;
        let text = WaitFor::Text(RequiredInput::try_from("own needle".to_owned()).unwrap());
        assert!(matches!(
            fixture.page.wait(text, WaitBudget::Default).await,
            Err(PageError::Navigation)
        ));
        assert!(fixture.page.revision() > BrowserRevision::ZERO);
        finish(fixture).await;
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn exhausted_wait_budget_never_becomes_a_synthetic_match() {
    tokio::time::timeout(FIXTURE_DEADLINE, async {
        let mut fixture = fixture(Scenario::NoMatch).await;
        let text = WaitFor::Text(RequiredInput::try_from("own needle".to_owned()).unwrap());
        let budget = WaitBudget::Explicit(WaitTimeout::try_from(1).unwrap());
        assert!(matches!(
            fixture.page.wait(text, budget).await,
            Err(PageError::WaitTimeout(_))
        ));
        finish(fixture).await;
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn cancelling_an_inflight_probe_cannot_rearm_the_uncertain_page() {
    tokio::time::timeout(FIXTURE_DEADLINE, async {
        let mut fixture = fixture(Scenario::Held).await;
        let text = WaitFor::Text(RequiredInput::try_from("own needle".to_owned()).unwrap());
        let mut pending = Box::pin(fixture.page.wait(text, WaitBudget::Default));
        tokio::select! {
            arrived = fixture.arrived => { arrived.unwrap(); },
            result = &mut pending => panic!("held probe completed early: {result:?}"),
        }
        drop(pending);
        fixture.release.send(()).unwrap();
        fixture.acknowledged.await.unwrap();
        assert!(matches!(
            fixture.page.history().await,
            Err(PageError::Uncertain)
        ));
        fixture.page.close().await.unwrap();
        fixture.client.shutdown().await.unwrap();
        fixture.peer.await.unwrap();
    })
    .await
    .unwrap();
}

async fn peer(
    listener: TcpListener,
    scenario: Scenario,
    seen: oneshot::Sender<()>,
    resume: oneshot::Receiver<()>,
    ack: oneshot::Sender<()>,
) {
    let (socket, _) = listener.accept().await.unwrap();
    let mut socket = tokio_tungstenite::accept_async(socket).await.unwrap();
    let mut barrier = Some((seen, resume, ack));
    let mut probes = 0_u32;
    let mut frames = 0_u32;
    while let Some(Ok(message)) = socket.next().await {
        if message.is_close() {
            break;
        }
        let call: serde_json::Value = serde_json::from_str(message.to_text().unwrap()).unwrap();
        let result = match call["method"].as_str().unwrap() {
            "Target.createTarget" => serde_json::json!({"targetId":"wait-target"}),
            "Target.attachToTarget" => serde_json::json!({"sessionId":"wait-session"}),
            "Target.closeTarget" => serde_json::json!({"success":true}),
            "Page.getFrameTree" => {
                frames = frames.checked_add(1).unwrap();
                let loader = match scenario {
                    Scenario::Navigate if frames == 2 => "replaced",
                    Scenario::Navigate | Scenario::Match | Scenario::Held | Scenario::NoMatch => {
                        "initial"
                    }
                };
                serde_json::json!({"frameTree":{"frame":{"id":"wait-frame","loaderId":loader}}})
            }
            "Page.createIsolatedWorld" => serde_json::json!({"executionContextId":7}),
            "Runtime.callFunctionOn" => {
                probes = probes.checked_add(1).unwrap();
                assert_eq!(call["params"]["executionContextId"], 7);
                assert_eq!(call["params"]["userGesture"], false);
                let condition = &call["params"]["arguments"][0]["value"];
                let matched = match scenario {
                    Scenario::Match => {
                        if probes <= 2 {
                            assert_eq!(condition["kind"], "text");
                            assert_eq!(condition["value"], "own needle");
                        } else {
                            assert_eq!(condition["kind"], "url");
                            assert_eq!(condition["value"], "own substring");
                        }
                        probes > 1
                    }
                    Scenario::Navigate => true,
                    Scenario::NoMatch => false,
                    Scenario::Held => {
                        let (seen, resume, ack) = barrier.take().unwrap();
                        seen.send(()).unwrap();
                        resume.await.unwrap();
                        ack.send(()).unwrap();
                        true
                    }
                };
                serde_json::json!({"result":{"type":"boolean","value":matched}})
            }
            "Page.enable"
            | "Runtime.enable"
            | "Network.enable"
            | "Emulation.setDeviceMetricsOverride" => serde_json::json!({}),
            unknown => panic!("unexpected wait fixture method: {unknown}"),
        };
        let reply =
            serde_json::json!({"id":call["id"],"sessionId":call.get("sessionId"),"result":result});
        socket
            .send(Message::Text(reply.to_string().into()))
            .await
            .unwrap();
    }
}
