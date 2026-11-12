//! Prompts naming attachments (protocol 10, ADR 0025): only the session's
//! own, within a prompt's bounds, of a kind its agent takes when the
//! prompt is sent (a queued one checked again then); a file swapped for a
//! link is never read back; deleting a session removes its files.

use std::collections::BTreeSet;

use bytes::Bytes;
use vessel_proto::{
    AgentEvent, AttachmentId, AttachmentKind, AttachmentRef, Delivered, Delivery, ErrorCode,
    EventSeq, QueueChange, QueueHold, SessionId, SessionInput,
};

use crate::agents::code;
use crate::attachments::{answer, attached, mime, name, png, running, text};
use crate::support::{Fixture, agent_of, read_until, status_of, within};

/// A prompt names only its session's attachments, each once, at most 16;
/// whatever way it comes in (`session.prompt`, the queue, `session.send`).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_prompt_names_only_its_sessions_attachments() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        let (mine, _, _) = running(&client, &fixture, None).await;
        let (theirs, _, _) = running(&client, &fixture, None).await;
        let file = |session| {
            client.attach(
                session,
                name("notes.txt"),
                mime("text/plain"),
                Bytes::from_static(b"notes"),
            )
        };
        let own = file(mine).await.unwrap();
        let foreign = file(theirs).await.unwrap();

        let prompt = |content, delivery| client.prompt(mine, content, delivery);
        for delivery in [Delivery::Start, Delivery::Queue] {
            assert_eq!(
                code(prompt(vec![text("look"), attached(&foreign)], delivery).await),
                ErrorCode::InvalidRequest,
                "another session's, {delivery:?}"
            );
        }
        assert_eq!(
            code(prompt(vec![attached(&own), attached(&own)], Delivery::Start).await),
            ErrorCode::InvalidRequest,
            "named twice"
        );
        let many: Vec<_> = (0..17)
            .map(|_| {
                attached(&AttachmentRef {
                    id: AttachmentId::random(),
                    ..own.clone()
                })
            })
            .collect();
        assert_eq!(
            code(prompt(many, Delivery::Start).await),
            ErrorCode::InvalidRequest,
            "17"
        );
        assert_eq!(
            code(
                client
                    .prompt(SessionId::random(), vec![attached(&own)], Delivery::Start)
                    .await
            ),
            ErrorCode::NotFound
        );
        let sent = client
            .send(
                mine,
                SessionInput::Prompt {
                    content: vec![attached(&foreign)],
                },
            )
            .await;
        assert_eq!(code(sent), ErrorCode::InvalidRequest, "session.send");

        // The queue's edit too.
        assert_eq!(
            prompt(vec![text("wait")], Delivery::Start).await.unwrap(),
            Delivered::Started
        );
        let Delivered::Queued { entry } =
            prompt(vec![text("describe"), attached(&own)], Delivery::Queue)
                .await
                .unwrap()
        else {
            panic!("a turn runs: it waits");
        };
        let edit = QueueChange::Edit {
            entry,
            revision: 0,
            content: vec![attached(&foreign)],
        };
        assert_eq!(
            code(client.change_queue(mine, edit).await),
            ErrorCode::InvalidRequest
        );
        node.stop().await.unwrap();
    })
    .await;
}

const HEARS: &str = r#"
[profiles.hears]
harness = "claude"
env = { FAKE_ACP_AUDIO = "1" }
"#;

const DEAF: &str = r#"
[profiles.hears]
harness = "claude"
"#;

