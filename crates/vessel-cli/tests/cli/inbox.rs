//! Real CLI list/read/answer commands against a node and fake ACP agent.
use crate::support::{Fixture, within};
use vessel_proto::inbox::{InboxRead, ListInbox};
use vessel_proto::{
    AgentEvent, ContentBlock, EventSeq, HarnessKind, SessionEvent, SessionInput, SessionMode,
    SessionSpec,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn inbox_lists_marks_and_answers_the_actors_own_request() {
    within(async {
        let fixture = Fixture::start().await;
        let owner = fixture.owner().await;
        let id = owner
            .create_session(
                SessionSpec::new(
                    HarnessKind::ClaudeCode,
                    SessionMode::Structured {
                        model: None,
                        resume: None,
                        options: Vec::new(),
                    },
                )
                .with_cwd(std::path::PathBuf::from(fixture.cwd())),
            )
            .await
            .unwrap()
            .id;
        let mut stream = owner.follow(id, EventSeq::ZERO).await.unwrap();
        owner
            .send(
                id,
                SessionInput::Prompt {
                    content: vec![ContentBlock::Text {
                        text: "tool".to_owned(),
                    }],
                },
            )
            .await
            .unwrap();
        loop {
            let record = stream.next().await.unwrap().unwrap();
            if matches!(
                record.event,
                SessionEvent::Agent {
                    event: AgentEvent::PermissionRequested(_)
                }
            ) {
                break;
            }
        }
        let _committed = owner.read(id, EventSeq::ZERO, 1).await.unwrap();
        let entry = owner
            .inbox(ListInbox::default())
            .await
            .unwrap()
            .entries
            .remove(0);
        let cursor = entry.cursor.get().to_string();
        let listed = fixture.vessel(&["inbox", "list"], b"").await;
        assert!(listed.ok, "{}", listed.stderr);
        let title = listed
            .stdout
            .lines()
            .find(|line| line.starts_with("  Run"))
            .unwrap();
        assert_eq!(title, "  Run fake tool");
        let read = fixture.vessel(&["inbox", "read", &cursor], b"").await;
        assert!(read.ok, "{}", read.stderr);
        assert_eq!(
            owner.inbox(ListInbox::default()).await.unwrap().entries[0].read,
            InboxRead::Read
        );
        let unread = fixture.vessel(&["inbox", "unread", &cursor], b"").await;
        assert!(unread.ok, "{}", unread.stderr);
        assert_eq!(
            owner.inbox(ListInbox::default()).await.unwrap().entries[0].read,
            InboxRead::Unread
        );
        let invalid = fixture
            .vessel(&["inbox", "answer", &cursor, "not-an-option"], b"")
            .await;
        assert!(!invalid.ok);
        assert_eq!(owner.conversation(id).await.unwrap().permissions.len(), 1);
        let answer = fixture
            .vessel(&["inbox", "answer", &cursor, "allow-once"], b"")
            .await;
        assert!(answer.ok, "{}", answer.stderr);
        loop {
            let record = stream.next().await.unwrap().unwrap();
            if matches!(
                record.event,
                SessionEvent::Agent {
                    event: AgentEvent::TurnEnded { .. }
                }
            ) {
                break;
            }
        }
        let _committed = owner.read(id, EventSeq::ZERO, 1).await.unwrap();
        let late = fixture.vessel(&["inbox", "read", &cursor], b"").await;
        assert!(!late.ok);
        assert_eq!(
            late.stderr.trim(),
            "error: this inbox entry has moved on; run `vessel inbox list` again"
        );
        fixture.stop().await;
    })
    .await;
}
