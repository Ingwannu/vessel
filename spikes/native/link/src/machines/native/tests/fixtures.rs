//! Isolated actor construction and bounded transcript checks.
use super::peer;
use crate::machines::Answer;
pub(super) use crate::testing::{Home, state};
use crate::{Connected, Event, State};
use tokio::sync::mpsc;
use vessel_app_view::machine_book::{Edit, Failure, Name};
use vessel_app_view::{Agent, Lang};
use vessel_proto::{Call, SessionId};
pub(super) const CANARY: &str = "synthetic-book-credential-never-display";
pub(super) const BOOK: &str = "default_node = \"laptop\"\n[nodes.laptop]\nurl = \"ws://127.0.0.1:0/rpc\"\n[nodes.other]\nurl = \"ws://127.0.0.1:0/rpc\"\n";
pub(super) fn name(text: &str) -> Name {
    Name::parse(text).unwrap()
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
            let client = peer.client.take().unwrap();
            let context = state
                .admissions
                .admit(
                    vessel_app_view::devices::MachineKey::from_book(name),
                    client.node().id,
                )
                .unwrap();
            let binding = crate::machines::edits::entries(&home.book())
                .unwrap()
                .into_iter()
                .find(|(key, _)| key.as_str() == name)
                .unwrap()
                .1;
            let previous = state.clients.insert(
                name.to_owned(),
                Connected::new(
                    client,
                    vec![Agent {
                        harness: vessel_proto::HarnessKind::ClaudeCode.into(),
                        name: "Example agent".to_owned(),
                        unavailable: None,
                        not_installed: false,
                    }],
                    binding,
                    context,
                ),
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
        tokio::time::timeout(crate::testing::DEADLINE, self.laptop.subscribed.recv())
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
    let event = tokio::time::timeout(crate::testing::DEADLINE, events.recv())
        .await
        .unwrap()
        .unwrap();
    safe(&event);
    event
}
