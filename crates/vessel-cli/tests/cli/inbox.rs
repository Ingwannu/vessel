//! Real CLI list/read/answer commands against a node and fake ACP agent.
use crate::support::{Fixture, within};
use vessel_proto::inbox::{InboxItem, InboxRead, ListInbox};
use vessel_proto::{
    AgentEvent, ContentBlock, EventSeq, HarnessKind, SessionEvent, SessionInput, SessionMode,
    SessionSpec,
};

#[derive(Clone, Copy)]
enum Wanted {
    Permission,
    Question,
    Ended,
    Pruned,
}
impl Wanted {
    fn matches(self, event: &AgentEvent) -> bool {
        match self {
            Self::Permission => matches!(event, AgentEvent::PermissionRequested(_)),
            Self::Question => matches!(event, AgentEvent::QuestionAsked(_)),
            Self::Ended => matches!(event, AgentEvent::TurnEnded { .. }),
            Self::Pruned => {
                matches!(event, AgentEvent::MessageChunk { content: ContentBlock::Text { text } } if text == "form source may now be pruned")
            }
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::Permission => "inbox permission request",
            Self::Question => "inbox question request",
            Self::Ended => "inbox answered turn ending",
            Self::Pruned => "inbox form source pruning marker",
        }
    }
}
async fn wait_for(stream: &mut vessel_client::SessionStream, wanted: Wanted) -> Vec<String> {
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        let mut messages = Vec::new();
        loop {
            let record = match stream.next().await.unwrap() {
                Ok(record) => record,
                Err(vessel_client::StreamError::Missing { .. })
                    if matches!(wanted, Wanted::Pruned) =>
                {
                    continue;
                }
                Err(error) => panic!("unexpected inbox stream failure: {error}"),
            };
            if let SessionEvent::Agent { event } = record.event {
                if let AgentEvent::MessageChunk {
                    content: ContentBlock::Text { text },
                } = &event
                {
                    messages.push(text.clone());
                }
                assert!(
                    !matches!(event, AgentEvent::TurnEnded { .. })
                        || matches!(wanted, Wanted::Ended),
                    "the fake turn ended before {}; text received: {messages:?}",
                    wanted.label()
                );
                if wanted.matches(&event) {
                    return messages;
                }
            }
        }
    })
    .await
    .unwrap_or_else(|_elapsed| panic!("timed out waiting for {}", wanted.label()))
}
async fn structured(
    fixture: &Fixture,
    owner: &vessel_client::NodeClient,
) -> vessel_proto::SessionId {
    owner
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
        .id
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn inbox_lists_marks_and_answers_the_actors_own_request() {
    within(async {
        let fixture = Fixture::start().await;
        let owner = fixture.owner().await;
        let id = structured(&fixture, &owner).await;
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
        drop(wait_for(&mut stream, Wanted::Permission).await);
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
        drop(wait_for(&mut stream, Wanted::Ended).await);
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn inbox_question_keeps_its_form_rejects_an_invalid_choice_and_delivers_cli_values() {
    within(async {
        let fixture = Fixture::start().await;
        let owner = fixture.owner().await;
        let id = structured(&fixture, &owner).await;
        let mut stream = owner.follow(id, EventSeq::ZERO).await.unwrap();
        owner.send(id, SessionInput::Prompt {
            content: vec![ContentBlock::Text { text: "ask".to_owned() }],
        }).await.unwrap();
        drop(wait_for(&mut stream, Wanted::Question).await);
        let _committed = owner.read(id, EventSeq::ZERO, 1).await.unwrap();
        let entry = owner.inbox(ListInbox::default()).await.unwrap().entries.remove(0);
        let InboxItem::Question { question } = entry.item else { panic!("the question must be complete"); };
        let cursor = entry.cursor.to_string();
        let listed = fixture.vessel(&["inbox", "list"], b"").await;
        assert!(listed.ok, "{}", listed.stderr);
        let header = format!("  [question {}] Which way?", question.id);
        let shown = listed.stdout.lines().skip_while(|line| *line != header)
            .take(6).collect::<Vec<_>>().join("\n");
        assert_eq!(shown, format!("{header}\n  name: Name (text)\n  sure: Sure (yes or no)\n  way (required): Which way (choose)\n    left: Left\n    right: Right"));
        let invalid = fixture.vessel(&["inbox", "answer", &cursor, "way=up"], b"").await;
        assert!(!invalid.ok);
        assert_eq!(invalid.stderr.trim(), format!("error: the node returned an error: InvalidRequest: the answer does not fit question {}: field `way`: not one of the field's options", question.id));
        assert!(owner.inbox_entry(entry.key).await.unwrap().is_some());
        assert_eq!(owner.conversation(id).await.unwrap().questions.len(), 1);
        let answer = fixture.vessel(&["inbox", "answer", &cursor, "name=Inbox User", "way=right", "sure=true"], b"").await;
        assert!(answer.ok, "{}", answer.stderr);
        assert_eq!(answer.stdout, "");
        assert_eq!(wait_for(&mut stream, Wanted::Ended).await,
            [r#"answered: accept {"name":"Inbox User","sure":true,"way":"right"}"#]);
        let _committed = owner.read(id, EventSeq::ZERO, 1).await.unwrap();
        assert!(owner.inbox_entry(entry.key).await.unwrap().is_none());
        let late = fixture.vessel(&["inbox", "answer", &cursor, "way=left"], b"").await;
        assert!(!late.ok);
        assert_eq!(late.stderr.trim(), "error: this inbox entry has moved on; run `vessel inbox list` again");
        fixture.stop().await;
    }).await;
}

mod show;
