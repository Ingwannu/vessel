//! Cancellation while a real framed reply is outstanding must win locally.

use std::sync::{Arc, Mutex};

use super::*;
use crate::workspace::scripts::peer;
use vessel_proto::workspace::{SetupFailure, SetupOutput};
use vessel_proto::{Call, Creator, ProjectId, Reply as WireReply, TerminalBytes, WorktreeId};

#[tokio::test]
async fn closing_the_creation_channel_during_the_final_output_read_never_authorizes_a_session() {
    for setup in [
        SetupState::Succeeded {
            finished_at: chrono::DateTime::UNIX_EPOCH,
        },
        SetupState::Failed {
            failure: SetupFailure::Spawn,
            finished_at: chrono::DateTime::UNIX_EPOCH,
        },
        SetupState::Unknown,
    ] {
        let (answers, receiver) = mpsc::channel(1);
        let answers = Arc::new(Mutex::new(Some(answers)));
        let peer = peer::connect(move |call| {
            if matches!(call, Call::SetupOutput(_)) {
                drop(answers.lock().unwrap().take());
                Ok(WireReply::SetupOutput(SetupOutput {
                    from: 0,
                    bytes: TerminalBytes::default(),
                    next: 0,
                    truncated: false,
                    setup: setup.clone(),
                }))
            } else {
                panic!("cancelled creation sent another call: {call:?}");
            }
        })
        .await;
        let (events, _received) = mpsc::channel(8);
        let reply = Reply {
            machine: "fake".to_owned(),
            events,
        };
        let walk = Walk {
            client: &peer.client,
            reply: &reply,
            key: StartKey(1),
            answers: receiver,
        };
        let info = WorktreeInfo {
            id: WorktreeId::random(),
            project: ProjectId::random(),
            path: "/worktree".into(),
            branch: None,
            created_branch: None,
            base: None,
            created_at: chrono::DateTime::UNIX_EPOCH,
            created_by: Creator::Owner,
            created_for: None,
            sessions: Vec::new(),
            setup: SetupState::Running {
                started_at: chrono::DateTime::UNIX_EPOCH,
            },
            kept: None,
        };
        assert!(matches!(output(&walk, &info, &[]).await, Followed::Left));
        let calls = peer.calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert!(matches!(calls.first(), Some(Call::SetupOutput(_))));
    }
}
