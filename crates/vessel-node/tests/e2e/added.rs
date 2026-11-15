//! Agents added by their command (protocol 11, ADR 0032), end to end: the
//! testkit's fake ACP agent, defined by its absolute path, runs a
//! conversation like one of the four; it gets only what its definition
//! names; an older client never sees it; only a person adds one.

use std::collections::BTreeSet;

use futures_util::{SinkExt as _, StreamExt as _};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest as _;
use tokio_tungstenite::tungstenite::http::header::AUTHORIZATION;
use vessel_client::{CallError, NodeClient};
use vessel_proto::agents::{
    AddAgent, AgentDefinition, AgentDetail, AgentToAdd, AgentVariable, ListAgents, RemoveAgent,
};
use vessel_proto::{
    AgentName, Call, ClientFrame, ConfigValue, ErrorCode, EventSeq, HarnessKind, HarnessRef, Hello,
    KillSession, ListHarnesses, ListSessions, Outcome, ProtocolRange, ProtocolVersion, Reply,
    Request as _, RequestId, ServerFrame, SessionId, SessionMode, SessionSpec, SessionStatus,
    TerminalSize,
};
use vessel_testkit::Fake;

use crate::agents::{as_agent, code, credential, structured, vessel_mcp};
use crate::support::{Fixture, client, prompt, read_until, status_of, url, within};

pub(crate) fn name(text: &str) -> AgentName {
    text.parse().unwrap()
}

/// The fake ACP agent, defined by its absolute path, telling the file it
/// writes its environment's names into.
fn definition(env_file: &std::path::Path) -> AgentDefinition {
    AgentDefinition {
        label: Some("Fake".parse().unwrap()),
        program: Fake::Acp.path().unwrap().to_str().unwrap().parse().unwrap(),
        args: Vec::new(),
        env: vec![
            AgentVariable {
                name: "FAKE_ACP_ENV_FILE".parse().unwrap(),
                value: ConfigValue::Text(env_file.to_str().unwrap().to_owned()),
            },
            AgentVariable {
                name: "FAKE_KEY".parse().unwrap(),
                value: ConfigValue::Secret("FAKE_SECRET".parse().unwrap()),
            },
        ],
        inherit: Vec::new(),
        mcp: Vec::new(),
    }
}

pub(crate) fn add(agent: &str, env_file: &std::path::Path) -> AddAgent {
    AddAgent {
        name: name(agent),
        agent: AgentToAdd::Command {
            definition: definition(env_file),
        },
        replace: false,
    }
}

/// A node that takes up `secrets.toml` within 50 ms (5 s by default).
pub(crate) async fn quick(fixture: &Fixture) -> vessel_node::Node {
    fixture
        .start_with(|config| config.fleet.poll = std::time::Duration::from_millis(50))
        .await
}

/// Set the agent's secret and wait until the node has taken it up.
pub(crate) async fn set_secret(owner: &NodeClient, value: &str) {
    owner
        .secret_set(
            "FAKE_SECRET".parse().unwrap(),
            vessel_proto::SecretValue::new(value.to_owned()),
        )
        .await
        .unwrap();
    let fake = HarnessRef::Added(name("fake"));
    for _ in 0..200 {
        match owner.harness_options(fake.clone(), None).await {
            Err(CallError::Remote(error)) if error.code == ErrorCode::Conflict => {
                tokio::time::sleep(std::time::Duration::from_millis(25)).await;
            }
            _taken => return,
        }
    }
    panic!("the node never took up the secret");
}

