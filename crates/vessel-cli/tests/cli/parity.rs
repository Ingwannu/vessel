//! Command parity is checked through the actual executable and a fake ACP agent.
use crate::support::{Fixture, within};
use serde::Deserialize;
use vessel_proto::{ContentBlock, Delivery, SessionId, SessionInfo, SessionMode, SessionSpec};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Waited {
    agent_id: SessionId,
    status: String,
}
#[derive(Debug, Deserialize)]
struct Listed {
    id: SessionId,
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn agent_shortcuts_wait_cancel_without_killing_and_protect_live_archive() {
    within(async {
        let fixture = Fixture::start().await;
        let cwd = fixture.cwd();
        let ran = fixture
            .vessel(
                &[
                    "agent",
                    "run",
                    "hello",
                    "--provider",
                    "claude",
                    "--cwd",
                    &cwd,
                    "--json",
                ],
                b"",
            )
            .await;
        assert!(ran.ok, "{}", ran.stderr);
        let waited: Waited = serde_json::from_str(&ran.stdout).unwrap();
        assert_eq!(waited.status, "idle");
        let id = waited.agent_id.to_string();
        let owner = fixture.owner().await;
        let before = owner.session(waited.agent_id).await.unwrap();
        assert!(before.status.is_live());
        let stopped = fixture.vessel(&["stop", &id, "--json"], b"").await;
        assert!(stopped.ok, "{}", stopped.stderr);
        assert!(
            owner
                .session(waited.agent_id)
                .await
                .unwrap()
                .status
                .is_live()
        );
        owner
            .prompt(
                waited.agent_id,
                vec![ContentBlock::Text {
                    text: "wait".to_owned(),
                }],
                Delivery::Start,
            )
            .await
            .unwrap();
        let wait = fixture
            .vessel(&["wait", &id, "--timeout", "1s", "--json"], b"")
            .await;
        assert!(wait.ok, "{}", wait.stderr);
        assert_eq!(
            serde_json::from_str::<Waited>(&wait.stdout).unwrap().status,
            "timeout"
        );
        let protected = fixture.vessel(&["archive", &id], b"").await;
        assert!(!protected.ok);
        assert!(
            owner
                .session(waited.agent_id)
                .await
                .unwrap()
                .marks
                .archived_at
                .is_none()
        );
        let stop = fixture.vessel(&["agent", "stop", &id], b"").await;
        assert!(stop.ok, "{}", stop.stderr);
        let wait = fixture
            .vessel(&["agent", "wait", &id, "--timeout", "5s", "--json"], b"")
            .await;
        assert!(wait.ok, "{}", wait.stderr);
        assert_eq!(
            serde_json::from_str::<Waited>(&wait.stdout).unwrap().status,
            "idle"
        );
        assert!(
            owner
                .session(waited.agent_id)
                .await
                .unwrap()
                .status
                .is_live()
        );
        let archived = fixture
            .vessel(&["archive", &id, "--force", "--json"], b"")
            .await;
        assert!(archived.ok, "{}", archived.stderr);
        assert!(
            owner
                .session(waited.agent_id)
                .await
                .unwrap()
                .marks
                .archived_at
                .is_some()
        );
        fixture.stop().await;
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_defaults_to_cwd_and_structured_refusals_have_no_creation_side_effect() {
    within(async {
        let fixture = Fixture::start().await;
        let owner = fixture.owner().await;
        let cwd = fixture.cwd();
        let here = owner
            .create_session(
                SessionSpec::new(
                    vessel_proto::HarnessKind::ClaudeCode,
                    SessionMode::Structured {
                        model: None,
                        resume: None,
                        options: Vec::new(),
                    },
                )
                .with_cwd(&cwd),
            )
            .await
            .unwrap();
        let other_dir = fixture.data().join("other");
        std::fs::create_dir(&other_dir).unwrap();
        let other = owner
            .create_session(
                SessionSpec::new(
                    vessel_proto::HarnessKind::ClaudeCode,
                    SessionMode::Structured {
                        model: None,
                        resume: None,
                        options: Vec::new(),
                    },
                )
                .with_cwd(&other_dir),
            )
            .await
            .unwrap();
        let listed = fixture.vessel(&["ls", "--json"], b"").await;
        assert!(listed.ok, "{}", listed.stderr);
        let entries: Vec<Listed> = serde_json::from_str(&listed.stdout).unwrap();
        assert_eq!(entries.iter().map(|e| e.id).collect::<Vec<_>>(), [here.id]);
        let listed = fixture
            .vessel(&["agent", "ls", "--global", "--json"], b"")
            .await;
        assert!(listed.ok, "{}", listed.stderr);
        let entries: Vec<Listed> = serde_json::from_str(&listed.stdout).unwrap();
        assert!(entries.iter().any(|e| e.id == other.id));
        let before = owner.sessions().await.unwrap().len();
        let refused = fixture
            .vessel(
                &[
                    "agent",
                    "run",
                    "hello",
                    "--provider",
                    "claude",
                    "--env",
                    "TEST=value",
                ],
                b"",
            )
            .await;
        assert!(!refused.ok);
        assert_eq!(owner.sessions().await.unwrap().len(), before);
        let inspected = fixture
            .vessel(&["inspect", &here.id.to_string(), "--json"], b"")
            .await;
        assert!(inspected.ok, "{}", inspected.stderr);
        let info: SessionInfo = serde_json::from_str(&inspected.stdout).unwrap();
        assert_eq!(info.id, here.id);
        fixture.stop().await;
    })
    .await;
}

#[derive(Deserialize)]
struct FailureEnvelope {
    error: Failure,
}
#[derive(Deserialize)]
struct Failure {
    code: vessel_proto::ErrorCode,
    message: String,
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pending_contracts_refuse_before_network_and_leave_structured_stdout_empty() {
    within(async {
        let fixture = Fixture::start().await;
        for args in [
            vec!["workspace", "create", "--isolation", "local", "--json"],
            vec!["provider", "ls", "--json"],
            vec!["usage", "inspect", "quota-id", "--json"],
            vec!["terminal", "create", "--json"],
            vec!["host", "ls", "--json"],
            vec![
                "agent",
                "run",
                "prompt",
                "--provider",
                "claude",
                "--env",
                "TEST=private-fixture-value",
                "--json",
            ],
        ] {
            // A connection attempt would produce unavailable, not this typed refusal.
            let ran = fixture
                .vessel_at("127.0.0.1:1", fixture.data(), &args, b"")
                .await;
            assert!(!ran.ok);
            assert!(ran.stdout.is_empty(), "{}", ran.stdout);
            let error: FailureEnvelope = serde_json::from_str(&ran.stderr).unwrap();
            assert_eq!(error.error.code, vessel_proto::ErrorCode::Unsupported);
            assert!(!error.error.message.contains("private-fixture-value"));
        }
        fixture.stop().await;
    })
    .await;
}
