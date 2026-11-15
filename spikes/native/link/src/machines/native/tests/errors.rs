//! Every actor error route is checked before it can become screen text.
use super::fixtures::*;
use crate::machines::{Answer, Ask};
use crate::{Command, Event, State};
use std::time::Duration;
use tokio::sync::mpsc;
use vessel_app_view::Agent;
use vessel_app_view::machine_book::{Edit, Failure};
async fn snapshot(events: &mut mpsc::Receiver<Event>) -> Vec<Event> {
    tokio::time::timeout(Duration::from_secs(10), async {
        let mut collected = Vec::new();
        loop {
            let event = next(events).await;
            let done = matches!(&event, Event::Machines(Answer::Read(Err(Failure::File(_)))));
            collected.push(event);
            if done {
                return collected;
            }
        }
    })
    .await
    .expect("the actor never settled its snapshot")
}

#[tokio::test]
async fn every_actor_book_error_path_keeps_the_canary_out_of_events() {
    let home = Home::new();
    std::fs::write(
        home.book(),
        format!("[nodes.laptop]\nurl = \"ws://127.0.0.1:0/rpc\"\ntoken = \"{CANARY}\"\n"),
    )
    .unwrap();
    let (state, mut events) = state(&home);
    let (send, inbox) = mpsc::channel(8);
    let running = tokio::spawn(state.run(inbox));
    // Startup reload and the interval's immediate first tick both reach refresh.
    for _ in 0..2 {
        assert!(matches!(next(&mut events).await, Event::BookUnreadable(_)));
    }
    send.send(Command::Reload).await.unwrap();
    send.send(Command::Machines(Ask::Read)).await.unwrap();
    let reloaded = snapshot(&mut events).await;
    assert!(
        reloaded
            .iter()
            .any(|event| matches!(event, Event::BookUnreadable(_)))
    );
    send.send(Command::Machines(Ask::Retry)).await.unwrap();
    let retried = snapshot(&mut events).await;
    assert!(
        retried
            .iter()
            .any(|event| matches!(event, Event::BookUnreadable(_)))
    );
    let edit = Edit::Rename {
        from: name("laptop"),
        to: name("renamed"),
    };
    send.send(Command::Machines(Ask::Edit(edit.clone())))
        .await
        .unwrap();
    let seen = snapshot(&mut events).await;
    let edited = outcomes(&seen);
    assert_eq!(edited.len(), 1);
    assert_eq!(edited.first().unwrap().0, edit);
    assert!(matches!(&edited.first().unwrap().1, Err(Failure::File(_))));
    drop(send);
    tokio::time::timeout(Duration::from_secs(5), running)
        .await
        .unwrap()
        .unwrap();
    for event in drain(&mut events) {
        safe(&event);
    }
}

#[derive(Debug, PartialEq, Eq)]
struct LiveState {
    clients: std::collections::BTreeMap<String, (vessel_proto::NodeId, Vec<Agent>)>,
    machine: Option<String>,
    follow: Option<tokio::task::Id>,
}
fn live(state: &State) -> LiveState {
    LiveState {
        clients: state
            .clients
            .iter()
            .map(|(name, connected)| {
                (
                    name.clone(),
                    (connected.client.node().id, connected.agents.clone()),
                )
            })
            .collect(),
        machine: state.follow_machine.clone(),
        follow: state.follow.as_ref().map(tokio::task::JoinHandle::id),
    }
}

#[tokio::test]
async fn a_malformed_book_does_not_change_either_live_connection_or_follow() {
    let mut f = Fixture::new().await;
    f.watch().await;
    let before = live(&f.state);
    let bad = format!("[nodes.laptop]\nurl = \"ws://127.0.0.1:0/rpc\"\ntoken = \"{CANARY}\"\n");
    std::fs::write(f.home.book(), &bad).unwrap();
    f.state
        .handle(Command::Machines(Ask::Edit(Edit::Rename {
            from: name("laptop"),
            to: name("renamed"),
        })))
        .await;
    assert_eq!(live(&f.state), before);
    assert_eq!(std::fs::read_to_string(f.home.book()).unwrap(), bad);
    let events = drain(&mut f.events);
    for event in &events {
        safe(event);
    }
    assert!(matches!(
        outcomes(&events).first().map(|(_, result)| result),
        Some(Err(Failure::File(_)))
    ));
    no_remote_changes(&f.laptop);
    no_remote_changes(&f.other);
}

#[cfg(unix)]
#[tokio::test]
async fn a_denied_write_keeps_the_clients_follow_and_original_file() {
    use std::os::unix::fs::PermissionsExt as _;
    let mut f = Fixture::new().await;
    f.watch().await;
    let before = live(&f.state);
    let edit = Edit::Rename {
        from: name("laptop"),
        to: name("renamed"),
    };
    std::fs::set_permissions(&f.home.0, std::fs::Permissions::from_mode(0o500)).unwrap();
    f.state
        .handle(Command::Machines(Ask::Edit(edit.clone())))
        .await;
    std::fs::set_permissions(&f.home.0, std::fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(live(&f.state), before);
    assert_eq!(
        outcomes(&drain(&mut f.events)),
        vec![(edit, Err(Failure::Write))]
    );
    assert_eq!(std::fs::read_to_string(f.home.book()).unwrap(), BOOK);
    no_remote_changes(&f.laptop);
    no_remote_changes(&f.other);
}