fn structured_of(fixture: &Fixture, harness: HarnessRef) -> SessionSpec {
    SessionSpec::new(
        harness,
        SessionMode::Structured {
            model: None,
            resume: None,
            options: Vec::new(),
        },
    )
    .with_cwd(fixture.cwd())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_added_agent_runs_a_conversation_with_only_what_it_is_given() {
    within(async {
        let fixture = Fixture::new();
        let node = quick(&fixture).await;
        let owner = fixture.connect(&node).await;
        let env_file = fixture.cwd().join("env.txt");

        let added = owner.call(add("fake", &env_file)).await.unwrap();
        // Without its secret it does not start: it would lack what its
        // definition says it needs.
        assert_eq!(
            code(
                owner
                    .harness_options(HarnessRef::Added(name("fake")), None)
                    .await
            ),
            ErrorCode::Conflict
        );
        set_secret(&owner, "s3cret-value").await;
        let AgentDetail::Added { added: info } = &added.detail else {
            panic!("an added agent: {added:?}");
        };
        assert_eq!(info.env, ["FAKE_ACP_ENV_FILE".parse().unwrap()]);
        assert_eq!(info.secrets[0].secret.as_str(), "FAKE_SECRET");
        let listed = owner.call(ListAgents {}).await.unwrap();
        let json = serde_json::to_string(&listed).unwrap();
        assert!(!json.contains("s3cret-value"), "{json}");
        let kinds: Vec<String> = listed.iter().map(|agent| agent.agent.to_string()).collect();
        assert_eq!(
            kinds,
            ["claude-code", "codex", "gemini", "opencode", "fake"]
        );

        // Found, and not for terminals.
        let fake = HarnessRef::Added(name("fake"));
        let found = owner.harnesses().await.unwrap();
        let harness = found.iter().find(|found| found.kind == fake).unwrap();
        assert!(!harness.terminal.is_available());

        // What it offers is asked of it, as of the four.
        let offered = owner.harness_options(fake.clone(), None).await.unwrap();
        assert!(!offered.options.is_empty());

        // A conversation, as with any structured session.
        let info = owner
            .create_session(structured_of(&fixture, fake.clone()))
            .await
            .unwrap();
        assert_eq!(info.spec.harness, fake);
        let mut stream = owner.follow(info.id, EventSeq::ZERO).await.unwrap();
        let mut seen = Vec::new();
        read_until(&mut stream, &mut seen, |record| {
            status_of(record) == Some(&SessionStatus::Running)
        })
        .await;
        owner.send(info.id, prompt("hello")).await.unwrap();

        // Its environment: what it was given, nothing of the node's.
        let names: BTreeSet<String> = std::fs::read_to_string(&env_file)
            .unwrap()
            .lines()
            .map(str::to_owned)
            .collect();
        for given in ["FAKE_ACP_ENV_FILE", "FAKE_KEY", "PATH"] {
            assert!(names.contains(given), "{given} missing: {names:?}");
        }
        let home = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
        assert!(names.contains(home), "{names:?}");
        let allowed: BTreeSet<&str> = vessel_harness::PASSED_ON
            .iter()
            .copied()
            .chain(["FAKE_ACP_ENV_FILE", "FAKE_KEY", "PATH", home])
            .collect();
        let extra: Vec<&String> = names
            .iter()
            .filter(|name| {
                !allowed
                    .iter()
                    .any(|allowed| allowed.eq_ignore_ascii_case(name))
            })
            .collect();
        assert!(
            extra.is_empty(),
            "the node's environment reached it: {extra:?}"
        );

        // No terminal: Vessel knows only its ACP command.
        let terminal = SessionSpec::new(
            fake.clone(),
            SessionMode::Terminal {
                args: Vec::new(),
                size: TerminalSize::default(),
            },
        )
        .with_cwd(fixture.cwd());
        assert_eq!(
            code(owner.create_session(terminal).await),
            ErrorCode::Unsupported
        );
        // An agent that is not there.
        assert_eq!(
            code(
                owner
                    .create_session(structured_of(&fixture, HarnessRef::Added(name("nobody"))))
                    .await
            ),
            ErrorCode::NotFound
        );
        owner.kill(info.id).await.unwrap();
        node.stop().await.unwrap();
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn only_a_person_adds_or_removes_an_agent_and_names_stay_one_space() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture
            .start_with(|config| config.vessel_mcp = Some(vessel_mcp(&fixture)))
            .await;
        let owner = fixture.connect(&node).await;
        let env_file = fixture.cwd().join("env.txt");

        // An agent principal reads the list, and adds nothing.
        let mut running = structured(&owner, &fixture).await;
        let (_program, socket, auth) = credential(&owner, &mut running).await;
        let agent = as_agent(&socket, &auth).await;
        assert!(agent.call(ListAgents {}).await.is_ok());
        assert_eq!(
            code(agent.call(add("sneaky", &env_file)).await),
            ErrorCode::Forbidden
        );

        let _added = owner.call(add("fake", &env_file)).await.unwrap();
        // Taken, ignoring case; replacing is asked for by name.
        assert_eq!(
            code(owner.call(add("FAKE", &env_file)).await),
            ErrorCode::Conflict
        );
        let mut replace = add("FAKE", &env_file);
        replace.replace = true;
        let _replaced = owner.call(replace).await.unwrap();
        assert_eq!(
            code(owner.call(RemoveAgent { name: name("fake") }).await),
            ErrorCode::NotFound
        );
        let _removed = owner
            .call(RemoveAgent { name: name("FAKE") })
            .await
            .unwrap();
        let listed = owner.call(ListAgents {}).await.unwrap();
        assert!(listed.iter().all(|agent| agent.agent.added().is_none()));
        node.stop().await.unwrap();
    })
    .await;
}

/// Calls on a raw connection that offers only protocol 10, one at a
/// time, with their outcomes.
async fn as_version_10(
    fixture: &Fixture,
    node: &vessel_node::Node,
    calls: Vec<Call>,
) -> Vec<Outcome> {
    let mut request = url(node).as_uri().clone().into_client_request().unwrap();
    let bearer = format!("Bearer {}", fixture.token().expose());
    let _previous = request
        .headers_mut()
        .insert(AUTHORIZATION, bearer.parse().unwrap());
    let (mut socket, _response) = tokio_tungstenite::connect_async(request).await.unwrap();
    let version = ProtocolVersion::new(10);
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
    let Some(Ok(Message::Text(_welcome))) = socket.next().await else {
        panic!("a welcome");
    };
    let mut outcomes = Vec::new();
    for (at, call) in (1_u64..).zip(calls) {
        let frame = ClientFrame::Request {
            id: RequestId::new(at),
            call,
        };
        socket
            .send(Message::text(serde_json::to_string(&frame).unwrap()))
            .await
            .unwrap();
        let Some(Ok(Message::Text(text))) = socket.next().await else {
            panic!("the node answers each call");
        };
        // Decoded as a protocol 10 client would: it must decode at all.
        let ServerFrame::Response { outcome, .. } =
            serde_json::from_str::<ServerFrame>(text.as_str()).unwrap()
        else {
            panic!("a response: {text}");
        };
        outcomes.push(outcome);
    }
    outcomes
}

fn refused(outcome: &Outcome) -> bool {
    matches!(outcome, Outcome::Err(error) if error.code == ErrorCode::Unsupported)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_older_client_is_never_handed_an_added_agent() {
    within(async {
        let fixture = Fixture::new();
        let node = quick(&fixture).await;
        let owner = fixture.connect(&node).await;
        let env_file = fixture.cwd().join("env.txt");
        let _added = owner.call(add("fake", &env_file)).await.unwrap();
        set_secret(&owner, "x").await;
        let fake = HarnessRef::Added(name("fake"));
        let added = owner
            .create_session(structured_of(&fixture, fake))
            .await
            .unwrap();
        let known = owner
            .create_session(structured_of(&fixture, HarnessKind::ClaudeCode.into()))
            .await
            .unwrap();

        let outcomes = as_version_10(
            &fixture,
            &node,
            vec![
                ListSessions::default().into_call(),
                ListHarnesses.into_call(),
                vessel_proto::GetSession { session: added.id }.into_call(),
                KillSession { session: added.id }.into_call(),
                vessel_proto::GetSession { session: known.id }.into_call(),
            ],
        )
        .await;
        let [listed, harnesses, get, kill, get_known] = outcomes.as_slice() else {
            panic!("five answers: {outcomes:?}");
        };
        let Outcome::Ok(Reply::ListSessions(sessions)) = listed else {
            panic!("the list: {listed:?}");
        };
        let ids: Vec<SessionId> = sessions.iter().map(|session| session.id).collect();
        assert!(
            ids.contains(&known.id) && !ids.contains(&added.id),
            "{ids:?}"
        );
        let Outcome::Ok(Reply::ListHarnesses(found)) = harnesses else {
            panic!("the harnesses: {harnesses:?}");
        };
        assert!(found.iter().all(|harness| harness.kind.added().is_none()));
        assert!(refused(get), "{get:?}");
        assert!(refused(kill), "{kill:?}");
        assert!(matches!(get_known, Outcome::Ok(Reply::GetSession(_))));
        // The refused kill changed nothing.
        assert!(owner.session(added.id).await.unwrap().status.is_live());
        owner.kill(added.id).await.unwrap();
        owner.kill(known.id).await.unwrap();
        node.stop().await.unwrap();
    })
    .await;
}
