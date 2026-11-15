//! Real delegated reads and local wait quotas, without an ideal long-poll mock.
mod approval;
#[cfg(unix)]
mod local_setup;
mod readiness;
use super::faults::bridge::Bridge;
use super::*;
use crate::support::{read_until, step, within};
use std::{path::Path, time::Duration};
use vessel_proto::workspace::{
    ApproveCommands, GetProjectCommands, ProjectCommands, ReadSetupOutput, SetupState,
};
use vessel_proto::{AgentEvent, EventRecord, Scope, SessionEvent};

fn with_git(config: &mut vessel_node::Config) {
    let git = which::which("git").unwrap();
    config.harness_lookup = vessel_harness::Lookup::new(
        config
            .harness_lookup
            .dirs()
            .iter()
            .cloned()
            .chain([git.parent().unwrap().to_owned()]),
    );
}
pub(super) fn commit(folder: &Path, file: &str) {
    for args in [
        vec!["add", file],
        vec![
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "-q",
            "-m",
            "comparison fixture",
        ],
    ] {
        assert!(
            std::process::Command::new("git")
                .args(args)
                .current_dir(folder)
                .status()
                .unwrap()
                .success()
        );
    }
}
pub(super) async fn approve(client: &NodeClient, project: ProjectId, approved: bool) {
    let commands = client
        .call(GetProjectCommands {
            project,
            base: None,
        })
        .await
        .unwrap();
    let ProjectCommands::Listed { digest, .. } = commands else {
        panic!("listed");
    };
    let _approved = client
        .call(ApproveCommands {
            project,
            digest,
            approved,
        })
        .await
        .unwrap();
}
/// Wait until `session`'s turn has ended (fake "plain" ends normally).
/// Seeing its `TurnStarted` live is not enough: the publisher broadcasts a
/// record before it moves the readable end past it, so only a later record
/// guarantees that a read holds the turn's start (N5's note, as in V1's
/// 9d3a30c6 review).
async fn turn_ended(owner: &NodeClient, session: SessionId) {
    let mut stream = owner.follow(session, EventSeq::ZERO).await.unwrap();
    step(
        "the turn's end",
        read_until(&mut stream, &mut Vec::<EventRecord>::new(), |record| {
            matches!(
                &record.event,
                SessionEvent::Agent {
                    event: AgentEvent::TurnEnded { .. }
                }
            )
        }),
    )
    .await;
}

