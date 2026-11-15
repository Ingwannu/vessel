//! Protocol 12's `session.search` over the wire (milestone 11, track A2,
//! ADR 0037): what the fake agent was asked and answered is found in every
//! session, newest first and pointing at its record; one session's; an
//! unknown session; an agent searching; an older client refused.

use futures_util::{SinkExt as _, StreamExt as _};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest as _;
use tokio_tungstenite::tungstenite::http::header::AUTHORIZATION;
use vessel_client::NodeClient;
use vessel_node::Node;
use vessel_proto::search::{MessageRole, SearchMessages, SearchPage};
use vessel_proto::{
    AgentEvent, ClientFrame, ErrorCode, EventRecord, Hello, Outcome, ProtocolRange,
    ProtocolVersion, Request as _, RequestId, ServerFrame, SessionEvent, SessionId,
};

use crate::agents::{as_agent, code, credential, structured, vessel_mcp};
use crate::support::{Fixture, ask, client, url, within};

fn request(query: &str, session: Option<SessionId>) -> SearchMessages {
    SearchMessages {
        query: query.parse().unwrap(),
        session,
        before: None,
        limit: None,
    }
}

async fn search(client: &NodeClient, query: &str, session: Option<SessionId>) -> SearchPage {
    client.call(request(query, session)).await.unwrap()
}

/// The `seq` of the prompt record holding `text`.
fn prompt_seq(seen: &[EventRecord], text: &str) -> vessel_proto::EventSeq {
    let holds = |record: &&EventRecord| {
        if let SessionEvent::Agent {
            event:
                AgentEvent::TurnStarted {
                    provenance: None,
                    content,
                    client_message_id: None,
                },
        } = &record.event
        {
            format!("{content:?}").contains(text)
        } else {
            false
        }
    };
    seen.iter()
        .find(holds)
        .map_or_else(|| panic!("no prompt holds {text:?}"), |record| record.seq)
}

/// Asked and answered in two sessions, then searched once the node has
/// stopped and started again (a stopped node's store has every record;
/// a running one persists what its sessions publish a moment after).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn what_was_said_is_found_in_every_session() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture
            .start_with(|config| config.vessel_mcp = Some(vessel_mcp(&fixture)))
            .await;
        let client = fixture.connect(&node).await;
        let mut first = structured(&client, &fixture).await;
        let reply = ask(
            &client,
            &mut first.stream,
            &mut first.seen,
            first.id,
            // The fake agent answers `env NAME` at once, with the name.
            "env ZEBRAFISH_PARSER",
        )
        .await;
        assert_eq!(reply, "env ZEBRAFISH_PARSER unset");
        let mut second = structured(&client, &fixture).await;
        let _reply = ask(
            &client,
            &mut second.stream,
            &mut second.seen,
            second.id,
            "env 로그인세션을",
        )
        .await;
        // An agent reads the fleet's sessions (ADR 0013), so it searches
        // them too; this one's own session started the session searched.
        let (_program, socket, auth) = credential(&client, &mut first).await;
        let agent = as_agent(&socket, &auth).await;
        node.stop().await.unwrap();

        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        let page = search(&client, "ZEBRAFISH", None).await;
        let found: Vec<_> = page
            .hits
            .iter()
            .map(|hit| (hit.session, hit.role))
            .collect();
        assert_eq!(
            found,
            [
                (first.id, MessageRole::Reply),
                (first.id, MessageRole::Prompt)
            ],
            "newest first"
        );
        let prompt = page.hits.last().unwrap();
        assert_eq!(prompt.seq, prompt_seq(&first.seen, "env ZEBRAFISH_PARSER"));
        assert!(
            prompt
                .snippet
                .parts
                .iter()
                .any(|part| part.matched && part.text == "ZEBRAFISH"),
            "{:?}",
            prompt.snippet
        );
        assert_eq!(page.next, None);

        // Two Hangul characters inside a word.
        let hangul = search(&client, "세션", None).await;
        assert!(!hangul.hits.is_empty());
        assert!(hangul.hits.iter().all(|hit| hit.session == second.id));
        // One session's.
        assert!(
            search(&client, "zebrafish", Some(second.id))
                .await
                .hits
                .is_empty()
        );
        assert_eq!(
            search(&client, "zebrafish", Some(first.id))
                .await
                .hits
                .len(),
            2
        );
        assert_eq!(
            code(
                client
                    .call(request("zebrafish", Some(SessionId::random())))
                    .await
            ),
            ErrorCode::NotFound
        );
        drop(agent);
        node.stop().await.unwrap();
    })
    .await;
}

/// An agent's credential searches like a person's; a client that speaks
/// protocol 11 is refused the call.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_agent_searches_and_an_older_client_is_refused() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture
            .start_with(|config| config.vessel_mcp = Some(vessel_mcp(&fixture)))
            .await;
        let owner = fixture.connect(&node).await;
        let mut parent = structured(&owner, &fixture).await;
        let (_program, socket, auth) = credential(&owner, &mut parent).await;
        let agent = as_agent(&socket, &auth).await;
        let page = agent.call(request("anything", None)).await.unwrap();
        assert!(page.hits.iter().all(|hit| hit.session == parent.id));

        let outcome = called_as_version_11(&fixture, &node).await;
        let Outcome::Err(error) = outcome else {
            panic!("refused: {outcome:?}");
        };
        assert_eq!(error.code, ErrorCode::Unsupported, "{error:?}");
        node.stop().await.unwrap();
    })
    .await;
}

/// `session.search` through a raw connection that offers only version 11.
async fn called_as_version_11(fixture: &Fixture, node: &Node) -> Outcome {
    let mut connect = url(node).as_uri().clone().into_client_request().unwrap();
    let bearer = format!("Bearer {}", fixture.token().expose());
    let _previous = connect
        .headers_mut()
        .insert(AUTHORIZATION, bearer.parse().unwrap());
    let (mut socket, _response) = tokio_tungstenite::connect_async(connect).await.unwrap();
    let version = ProtocolVersion::new(11);
    let mut frames = Vec::new();
    for frame in [
        ClientFrame::Hello(Hello {
            protocol: ProtocolRange {
                min: version,
                max: version,
            },
            client: client(),
        }),
        ClientFrame::Request {
            id: RequestId::new(1),
            call: request("anything", None).into_call(),
        },
    ] {
        socket
            .send(Message::text(serde_json::to_string(&frame).unwrap()))
            .await
            .unwrap();
        let Some(Ok(Message::Text(text))) = socket.next().await else {
            panic!("the node answers each frame");
        };
        frames.push(serde_json::from_str::<ServerFrame>(text.as_str()).unwrap());
    }
    match frames.pop() {
        Some(ServerFrame::Response { outcome, .. }) => outcome,
        other => panic!("a response: {other:?}"),
    }
}
