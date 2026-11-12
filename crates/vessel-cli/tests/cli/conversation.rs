//! The conversation's commands (protocol 10) against a node in this
//! process with `fake-acp` as Claude's adapter: `vessel send` delivered
//! as asked and with files, `vessel session queue`, `answer`, `resume`,
//! the marks and `delete`, each with its main refusal.
//!
//! Each command is the `vessel` binary; the test follows the session's log
//! with a library client to know when a turn has ended, so it waits on
//! records, never on time.
#![expect(
    clippy::wildcard_enum_match_arm,
    reason = "tests reading a session's log for the few records they need"
)]

use vessel_client::NodeClient;
use vessel_proto::{AgentEvent, ContentBlock, EventSeq, SessionEvent, SessionId};

use crate::support::{Fixture, Ran, within};

/// A structured session, started and left running; its id.
async fn structured(fixture: &Fixture) -> String {
    let cwd = fixture.cwd();
    let run = fixture
        .vessel(
            &["run", "claude", "--structured", "--detach", "--cwd", &cwd],
            b"",
        )
        .await;
    assert!(run.ok, "{}", run.stderr);
    run.stdout.trim().to_owned()
}

/// Where `id`'s log stands now: records from here on are new.
async fn mark(owner: &NodeClient, id: &str) -> EventSeq {
    owner.session(parse(id)).await.unwrap().next_seq
}

fn parse(id: &str) -> SessionId {
    id.parse().unwrap()
}

/// Read `id` from `from` until the agent event `wanted` picks one; what the
/// agent said meanwhile, and that pick.
async fn until<T>(
    owner: &NodeClient,
    id: &str,
    from: EventSeq,
    wanted: impl Fn(&AgentEvent) -> Option<T>,
) -> (String, T) {
    let mut stream = owner.follow(parse(id), from).await.unwrap();
    let mut said = String::new();
    while let Some(record) = stream.next().await {
        if let SessionEvent::Agent { event } = record.unwrap().event {
            if let AgentEvent::MessageChunk {
                content: ContentBlock::Text { text },
            } = &event
            {
                said.push_str(text);
            }
            if let Some(found) = wanted(&event) {
                return (said, found);
            }
        }
    }
    panic!("the log of {id} ended first; said: {said}");
}

/// What the agent said from `from` until `count` turns have ended.
async fn turns(owner: &NodeClient, id: &str, from: EventSeq, count: usize) -> String {
    let mut stream = owner.follow(parse(id), from).await.unwrap();
    let (mut said, mut ended) = (String::new(), 0_usize);
    while let Some(record) = stream.next().await {
        match record.unwrap().event {
            SessionEvent::Agent {
                event:
                    AgentEvent::MessageChunk {
                        content: ContentBlock::Text { text },
                    },
            } => said.push_str(&text),
            SessionEvent::Agent {
                event: AgentEvent::TurnEnded { .. },
            } => {
                ended = ended.saturating_add(1);
                if ended == count {
                    return said;
                }
            }
            _ => {}
        }
    }
    panic!("the log of {id} ended first; said: {said}");
}

/// `vessel session queue ID <args>`.
async fn queue(fixture: &Fixture, id: &str, args: &[&str]) -> Ran {
    let mut full = vec!["session", "queue", id];
    full.extend_from_slice(args);
    fixture.vessel(&full, b"").await
}

/// `vessel session answer ID REQUEST <values>`.
async fn answer(fixture: &Fixture, id: &str, request: &str, values: &[&str]) -> Ran {
    let mut full = vec!["session", "answer", id, request];
    full.extend_from_slice(values);
    fixture.vessel(&full, b"").await
}

/// `vessel session COMMAND ID`.
async fn session(fixture: &Fixture, command: &str, id: &str) -> Ran {
    fixture.vessel(&["session", command, id], b"").await
}

/// An id's first eight characters, a prefix that names it.
fn short(id: &str) -> &str {
    id.get(..8).unwrap()
}

