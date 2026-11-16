//! Normal product binary: canonical pending reads, shared formats, no response edge.
use crate::support::{Fixture, within};
use serde::Deserialize;
use vessel_proto::{
    AgentEvent, ContentBlock, EventSeq, HarnessKind, SessionEvent, SessionId, SessionInput,
    SessionMode, SessionSpec,
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Listed {
    id: String,
    agent_id: String,
    name: String,
}

async fn pending(fixture: &Fixture, prompt: &str) -> (SessionId, String) {
    let owner = fixture.owner().await;
    let session = owner
        .create_session(
            SessionSpec::new(
                HarnessKind::ClaudeCode,
                SessionMode::Structured {
                    model: None,
                    resume: None,
                    options: Vec::new(),
                },
            )
            .with_cwd(fixture.data().to_path_buf()),
        )
        .await
        .unwrap()
        .id;
    let mut stream = owner.follow(session, EventSeq::ZERO).await.unwrap();
    owner
        .send(
            session,
            SessionInput::Prompt {
                content: vec![ContentBlock::Text {
                    text: prompt.to_owned(),
                }],
            },
        )
        .await
        .unwrap();
    let id = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            let record = stream.next().await.unwrap().unwrap();
            if let SessionEvent::Agent { event } = record.event {
                match event {
                    AgentEvent::QuestionAsked(question) => break question.id.to_string(),
                    AgentEvent::PermissionRequested(request) => break request.id.to_string(),
                    AgentEvent::TurnEnded { .. } => panic!("fake turn ended before a request"),
                    _ => {}
                }
            }
        }
    })
    .await
    .unwrap();
    (session, id)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn permit_ls_process_reads_actual_forms_in_every_shared_format_without_resolving_them() {
    within(async {
        let fixture = Fixture::start().await;
        let (permission_session, permission) = pending(&fixture, "build").await;
        let (question_session, question) = pending(&fixture, "ask").await;
        let json = fixture.vessel(&["permit", "ls", "--json"], b"").await;
        assert!(json.ok, "{}", json.stderr);
        let rows: Vec<Listed> = serde_json::from_str(&json.stdout).unwrap();
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().any(|row| row.id == permission
            && row.agent_id == permission_session.to_string()
            && row.name == "Permission"));
        assert!(rows.iter().any(|row| row.id == question
            && row.agent_id == question_session.to_string()
            && row.name == "Question"));
        for flags in [
            vec![],
            vec!["--no-headers"],
            vec!["--quiet"],
            vec!["-o", "yaml"],
        ] {
            let mut args = vec!["permit", "ls"];
            args.extend(flags.iter().copied());
            let result = fixture.vessel(&args, b"").await;
            assert!(result.ok, "{}", result.stderr);
            if flags == ["--quiet"] || flags == ["-o", "yaml"] {
                assert!(result.stdout.contains(&permission));
                assert!(result.stdout.contains(&question));
            } else {
                assert!(result.stdout.contains("Permission"));
                assert!(result.stdout.contains("Question"));
                assert_eq!(result.stdout.contains("REQ_ID"), flags.is_empty());
            }
        }
        for name in ["allow", "deny"] {
            let result = fixture
                .vessel(
                    &["permit", name, &permission_session.to_string(), "--all"],
                    b"",
                )
                .await;
            assert!(!result.ok);
            assert!(result.stdout.is_empty());
            assert!(
                result.stderr.contains("unrecognized subcommand"),
                "{}",
                result.stderr
            );
        }
        let owner = fixture.owner().await;
        assert_eq!(
            owner
                .conversation(permission_session)
                .await
                .unwrap()
                .permissions[0]
                .id
                .to_string(),
            permission
        );
        assert_eq!(
            owner
                .conversation(question_session)
                .await
                .unwrap()
                .questions[0]
                .id
                .to_string(),
            question
        );
        fixture.stop().await;
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn permit_ls_empty_and_help_are_real_product_commands() {
    within(async {
        let fixture = Fixture::start().await;
        let empty = fixture.vessel(&["permit", "ls", "--json"], b"").await;
        assert!(empty.ok, "{}", empty.stderr);
        let rows: Vec<Listed> = serde_json::from_str(&empty.stdout).unwrap();
        assert!(rows.is_empty());
        let help = fixture.vessel(&["permit", "--help"], b"").await;
        assert!(help.ok, "{}", help.stderr);
        assert!(help.stdout.contains("ls"));
        assert!(!help.stdout.contains("allow"));
        assert!(!help.stdout.contains("deny"));
        fixture.stop().await;
    })
    .await;
}
