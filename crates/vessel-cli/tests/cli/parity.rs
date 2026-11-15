//! Command parity is checked through the actual executable and a fake ACP agent.
use crate::support::{Fixture, within};
use serde::Deserialize;
use vessel_proto::{ContentBlock, Delivery, SessionId, SessionMode, SessionSpec};

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
#[derive(Deserialize)]
struct Inspected {
    #[serde(rename = "Id")]
    id: SessionId,
    #[serde(rename = "Cwd")]
    cwd: String,
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn project_creation_consumes_table_headers_json_and_quiet_on_actual_acknowledgements() {
    within(async {
        let fixture = Fixture::start().await;
        let owner = fixture.owner().await;
        for (name, flags) in [
            ("table", vec![]),
            ("headerless", vec!["--no-headers"]),
            ("json", vec!["--json"]),
            ("quiet", vec!["--quiet"]),
        ] {
            let path = fixture.data().join(name);
            std::fs::create_dir(&path).unwrap();
            let mut args = vec!["project", "create", path.to_str().unwrap()];
            args.extend(flags);
            let result = fixture.vessel(&args, b"").await;
            assert!(result.ok, "{}", result.stderr);
            let projects = owner
                .call(vessel_proto::workspace::ListProjects {})
                .await
                .unwrap();
            let created = projects
                .projects
                .iter()
                .find(|project| project.root == path)
                .unwrap();
            match name {
                "table" => {
                    assert!(result.stdout.starts_with("PROJECT ID  NAME  KIND  PATH\n"));
                    assert!(result.stdout.contains(path.to_str().unwrap()));
                }
                "headerless" => {
                    assert!(result.stdout.starts_with(&created.id.to_string()));
                    assert!(!result.stdout.contains("PROJECT ID"));
                }
                "json" => {
                    #[derive(Deserialize)]
                    #[serde(rename_all = "camelCase")]
                    struct Created {
                        project_id: vessel_proto::ProjectId,
                        path: std::path::PathBuf,
                    }
                    let decoded: Created = serde_json::from_str(&result.stdout).unwrap();
                    assert_eq!(decoded.project_id, created.id);
                    assert_eq!(decoded.path, path);
                }
                "quiet" => assert_eq!(result.stdout, format!("{}\n", created.id)),
                _ => unreachable!(),
            }
        }
        fixture.stop().await;
    })
    .await;
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
                    "plain",
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
        assert_eq!(waited.status, "completed");
        let id = waited.agent_id.to_string();
        let owner = fixture.owner().await;
        let before = owner.session(waited.agent_id).await.unwrap();
        assert!(before.status.is_live());
        let stopped = fixture.vessel(&["stop", &id, "--json"], b"").await;
        assert!(stopped.ok, "{}", stopped.stderr);
        let no_ids = fixture.vessel(&["stop", &id, "--quiet"], b"").await;
        assert!(no_ids.ok, "{}", no_ids.stderr);
        assert!(no_ids.stdout.is_empty(), "{}", no_ids.stdout);
        let count = fixture.vessel(&["stop", &id, "--no-headers"], b"").await;
        assert!(count.ok, "{}", count.stderr);
        assert_eq!(count.stdout, "0\n");
        assert!(
            owner
                .session(waited.agent_id)
                .await
                .unwrap()
                .status
                .is_live()
        );
        let _delivered = owner
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
        let info: Inspected = serde_json::from_str(&inspected.stdout).unwrap();
        assert_eq!(info.id, here.id);
        assert_eq!(info.cwd, cwd);
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn image_send_reuses_chunk_upload_and_delivers_the_acknowledged_attachment() {
    within(async {
        let fixture = Fixture::start().await;
        let owner = fixture.owner().await;
        let info = owner
            .create_session(
                SessionSpec::new(
                    vessel_proto::HarnessKind::ClaudeCode,
                    SessionMode::Structured {
                        model: None,
                        resume: None,
                        options: Vec::new(),
                    },
                )
                .with_cwd(fixture.cwd()),
            )
            .await
            .unwrap();
        let _options = owner.session_options(info.id).await.unwrap();
        let path = fixture.data().join("image.png");
        let mut bytes = b"\x89PNG\r\n\x1a\n".to_vec();
        bytes.extend_from_slice(&[0_u8; 8]);
        std::fs::write(&path, bytes).unwrap();
        let before = owner.session(info.id).await.unwrap().next_seq;
        let sent = fixture
            .vessel(
                &[
                    "agent",
                    "send",
                    &info.id.to_string(),
                    "--prompt",
                    "describe",
                    "--image",
                    &path.display().to_string(),
                    "--json",
                ],
                b"",
            )
            .await;
        assert!(sent.ok, "{}", sent.stderr);
        let waited: Waited = serde_json::from_str(&sent.stdout).unwrap();
        assert_eq!(waited.status, "completed");
        let mut stream = owner.follow(info.id, before).await.unwrap();
        let mut said = String::new();
        while let Some(record) = stream.next().await {
            if let vessel_proto::SessionEvent::Agent { event } = record.unwrap().event {
                if let vessel_proto::AgentEvent::MessageChunk {
                    content: ContentBlock::Text { text },
                } = &event
                {
                    said.push_str(text);
                }
                if matches!(event, vessel_proto::AgentEvent::TurnEnded { .. }) {
                    break;
                }
            }
        }
        assert!(said.contains("image image/png 16 bytes"), "{said}");
        fixture.stop().await;
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn selected_loopback_destination_never_borrows_cli_project_cwd() {
    within(async {
        let fixture = Fixture::start().await;
        let repo = fixture.data().join("selected-destination-repo");
        std::fs::create_dir(&repo).unwrap();
        let initialized = tokio::process::Command::new("git")
            .arg("init")
            .arg(&repo)
            .output()
            .await
            .unwrap();
        assert!(initialized.status.success());
        let url = format!("ws://{}", fixture.node_address());
        let token = fixture.data().join("token");
        let added = fixture
            .vessel_book(
                &repo,
                &[
                    "nodes",
                    "add",
                    "selected",
                    &url,
                    "--token-file",
                    token.to_str().unwrap(),
                    "--default",
                ],
                &[],
            )
            .await;
        assert!(added.ok, "{}", added.stderr);
        let owner = fixture.owner().await;
        let before = owner
            .call(vessel_proto::workspace::ListProjects {})
            .await
            .unwrap();
        assert!(before.projects.is_empty());
        for args in [
            vec!["project", "create", "--json"],
            vec!["project", "create", ".", "--json"],
        ] {
            let refused = fixture.vessel_book(&repo, &args, &[]).await;
            assert!(!refused.ok);
            assert!(refused.stdout.is_empty());
            assert!(
                refused.stderr.contains("absolute path")
                    || refused.stderr.contains("must be absolute")
            );
        }
        let after = owner
            .call(vessel_proto::workspace::ListProjects {})
            .await
            .unwrap();
        assert!(after.projects.is_empty());
        let explicit = fixture
            .vessel_book(
                &repo,
                &["project", "create", repo.to_str().unwrap(), "--json"],
                &[],
            )
            .await;
        assert!(explicit.ok, "{}", explicit.stderr);
        let after = owner
            .call(vessel_proto::workspace::ListProjects {})
            .await
            .unwrap();
        assert_eq!(after.projects.len(), 1);
        fixture.stop().await;
    })
    .await;
}