/// Audio reaches an agent that takes it; once the agent no longer does
/// (started again without), a prompt naming it is refused and a queued
/// one is not sent: the queue is held, and its log says why.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_queued_prompt_is_checked_again_when_it_is_sent() {
    within(async {
        let fixture = Fixture::new();
        std::fs::write(fixture.data_dir().profiles_path(), HEARS).unwrap();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        let (id, mut stream, mut seen) = running(&client, &fixture, Some("hears")).await;
        let audio = client
            .attach(
                id,
                name("note.wav"),
                mime("audio/wav"),
                Bytes::from_static(b"RIFF....WAVE"),
            )
            .await
            .unwrap();
        assert_eq!(audio.kind, AttachmentKind::Audio);
        let said = answer(
            &client,
            (&mut stream, &mut seen),
            id,
            vec![text("describe"), attached(&audio)],
        )
        .await;
        assert_eq!(said, "text 8; audio audio/wav 12 bytes");

        // Queued behind a turn, then the session stops with it waiting.
        let wait = vec![text("wait")];
        let _started = client.prompt(id, wait, Delivery::Start).await.unwrap();
        let queued = vec![text("describe"), attached(&audio)];
        let Delivered::Queued { .. } = client
            .prompt(id, queued.clone(), Delivery::Queue)
            .await
            .unwrap()
        else {
            panic!("a turn runs: it waits");
        };
        client.kill(id).await.unwrap();
        read_until(&mut stream, &mut seen, |record| {
            status_of(record).is_some_and(|status| !status.is_live())
        })
        .await;

        // The profile no longer takes audio; the session starts again.
        node.stop().await.unwrap();
        std::fs::write(fixture.data_dir().profiles_path(), DEAF).unwrap();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        let resumed = client.resume_session(id).await.unwrap();
        let mut stream = client.follow(id, resumed.next_seq).await.unwrap();
        let mut seen = Vec::new();
        assert_eq!(
            code(client.prompt(id, queued.clone(), Delivery::Start).await),
            ErrorCode::InvalidRequest,
            "the agent takes no audio now"
        );
        let changed = client.change_queue(id, QueueChange::Resume).await.unwrap();
        assert_eq!(changed.delivered, None, "not sent");
        assert_eq!(changed.queue.held, Some(QueueHold::Failed));
        assert_eq!(changed.queue.entries.len(), 1, "kept");
        assert_eq!(changed.queue.entries[0].content, queued);
        read_until(&mut stream, &mut seen, |record| {
            matches!(
                agent_of(record),
                Some(AgentEvent::HarnessLog { line }) if line.contains("was not sent")
            )
        })
        .await;
        let started = seen
            .iter()
            .any(|record| matches!(agent_of(record), Some(AgentEvent::TurnStarted { .. })));
        assert!(!started, "nothing was sent");
        node.stop().await.unwrap();
    })
    .await;
}

/// The agent swaps the file it was handed for a link to the node's token:
/// neither `attachment.read` nor a prompt reads it, and no answer carries
/// the token.
#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_file_swapped_for_a_link_is_never_read_back() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        let (id, _, _) = running(&client, &fixture, None).await;
        let token = fixture.data_dir().token_path();
        let size = usize::try_from(std::fs::metadata(&token).unwrap().len()).unwrap();
        // As large as the token, so only its content can give it away.
        let image = client
            .attach(id, name("shot.png"), mime("image/png"), png(size))
            .await
            .unwrap();
        let path = fixture
            .data_dir()
            .attachments_dir()
            .join(id.to_string())
            .join(image.id.to_string())
            .join("shot.png");
        std::fs::remove_file(&path).unwrap();
        std::os::unix::fs::symlink(&token, &path).unwrap();
        let secret = std::fs::read_to_string(&token).unwrap();

        let Err(vessel_rpc::CallError::Remote(error)) =
            client.read_attachment(id, image.id, 0).await
        else {
            panic!("a link is never read");
        };
        assert_eq!(error.code, ErrorCode::Conflict);
        assert!(!error.message.contains(secret.trim()));
        let refused = client
            .prompt(
                id,
                vec![text("describe"), attached(&image)],
                Delivery::Start,
            )
            .await;
        assert_eq!(code(refused), ErrorCode::Conflict);

        // A hard link is a regular file: its content gives it away.
        std::fs::remove_file(&path).unwrap();
        std::fs::hard_link(&token, &path).unwrap();
        assert_eq!(
            code(client.read_attachment(id, image.id, 0).await),
            ErrorCode::Conflict
        );
        node.stop().await.unwrap();
    })
    .await;
}

/// `session.delete` takes a session's attachment files with it.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn deleting_a_session_removes_its_attachments() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        let (id, _, _) = running(&client, &fixture, None).await;
        let kept = client
            .attach(
                id,
                name("notes.txt"),
                mime("text/plain"),
                Bytes::from_static(b"notes"),
            )
            .await
            .unwrap();
        let folder = fixture.data_dir().attachments_dir().join(id.to_string());
        let names: BTreeSet<_> = std::fs::read_dir(&folder)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(names, BTreeSet::from([kept.id.to_string().into()]));
        node.stop().await.unwrap();

        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        assert!(folder.exists(), "kept across a restart: its row names it");
        let deleted = client.delete_session(id).await.unwrap();
        assert_eq!(deleted.worktree, None, "it ran in no worktree");
        assert!(!folder.exists());
        let log = client.read(id, EventSeq::ZERO, 10).await;
        assert_eq!(code(log), ErrorCode::NotFound);
        node.stop().await.unwrap();
    })
    .await;
}
