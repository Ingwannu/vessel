//! Several clients on one node's terminals over the wire (ADR 0116): a
//! `terminal.list` long poll answers when a shell starts, is renamed or
//! closes, and not for a resize or output; only the connection that
//! claimed a terminal (or a terminal-mode session) updates its size.
//!
//! Unix only: the shell is `/bin/sh`.

use std::time::Duration;

use bytes::Bytes;
use vessel_client::NodeClient;
use vessel_node::{Config, Shell};
use vessel_proto::terminals::{
    CloseTerminal, CreateTerminal, GetTerminal, ListTerminals, RenameTerminal,
    SHARED_TERMINALS_PROTOCOL, SendTerminal, TerminalList, TerminalPlace,
};
use vessel_proto::{
    EventSeq, HarnessKind, ResizeIntent, SessionInput, TerminalBytes, TerminalId, TerminalInput,
    TerminalSize,
};

use crate::support::{Fixture, collect_until, output_of, terminal_spec, within};

/// How long a poll that must not answer is watched.
const QUIET: Duration = Duration::from_millis(400);

fn with_shell(config: &mut Config) {
    config.terminals.shell = Some(Shell::plain("/bin/sh"));
}

fn resize(cols: u16, rows: u16, intent: Option<ResizeIntent>) -> TerminalInput {
    TerminalInput::Resize {
        size: TerminalSize { cols, rows },
        intent,
    }
}

async fn send(client: &NodeClient, terminal: TerminalId, input: TerminalInput) {
    let _ack = client.call(SendTerminal { terminal, input }).await.unwrap();
}

async fn size_of(client: &NodeClient, terminal: TerminalId) -> Option<TerminalSize> {
    client
        .call(GetTerminal {
            terminal,
            changed_from: None,
        })
        .await
        .unwrap()
        .size
}

/// A `terminal.list` poll of `place` from `revision`, answered in the
/// background.
fn poll(
    client: &NodeClient,
    place: &TerminalPlace,
    revision: u64,
) -> tokio::task::JoinHandle<TerminalList> {
    let client = client.clone();
    let asked = ListTerminals {
        place: Some(place.clone()),
        changed_from: Some(revision),
    };
    tokio::spawn(async move { client.call(asked).await.unwrap() })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_list_poll_answers_a_start_a_rename_and_a_close_but_not_a_resize() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture.start_with(with_shell).await;
        let page = fixture.connect(&node).await;
        let app = fixture.connect(&node).await;
        assert!(Some(page.protocol()) >= SHARED_TERMINALS_PROTOCOL);
        let session = page
            .create_session(terminal_spec(&fixture, HarnessKind::ClaudeCode))
            .await
            .unwrap();
        let place = TerminalPlace::Session {
            session: session.id,
        };
        let first = page
            .call(ListTerminals {
                place: Some(place.clone()),
                changed_from: None,
            })
            .await
            .unwrap();
        assert!(first.terminals.is_empty());

        // The app opens a shell: the page's waiting poll answers with it.
        let waiting = poll(&page, &place, first.revision);
        let shell = app
            .call(CreateTerminal {
                place: place.clone(),
                size: None,
                title: None,
            })
            .await
            .unwrap();
        let started = waiting.await.unwrap();
        assert_ne!(started.revision, first.revision);
        assert!(started.terminals.iter().any(|info| info.id == shell.id));

        // A resize and output change nothing a list shows.
        let waiting = poll(&page, &place, started.revision);
        send(&app, shell.id, resize(100, 30, Some(ResizeIntent::Claim))).await;
        let typed = TerminalBytes::new(Bytes::from_static(b"echo quiet\n"));
        send(&app, shell.id, TerminalInput::Data { data: typed }).await;
        tokio::time::sleep(QUIET).await;
        assert!(!waiting.is_finished(), "a resize or output moved the list");

        // A rename does.
        let _renamed = app
            .call(RenameTerminal {
                terminal: shell.id,
                title: "build".parse().unwrap(),
            })
            .await
            .unwrap();
        let renamed = waiting.await.unwrap();
        assert_ne!(renamed.revision, started.revision);

        // So does a close (the shell ends, then is forgotten).
        let waiting = poll(&page, &place, renamed.revision);
        let _ack = app
            .call(CloseTerminal { terminal: shell.id })
            .await
            .unwrap();
        let closed = waiting.await.unwrap();
        assert_ne!(closed.revision, renamed.revision);
        node.stop().await.unwrap();
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn only_the_connection_that_claimed_a_terminal_updates_its_size() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture.start_with(with_shell).await;
        let app = fixture.connect(&node).await;
        let page = fixture.connect(&node).await;
        let session = app
            .create_session(terminal_spec(&fixture, HarnessKind::ClaudeCode))
            .await
            .unwrap();
        let shell = app
            .call(CreateTerminal {
                place: TerminalPlace::Session {
                    session: session.id,
                },
                size: None,
                title: None,
            })
            .await
            .unwrap();
        let size = |cols, rows| Some(TerminalSize { cols, rows });

        send(&app, shell.id, resize(100, 30, Some(ResizeIntent::Claim))).await;
        assert_eq!(size_of(&app, shell.id).await, size(100, 30));
        // The page does not own it: its update is dropped, answered as done.
        send(&page, shell.id, resize(80, 24, Some(ResizeIntent::Update))).await;
        assert_eq!(size_of(&app, shell.id).await, size(100, 30));
        send(&app, shell.id, resize(110, 32, Some(ResizeIntent::Update))).await;
        assert_eq!(size_of(&app, shell.id).await, size(110, 32));

        // No intent is a claim, as every older client sends.
        send(&page, shell.id, resize(90, 20, None)).await;
        assert_eq!(size_of(&app, shell.id).await, size(90, 20));
        send(&app, shell.id, resize(120, 40, Some(ResizeIntent::Update))).await;
        assert_eq!(size_of(&app, shell.id).await, size(90, 20));

        // A gone owner matches no later update; the next claim takes it.
        drop(page);
        send(&app, shell.id, resize(70, 20, Some(ResizeIntent::Update))).await;
        assert_eq!(size_of(&app, shell.id).await, size(90, 20));
        send(&app, shell.id, resize(70, 20, Some(ResizeIntent::Claim))).await;
        assert_eq!(size_of(&app, shell.id).await, size(70, 20));
        node.stop().await.unwrap();
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_terminal_sessions_size_follows_its_claim_too() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture.start().await;
        let app = fixture.connect(&node).await;
        let page = fixture.connect(&node).await;
        let info = app
            .create_session(terminal_spec(&fixture, HarnessKind::ClaudeCode))
            .await
            .unwrap();
        let mut live = app.follow(info.id, EventSeq::ZERO).await.unwrap();
        let mut seen = Vec::new();
        collect_until(&mut live, &mut seen, "> ").await;
        let sized = |cols, rows, intent| SessionInput::Terminal {
            input: resize(cols, rows, intent),
        };

        app.send(info.id, sized(120, 40, Some(ResizeIntent::Claim)))
            .await
            .unwrap();
        collect_until(&mut live, &mut seen, "size 120x40").await;
        // The page's update is dropped; its claim then applies. Had the
        // update applied, its size would be printed before the claim's.
        page.send(info.id, sized(90, 30, Some(ResizeIntent::Update)))
            .await
            .unwrap();
        page.send(info.id, sized(100, 30, Some(ResizeIntent::Claim)))
            .await
            .unwrap();
        collect_until(&mut live, &mut seen, "size 100x30").await;
        assert!(!output_of(&seen).contains("size 90x30"), "{seen:?}");
        node.stop().await.unwrap();
    })
    .await;
}