fn refused(ran: &Ran, why: &str) {
    assert!(!ran.ok, "{}", ran.stdout);
    assert!(ran.stderr.contains(why), "{why:?} not in: {}", ran.stderr);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn send_queues_steers_and_attaches_files() {
    within(async {
        let fixture = Fixture::start().await;
        let owner = fixture.owner().await;
        let id = structured(&fixture).await;
        let from = mark(&owner, &id).await;

        // The old `send` starts a turn, and is refused while one runs.
        let wait = fixture.vessel(&["send", &id, "wait"], b"").await;
        assert!(wait.ok && wait.stdout.is_empty(), "{}", wait.stderr);
        refused(
            &fixture.vessel(&["send", &id, "again"], b"").await,
            "Conflict",
        );
        let queued = fixture
            .vessel(&["send", &id, "plain", "--queue"], b"")
            .await;
        assert!(queued.ok, "{}", queued.stderr);
        assert!(queued.stdout.starts_with("queued "), "{}", queued.stdout);
        let steered = fixture
            .vessel(&["send", &id, "finish", "--steer"], b"")
            .await;
        assert!(steered.ok, "{}", steered.stderr);
        assert_eq!(steered.stdout, "steered\n");
        // The steered turn ends, then the queue sends what waited.
        let said = turns(&owner, &id, from, 2).await;
        assert!(said.contains("echo: plain"), "{said}");

        // Files go up a piece at a time and come to the agent with the
        // prompt: an image inline, a text file as a link it reads.
        let notes = fixture.data().join("notes.txt");
        std::fs::write(&notes, b"hello").unwrap();
        let shot = fixture.data().join("shot.png");
        let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
        png.extend_from_slice(&[0_u8; 8]);
        std::fs::write(&shot, &png).unwrap();
        let from = mark(&owner, &id).await;
        let notes = notes.display().to_string();
        let shot = shot.display().to_string();
        let sent = fixture
            .vessel(
                &[
                    "send", &id, "describe", "--attach", &notes, "--attach", &shot,
                ],
                b"",
            )
            .await;
        assert!(sent.ok, "{}", sent.stderr);
        assert_eq!(sent.stdout, "started\n");
        let said = turns(&owner, &id, from, 1).await;
        assert!(
            said.contains("link notes.txt text/plain 5 read 5"),
            "{said}"
        );
        assert!(said.contains("image image/png 16 bytes"), "{said}");

        let missing = fixture.data().join("missing.txt").display().to_string();
        refused(
            &fixture
                .vessel(&["send", &id, "x", "--attach", &missing], b"")
                .await,
            "cannot attach",
        );
        let terminal = fixture.run_detached().await;
        refused(
            &fixture
                .vessel(&["send", &terminal, "x", "--queue"], b"")
                .await,
            "for structured sessions",
        );
        fixture.stop().await;
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_queue_is_shown_and_changed() {
    within(async {
        let fixture = Fixture::start().await;
        let owner = fixture.owner().await;
        let id = structured(&fixture).await;
        let from = mark(&owner, &id).await;
        assert!(fixture.vessel(&["send", &id, "wait"], b"").await.ok);
        let mut entries = Vec::new();
        for text in ["draft", "second"] {
            let queued = fixture.vessel(&["send", &id, text, "--queue"], b"").await;
            assert!(queued.ok, "{}", queued.stderr);
            entries.push(
                queued
                    .stdout
                    .trim()
                    .strip_prefix("queued ")
                    .unwrap()
                    .to_owned(),
            );
        }
        let (first, second) = (&entries[0], &entries[1]);

        let shown = queue(&fixture, &id, &[]).await;
        assert!(shown.ok, "{}", shown.stderr);
        let lines: Vec<&str> = shown.stdout.lines().collect();
        assert_eq!(lines[0], "[queue: 2 waiting]", "{}", shown.stdout);
        assert!(lines[1].contains(first.as_str()) && lines[1].ends_with("draft"));
        assert!(lines[2].contains(second.as_str()) && lines[2].ends_with("second"));

        let held = queue(&fixture, &id, &["hold"]).await;
        assert!(
            held.stdout.starts_with("[queue: 2 waiting, held]"),
            "{}",
            held.stdout
        );
        let edited = queue(&fixture, &id, &["edit", short(first), "plain"]).await;
        assert!(edited.ok, "{}", edited.stderr);
        let line = edited.stdout.lines().nth(1).unwrap();
        assert!(
            line.contains("  r1  owner") && line.ends_with(" plain"),
            "{line}"
        );
        let moved = queue(&fixture, &id, &["move", short(first)]).await;
        let lines: Vec<&str> = moved.stdout.lines().collect();
        assert!(lines[1].contains(second.as_str()), "{}", moved.stdout);
        assert!(lines[2].contains(first.as_str()), "{}", moved.stdout);
        let removed = queue(&fixture, &id, &["remove", short(second)]).await;
        assert!(
            removed.stdout.starts_with("[queue: 1 waiting, held]"),
            "{}",
            removed.stdout
        );
        refused(
            &queue(&fixture, &id, &["remove", short(second)]).await,
            "no waiting prompt matches",
        );

        // Sending it now starts a turn: refused while one runs, and it
        // stays where it was.
        refused(
            &queue(&fixture, &id, &["send-now", short(first)]).await,
            "Conflict",
        );
        assert!(
            queue(&fixture, &id, &[])
                .await
                .stdout
                .contains(first.as_str())
        );
        assert!(
            fixture
                .vessel(&["send", &id, "finish", "--steer"], b"")
                .await
                .ok
        );
        let _said = turns(&owner, &id, from, 1).await;
        let resumed = queue(&fixture, &id, &["resume"]).await;
        assert!(resumed.ok, "{}", resumed.stderr);
        assert!(resumed.stdout.ends_with("started\n"), "{}", resumed.stdout);
        let said = turns(&owner, &id, from, 2).await;
        assert!(said.contains("echo: plain"), "{said}");
        assert_eq!(
            queue(&fixture, &id, &["clear"]).await.stdout,
            "[queue: 0 waiting]\n"
        );

        let terminal = fixture.run_detached().await;
        refused(
            &fixture.vessel(&["session", "queue", &terminal], b"").await,
            "Unsupported",
        );
        fixture.stop().await;
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn questions_and_permissions_are_answered() {
    within(async {
        let fixture = Fixture::start().await;
        let owner = fixture.owner().await;
        let id = structured(&fixture).await;
        let from = mark(&owner, &id).await;
        assert!(fixture.vessel(&["send", &id, "ask"], b"").await.ok);
        let (_said, question) = until(&owner, &id, from, |event| match event {
            AgentEvent::QuestionAsked(question) => Some(question.id.to_string()),
            _ => None,
        })
        .await;
        let prefix = short(&question);
        refused(
            &answer(&fixture, &id, prefix, &["way=up"]).await,
            "InvalidRequest",
        );
        refused(
            &answer(&fixture, &id, prefix, &["sure=maybe"]).await,
            "field `sure` takes yes or no",
        );
        refused(
            &answer(&fixture, &id, prefix, &["left"]).await,
            "`field=value`",
        );
        let answered = answer(&fixture, &id, prefix, &["way=left", "sure=yes"]).await;
        assert!(answered.ok, "{}", answered.stderr);
        let said = turns(&owner, &id, from, 1).await;
        assert!(
            said.contains(r#"answered: accept {"sure":true,"way":"left"}"#),
            "{said}"
        );
        refused(
            &answer(&fixture, &id, prefix, &["--decline"]).await,
            "no waiting question or permission request matches",
        );

        // Anything else asks a permission for its tool call; an option's
        // id answers it.
        let from = mark(&owner, &id).await;
        assert!(fixture.vessel(&["send", &id, "build"], b"").await.ok);
        let (_said, request) = until(&owner, &id, from, |event| match event {
            AgentEvent::PermissionRequested(request) => Some(request.id.to_string()),
            _ => None,
        })
        .await;
        let allowed = fixture
            .vessel(&["session", "answer", &id, &request, "allow-once"], b"")
            .await;
        assert!(allowed.ok, "{}", allowed.stderr);
        let _said = turns(&owner, &id, from, 1).await;
        let log = fixture.vessel(&["log", &id], b"").await;
        assert!(
            log.stdout.contains("answered: allow-once"),
            "{}",
            log.stdout
        );
        fixture.stop().await;
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sessions_are_marked_started_again_and_deleted() {
    within(async {
        let fixture = Fixture::start().await;
        let owner = fixture.owner().await;
        let id = structured(&fixture).await;
        let from = mark(&owner, &id).await;
        assert!(fixture.vessel(&["send", &id, "plain"], b"").await.ok);
        let _said = turns(&owner, &id, from, 1).await;
        drop(owner);

        // A turn's end asks for attention until the session is read.
        let pinned = session(&fixture, "pin", &id).await;
        assert!(pinned.ok, "{}", pinned.stderr);
        assert!(
            pinned.stdout.ends_with("[pinned, unread]\n"),
            "{}",
            pinned.stdout
        );
        assert!(
            session(&fixture, "read", &id)
                .await
                .stdout
                .ends_with("[pinned]\n")
        );
        assert!(
            session(&fixture, "unread", &id)
                .await
                .stdout
                .ends_with("[pinned, unread]\n")
        );
        assert!(
            session(&fixture, "unpin", &id)
                .await
                .stdout
                .ends_with("[unread]\n")
        );
        refused(
            &fixture.vessel(&["session", "pin", "abcdef"], b"").await,
            "no session matches",
        );

        // Archiving a running session stops it.
        let archived = session(&fixture, "archive", &id).await;
        assert!(archived.ok, "{}", archived.stderr);
        assert!(archived.stdout.contains("archived "), "{}", archived.stdout);
        assert!(!archived.stdout.contains("running"), "{}", archived.stdout);
        refused(
            &session(&fixture, "delete", &id).await,
            "confirm with --yes",
        );

        // Once the store has it whole, it starts again, out of the archive.
        let fixture = fixture.restart().await;
        let unarchived = session(&fixture, "unarchive", &id).await;
        assert!(
            !unarchived.stdout.contains("archived"),
            "{}",
            unarchived.stdout
        );
        assert!(
            session(&fixture, "archive", &id)
                .await
                .stdout
                .contains("archived ")
        );
        let resumed = session(&fixture, "resume", &id).await;
        assert!(resumed.ok, "{}", resumed.stderr);
        assert!(resumed.stdout.contains(" running "), "{}", resumed.stdout);
        assert!(!resumed.stdout.contains("archived"), "{}", resumed.stdout);
        refused(&session(&fixture, "resume", &id).await, "Conflict");
        refused(
            &fixture
                .vessel(&["session", "delete", &id, "--yes"], b"")
                .await,
            "Conflict",
        );

        let killed = fixture.vessel(&["kill", &id], b"").await;
        assert!(killed.ok, "{}", killed.stderr);
        let fixture = fixture.restart().await;
        let deleted = fixture
            .vessel(&["session", "delete", &id, "--yes"], b"")
            .await;
        assert!(deleted.ok, "{}", deleted.stderr);
        let listed = fixture.vessel(&["sessions"], b"").await;
        assert!(!listed.stdout.contains(&id), "{}", listed.stdout);
        refused(
            &fixture
                .vessel(&["session", "delete", &id, "--yes"], b"")
                .await,
            "NotFound",
        );
        fixture.stop().await;
    })
    .await;
}
