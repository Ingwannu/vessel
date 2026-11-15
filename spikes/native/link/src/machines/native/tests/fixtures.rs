//! Isolated actor construction and bounded transcript checks.
use super::peer;
use crate::machines::{Answer, BookPath};
use crate::{Connected, Event, State};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tokio::sync::mpsc;
use vessel_app_view::machine_book::{Edit, Failure, Name};
use vessel_app_view::{Agent, Lang};
use vessel_proto::{Call, SessionId};
pub(super) const CANARY: &str = "synthetic-book-credential-never-display";
pub(super) const BOOK: &str = "default_node = \"laptop\"\n[nodes.laptop]\nurl = \"ws://127.0.0.1:0/rpc\"\n[nodes.other]\nurl = \"ws://127.0.0.1:0/rpc\"\n";
static NEXT: AtomicU64 = AtomicU64::new(0);
pub(super) struct Home(pub(super) PathBuf);
impl Home {
    pub(super) fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "vessel-machine-actor-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
    pub(super) fn book(&self) -> PathBuf {
        self.0.join("nodes.toml")
    }
}
impl Drop for Home {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}
pub(super) fn name(text: &str) -> Name {
    Name::parse(text).unwrap()
}
pub(super) fn state(home: &Home) -> (State, mpsc::Receiver<Event>) {
    let (events, receiving) = mpsc::channel(256);
    (
        State {
            label: "test".to_owned(),
            events,
            clients: HashMap::new(),
            follow: None,
            follow_machine: None,
            book_path: BookPath::File(home.book()),
            workspace: crate::workspace::Hub::default(),
            agents: crate::agents::Hub::default(),
            device_calls: crate::devices::Hub::default(),
            sync_calls: crate::sync::Hub::default(),
            offers: std::sync::Arc::default(),
        },
        receiving,
    )
}
pub(super) struct Fixture {
    pub(super) home: Home,
    pub(super) state: State,
    pub(super) events: mpsc::Receiver<Event>,
    pub(super) laptop: peer::Peer,
    pub(super) other: peer::Peer,
}
impl Fixture {
    pub(super) async fn new() -> Self {
        let home = Home::new();
        std::fs::write(home.book(), BOOK).unwrap();
        let (mut state, events) = state(&home);
        let mut laptop = peer::connect("laptop").await;
        let mut other = peer::connect("other").await;
        for (name, peer) in [("laptop", &mut laptop), ("other", &mut other)] {
            let previous = state.clients.insert(
                name.to_owned(),
                Connected {
                    client: peer.client.take().unwrap(),
                    agents: vec![Agent {
                        harness: vessel_proto::HarnessKind::ClaudeCode.into(),
                        name: "Example agent".to_owned(),
                        unavailable: None,
                        not_installed: false,
                    }],
                },
            );
            assert!(previous.is_none());
        }
        Self {
            home,
            state,
            events,
            laptop,
            other,
        }
    }
    pub(super) async fn watch(&mut self) {
        let session: SessionId = "00000000-0000-4000-8000-000000000001".parse().unwrap();
        self.state.open("laptop", session);
        tokio::time::timeout(Duration::from_secs(5), self.laptop.subscribed.recv())
            .await
            .unwrap()
            .unwrap();
        assert!(self.state.follow.is_some());
    }
}
pub(super) fn drain(events: &mut mpsc::Receiver<Event>) -> Vec<Event> {
    std::iter::from_fn(|| events.try_recv().ok()).collect()
}
pub(super) fn outcomes(events: &[Event]) -> Vec<(Edit, Result<(), Failure>)> {
    events
        .iter()
        .filter_map(|event| {
            if let Event::Machines(Answer::Edited { edit, result }) = event {
                Some((edit.clone(), result.clone()))
            } else {
                None
            }
        })
        .collect()
}
pub(super) fn safe(event: &Event) {
    assert!(
        !format!("{event:?}").contains(CANARY),
        "a credential reached an event"
    );
    for lang in [Lang::En, Lang::Ko] {
        if let Event::BookUnreadable(fault) = event {
            assert!(
                !lang.machines_unreadable(fault).contains(CANARY),
                "a credential reached a file notice"
            );
        }
        if let Event::Machines(
            Answer::Read(Err(why))
            | Answer::Edited {
                result: Err(why), ..
            },
        ) = event
        {
            assert!(
                !lang.machine_book_failure(why).contains(CANARY),
                "a credential reached a machine notice"
            );
        }
    }
}
pub(super) fn no_remote_changes(peer: &peer::Peer) {
    assert!(
        peer.calls.lock().unwrap().iter().all(|call| matches!(
            call,
            Call::ListSessions(_) | Call::Subscribe(_) | Call::Unsubscribe(_)
        )),
        "book edits sent a remote mutation"
    );
}
pub(super) async fn next(events: &mut mpsc::Receiver<Event>) -> Event {
    let event = tokio::time::timeout(Duration::from_secs(5), events.recv())
        .await
        .unwrap()
        .unwrap();
    safe(&event);
    event
}
