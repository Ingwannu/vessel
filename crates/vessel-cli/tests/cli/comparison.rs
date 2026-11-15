//! Real binary comparison: bounded manifest, shared upload and complete partial reports.
use super::support::{Fixture, within};
use bytes::Bytes;
use serde::Deserialize;
use vessel_proto::{AgentEvent, AttachmentRef, ContentBlock, EventSeq, SessionEvent, SessionId};

#[derive(Deserialize)]
struct Report {
    comparison: Compared,
    source_upload: Source,
}
#[derive(Deserialize)]
struct Compared {
    targets: Vec<Target>,
}
#[derive(Deserialize)]
struct Target {
    status: Status,
    resources: Resources,
}
#[derive(Deserialize)]
struct Status {
    state: State,
}
#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum State {
    Started,
    Failed,
}
#[derive(Deserialize)]
struct Resources {
    session: Option<SessionId>,
}
#[derive(Deserialize)]
struct Source {
    session: Option<SessionId>,
    finished: Vec<AttachmentRef>,
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn comparison_binary_uploads_once_and_prints_every_independent_outcome() {
    within(async {
        let fixture = Fixture::start_with(|config, _| {
            let git = which::which("git").unwrap();
            config.harness_lookup = vessel_harness::Lookup::new(config.harness_lookup.dirs().iter()
                .cloned().chain([git.parent().unwrap().to_owned()]));
        }).await;
        let owner = fixture.owner().await;
        let folder = tempfile::tempdir().unwrap();
        let project = owner.call(vessel_proto::workspace::InitProject {
            path: folder.path().join("project"), name: None }).await.unwrap();
        let readme = project.root.join("README.md");
        std::fs::write(&readme, "comparison fixture\n").unwrap();
        for args in [vec!["symbolic-ref", "HEAD", "refs/heads/main"], vec!["add", "README.md"], vec!["-c", "user.name=Test", "-c", "user.email=test@example.invalid",
            "-c", "commit.gpgsign=false", "commit", "-q", "-m", "first"]] {
            assert!(std::process::Command::new("git").args(args).current_dir(&project.root).status().unwrap().success());
        }
        let manifest = folder.path().join("targets.json");
        let file = folder.path().join("common.txt");
        std::fs::write(&manifest, r#"[{"harness":"claude-code","model":"fake-small"},{"harness":"claude-code","model":"fake-large"}]"#).unwrap();
        std::fs::write(&file, b"one common file").unwrap();
        let project_id = project.id.to_string();
        let base = "main";
        let args = ["session", "compare", "--project", &project_id, "--base", base,
            "--targets", manifest.to_str().unwrap(), "plain", "--attach", file.to_str().unwrap(), "--json"];
        let sent = fixture.vessel(&args, b"").await;
        assert!(sent.ok, "{}\n{}", sent.stderr, sent.stdout);
        let report: Report = serde_json::from_str(&sent.stdout).unwrap();
        assert_eq!(report.comparison.targets.len(), 2);
        assert_eq!(report.source_upload.finished.len(), 1);
        for target in report.comparison.targets {
            assert_eq!(target.status.state, State::Started);
            let session = target.resources.session.unwrap();
            let history = owner.read(session, EventSeq::ZERO, 1000).await.unwrap();
            let turns: Vec<_> = history.records.iter().filter_map(|record| if let SessionEvent::Agent {
                event: AgentEvent::TurnStarted { provenance: None, content ,
client_message_id: None,
} } = &record.event { Some(content) } else { None }).collect();
            assert_eq!(turns.len(), 1);
            let ContentBlock::Attachment { attachment } = &turns[0][1] else { panic!("file"); };
            if Some(session) != report.source_upload.session { assert_ne!(attachment.id, report.source_upload.finished[0].id); }
            assert_eq!(owner.read_attachment(session, attachment.id, 0).await.unwrap().data,
                Bytes::from_static(b"one common file"));
        }
        std::fs::write(&manifest, r#"[{"harness":"claude-code","model":"fake-small"},{"harness":"claude-code","model":"not-a-model"}]"#).unwrap();
        let partial = fixture.vessel(&args, b"").await;
        assert!(!partial.ok);
        let report: Report = serde_json::from_str(&partial.stdout).unwrap();
        assert_eq!(report.comparison.targets[0].status.state, State::Started);
        assert_eq!(report.comparison.targets[1].status.state, State::Failed);
        assert!(partial.stderr.contains("did not start every target"));
        fixture.stop().await;
    }).await;
}