async fn finish(
    job: &ComparisonJob,
    owner: &NodeClient,
    index: vessel_client::comparison::SelectionIndex,
) {
    // One timer deliberately exercises the helper's >=1 s eligibility, not
    // coordination: lifecycle completion is observed from actual node records.
    for _ in 0..3 {
        let target = job.snapshot().targets[index.get()].clone();
        match target.status {
            TargetStatus::WaitingSetup => {
                let worktree = target.resources.worktree.unwrap();
                let mut cursor = 0;
                loop {
                    let output = owner
                        .call(ReadSetupOutput {
                            worktree,
                            from: cursor,
                            wait: Some(30),
                        })
                        .await
                        .unwrap();
                    cursor = output.next;
                    if matches!(
                        output.setup,
                        SetupState::None | SetupState::Succeeded { .. }
                    ) {
                        break;
                    }
                    assert!(matches!(output.setup, SetupState::Running { .. }));
                }
            }
            TargetStatus::WaitingReady => {
                let mut stream = owner
                    .follow(target.resources.session.unwrap(), EventSeq::ZERO)
                    .await
                    .unwrap();
                read_until(&mut stream, &mut Vec::<EventRecord>::new(), |record| {
                    matches!(
                        &record.event,
                        SessionEvent::Agent {
                            event: AgentEvent::Ready { .. }
                        }
                    )
                })
                .await;
            }
            TargetStatus::Ready | TargetStatus::Started => return,
            other @ (TargetStatus::Planned
            | TargetStatus::InFlight
            | TargetStatus::NeedsApproval(_)
            | TargetStatus::Failed(_)
            | TargetStatus::Uncertain(_)
            | TargetStatus::Cancelled) => panic!("cannot finish {other:?}"),
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
        let _continued = job.continue_target(index).await.unwrap();
    }
    assert!(matches!(
        job.snapshot().targets[index.get()].status,
        TargetStatus::Ready | TargetStatus::Started
    ));
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_member_keeps_mixed_pending_setup_and_frozen_input_after_partial_start() {
    within(async {
        let relay = crate::secure::Relay::start().await;
        let sources = tempfile::tempdir().unwrap();
        let home = crate::across::home(&relay, sources.path()).await;
        let member = crate::across::member_custom(
            &relay,
            &home,
            "comparison member",
            vec![Scope::Sync],
            vessel_node::fleet::DEFAULT_GRANT.to_vec(),
            with_git,
        )
        .await;
        let (folder, project) = crate::tidying::project(&member.owner).await;
        let gate = tempfile::tempdir().unwrap();
        let fifo = gate.path().join("release");
        assert!(
            std::process::Command::new("mkfifo")
                .arg(&fifo)
                .status()
                .unwrap()
                .success()
        );
        let command = format!(
            "if mkdir '{}' 2>/dev/null; then :; else cat '{}'; fi",
            gate.path().join("first").display(),
            fifo.display()
        );
        let mut config = toml_edit::DocumentMut::new();
        config["worktree"]["setup"] = toml_edit::value(command);
        std::fs::write(folder.path().join("vessel.toml"), config.to_string()).unwrap();
        commit(folder.path(), "vessel.toml");
        approve(&member.owner, project.id, true).await;
        let bridge = Bridge::open(&home.fixture, &home.node).await;
        let job = bridge
            .client
            .comparison(member.id(), plan(project.id, &["fake-small", "fake-large"]));
        let _prepared = job.prepare().await.unwrap();
        let first = job.snapshot().targets[0].index;
        let second = job.snapshot().targets[1].index;
        assert!(matches!(
            job.snapshot().targets[1].status,
            TargetStatus::WaitingSetup
        ));
        let reads = bridge.count(vessel_proto::Method::SetupOutput).await;
        let _continued = job.continue_target(second).await.unwrap();
        assert_eq!(bridge.count(vessel_proto::Method::SetupOutput).await, reads);
        finish(&job, &member.owner, first).await;
        let source = job.snapshot().targets[0].resources.session.unwrap();
        let file = attached(
            &member.owner,
            source,
            Bytes::from_static(b"one frozen snapshot"),
        )
        .await;
        let sent = job
            .start(input("plain", Some((source, file.clone()))))
            .await
            .unwrap();
        assert!(matches!(sent.targets[0].status, TargetStatus::Started));
        assert!(matches!(sent.targets[1].status, TargetStatus::WaitingSetup));
        tokio::fs::write(&fifo, b"go\n").await.unwrap();
        finish(&job, &member.owner, second).await;
        let done = job.snapshot();
        assert!(
            matches!(done.targets[1].status, TargetStatus::Started),
            "{done:?}"
        );
        assert_eq!(turns(&member.owner, source).await.len(), 1);
        let destination = done.targets[1].resources.session.unwrap();
        // The destination's turn is recorded after its prompt is accepted,
        // which can come after the job reports Started (W15's run,
        // 2026-11-15): wait for its end, then read (`turn_ended`).
        turn_ended(&member.owner, destination).await;
        let history = turns(&member.owner, destination).await;
        let vessel_proto::ContentBlock::Attachment { attachment } = &history[0][1] else {
            panic!("file");
        };
        assert_ne!(attachment.id, file.id);
        assert_eq!(
            member
                .owner
                .read_attachment(destination, attachment.id, 0)
                .await
                .unwrap()
                .data,
            Bytes::from_static(b"one frozen snapshot")
        );
        assert_eq!(bridge.count(vessel_proto::Method::CreateWorktree).await, 2);
        assert_eq!(bridge.count(vessel_proto::Method::CreateSession).await, 2);
        assert_eq!(bridge.count(vessel_proto::Method::PromptSession).await, 2);
        bridge.stop().await;
        member.node.stop().await.unwrap();
        home.node.stop().await.unwrap();
        relay.stop().await;
    })
    .await;
}
