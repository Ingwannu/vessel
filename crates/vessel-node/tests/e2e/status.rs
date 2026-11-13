//! `agent.status` end to end (protocol 11, ADR 0032): installed and
//! signed-in facts of added agents (the testkit's fake ACP agent), found
//! without a turn, and only when a person asks.

use vessel_proto::agents::{
    AddAgent, AgentDefinition, AgentReadiness, AgentToAdd, AgentVariable, GetAgentStatus,
    Installed, SignedIn, UnknownReason,
};
use vessel_proto::{AgentName, ConfigValue, ErrorCode, HarnessKind, HarnessRef};
use vessel_testkit::Fake;

use crate::agents::{as_agent, code, credential, structured, vessel_mcp};
use crate::support::{Fixture, within};

fn name(text: &str) -> AgentName {
    text.parse().unwrap()
}

/// The fake ACP agent by its absolute path (or `program`), with `env`.
fn add(agent: &str, program: Option<&std::path::Path>, env: &[(&str, &str)]) -> AddAgent {
    let fake = Fake::Acp.path().unwrap();
    let program = program.unwrap_or(&fake);
    AddAgent {
        name: name(agent),
        agent: AgentToAdd::Command {
            definition: AgentDefinition {
                label: None,
                program: program.to_str().unwrap().parse().unwrap(),
                args: Vec::new(),
                env: env
                    .iter()
                    .map(|(variable, value)| AgentVariable {
                        name: variable.parse().unwrap(),
                        value: ConfigValue::Text((*value).to_owned()),
                    })
                    .collect(),
                inherit: Vec::new(),
                mcp: Vec::new(),
            },
        },
        replace: false,
    }
}

fn of(listed: &[AgentReadiness], agent: &str) -> AgentReadiness {
    let agent = HarnessRef::Added(name(agent));
    listed
        .iter()
        .find(|readiness| readiness.agent == agent)
        .cloned()
        .unwrap()
}

fn status(agent: Option<&str>, refresh: bool) -> GetAgentStatus {
    GetAgentStatus {
        agent: agent.map(|agent| HarnessRef::Added(name(agent))),
        profile: None,
        refresh,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn status_is_found_without_a_turn_and_only_when_asked() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture.start().await;
        let owner = fixture.connect(&node).await;
        let missing = fixture.cwd().join("missing-acp");
        for request in [
            add("asks", None, &[("FAKE_ACP_SIGN_IN", "1")]),
            add("plain", None, &[]),
            add("gone", Some(&missing), &[]),
        ] {
            let _added = owner.call(request).await.unwrap();
        }

        // Without `refresh`, nothing runs: installed or not, never signed in.
        let listed = owner.call(status(None, false)).await.unwrap();
        let asks = of(&listed, "asks");
        assert_eq!(asks.profile.as_str(), "asks");
        assert!(
            matches!(asks.installed, Installed::Found { version: None, .. }),
            "{asks:?}"
        );
        assert_eq!(
            asks.signed_in,
            SignedIn::Unknown {
                reason: UnknownReason::NotChecked
            }
        );
        let gone = of(&listed, "gone");
        assert!(
            matches!(gone.installed, Installed::Missing { .. }),
            "{gone:?}"
        );
        assert_eq!(
            gone.signed_in,
            SignedIn::Unknown {
                reason: UnknownReason::NotInstalled
            }
        );
        // The four are there too, each under its default profile.
        assert!(
            listed
                .iter()
                .any(|readiness| readiness.agent == HarnessKind::Codex)
        );

        // A refresh names its agent.
        assert_eq!(
            code(owner.call(status(None, true)).await),
            ErrorCode::InvalidRequest
        );
        assert_eq!(
            code(owner.call(status(Some("nobody"), true)).await),
            ErrorCode::NotFound
        );

        // `auth_required` in the handshake, with the methods it offered.
        let asks = owner.call(status(Some("asks"), true)).await.unwrap();
        let [asks] = asks.as_slice() else {
            panic!("one profile: {asks:?}");
        };
        let SignedIn::No { help } = &asks.signed_in else {
            panic!("not signed in: {asks:?}");
        };
        assert_eq!(help.methods.len(), 1);
        assert_eq!(help.methods[0].id.as_str(), "fake-login");
        assert!(help.command.is_none());
        let Installed::Found {
            version: Some(version),
            ..
        } = &asks.installed
        else {
            panic!("its version from its handshake: {asks:?}");
        };
        assert_eq!(version.as_str(), env!("CARGO_PKG_VERSION"));

        // A handshake that asks nothing says nothing of a sign-in.
        let plain = owner.call(status(Some("plain"), true)).await.unwrap();
        assert_eq!(
            plain[0].signed_in,
            SignedIn::Unknown {
                reason: UnknownReason::NotSaid
            }
        );

        // What a refresh found is kept for the next reader.
        let listed = owner.call(status(None, false)).await.unwrap();
        assert!(matches!(of(&listed, "asks").signed_in, SignedIn::No { .. }));
        node.stop().await.unwrap();
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_status_command_is_read_and_not_run_beside_a_session() {
    within(async {
        let fixture = Fixture::new();
        // Two more accounts, one signed out (the fake's status command
        // says so).
        std::fs::write(
            fixture.data_dir().profiles_path(),
            "[profiles.work]\nharness = \"claude\"\nenv = { FAKE_TUI_LOGGED_OUT = \"1\" }\n\n\
             [profiles.other]\nharness = \"claude\"\nenv = { FAKE_OTHER = \"1\" }\n",
        )
        .unwrap();
        let node = fixture
            .start_with(|config| config.vessel_mcp = Some(vessel_mcp(&fixture)))
            .await;
        let owner = fixture.connect(&node).await;
        let claude = |profile: Option<&str>, refresh| GetAgentStatus {
            agent: Some(HarnessKind::ClaudeCode.into()),
            profile: profile.map(|profile| profile.parse().unwrap()),
            refresh,
        };

        // A session of the default profile runs: its check could race it
        // for the credentials, so none runs.
        let mut running = structured(&owner, &fixture).await;
        let checked = owner.call(claude(None, true)).await.unwrap();
        assert_eq!(
            checked[0].signed_in,
            SignedIn::Unknown {
                reason: UnknownReason::InUse
            },
            "{checked:?}"
        );

        // Claude Code's own command, read by its JSON.
        let other = owner.call(claude(Some("other"), true)).await.unwrap();
        assert_eq!(other[0].signed_in, SignedIn::Yes, "{other:?}");
        let work = owner.call(claude(Some("work"), true)).await.unwrap();
        let SignedIn::No { help } = &work[0].signed_in else {
            panic!("signed out: {work:?}");
        };
        assert_eq!(help.command.as_deref(), Some("claude auth login"));
        assert_eq!(
            help.home.as_ref().map(vessel_proto::EnvName::as_str),
            Some("CLAUDE_CONFIG_DIR")
        );
        // A profile of another agent is not one of its.
        assert_eq!(
            code(
                owner
                    .call(GetAgentStatus {
                        agent: Some(HarnessKind::Codex.into()),
                        profile: Some("work".parse().unwrap()),
                        refresh: false,
                    })
                    .await
            ),
            ErrorCode::InvalidRequest
        );

        // An agent principal reads, and does not run checks.
        let (_program, socket, auth) = credential(&owner, &mut running).await;
        let agent = as_agent(&socket, &auth).await;
        assert!(agent.call(claude(None, false)).await.is_ok());
        assert_eq!(
            code(agent.call(claude(Some("work"), true)).await),
            ErrorCode::Forbidden
        );
        node.stop().await.unwrap();
    })
    .await;
}
