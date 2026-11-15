//! Connection and watched-stream lifetimes across confirmed local edits.
use super::fixtures::*;
use crate::Command;
use crate::machines::Ask;
use std::time::Duration;
use vessel_app_view::machine_book::{Edit, Failure};
#[tokio::test]
async fn successful_rename_keeps_the_connection_agents_and_watched_task() {
    let mut f = Fixture::new().await;
    f.watch().await;
    let laptop = f.state.clients["laptop"].client.node().id;
    let other = f.state.clients["other"].client.node().id;
    let agents = f.state.clients["laptop"].agents.clone();
    let followed = f.state.follow.as_ref().unwrap().id();
    let edit = Edit::Rename {
        from: name("laptop"),
        to: name("renamed"),
    };
    f.state
        .handle(Command::Machines(Ask::Edit(edit.clone())))
        .await;
    assert_eq!(outcomes(&drain(&mut f.events)), vec![(edit, Ok(()))]);
    assert!(!f.state.clients.contains_key("laptop"));
    let renamed = &f.state.clients["renamed"];
    assert_eq!(renamed.client.node().id, laptop);
    assert_eq!(renamed.agents, agents);
    assert_eq!(f.state.clients["other"].client.node().id, other);
    assert_eq!(f.state.follow_machine.as_deref(), Some("renamed"));
    assert_eq!(f.state.follow.as_ref().unwrap().id(), followed);
    assert!(
        f.state
            .client("renamed")
            .unwrap()
            .sessions()
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        f.state
            .client("other")
            .unwrap()
            .sessions()
            .await
            .unwrap()
            .is_empty()
    );
    let saved = vessel_client::book::BookFile::open(&f.home.book()).unwrap();
    assert_eq!(
        saved
            .book()
            .default_node
            .as_ref()
            .map(vessel_client::book::NodeName::as_str),
        Some("renamed")
    );
    no_remote_changes(&f.laptop);
    no_remote_changes(&f.other);
}

#[tokio::test]
async fn failed_edits_leave_both_clients_and_the_watched_task_unchanged() {
    let mut f = Fixture::new().await;
    f.watch().await;
    let laptop = f.state.clients["laptop"].client.node().id;
    let other = f.state.clients["other"].client.node().id;
    let followed = f.state.follow.as_ref().unwrap().id();
    let before = f.laptop.calls.lock().unwrap().len();
    for (edit, why) in [
        (
            Edit::Rename {
                from: name("laptop"),
                to: name("other"),
            },
            Failure::Taken,
        ),
        (Edit::Forget(name("missing")), Failure::Missing),
        (Edit::MakeDefault(name("missing")), Failure::Missing),
    ] {
        f.state
            .handle(Command::Machines(Ask::Edit(edit.clone())))
            .await;
        assert_eq!(outcomes(&drain(&mut f.events)), vec![(edit, Err(why))]);
        assert_eq!(f.state.clients["laptop"].client.node().id, laptop);
        assert_eq!(f.state.clients["other"].client.node().id, other);
        assert_eq!(f.state.follow_machine.as_deref(), Some("laptop"));
        assert_eq!(f.state.follow.as_ref().unwrap().id(), followed);
        assert_eq!(f.laptop.calls.lock().unwrap().len(), before);
        assert_eq!(std::fs::read_to_string(f.home.book()).unwrap(), BOOK);
    }
    no_remote_changes(&f.laptop);
    no_remote_changes(&f.other);
}

#[tokio::test]
async fn forgetting_a_watched_machine_closes_only_its_app_connection() {
    let mut f = Fixture::new().await;
    f.watch().await;
    let other = f.state.clients["other"].client.node().id;
    let edit = Edit::Forget(name("laptop"));
    f.state
        .handle(Command::Machines(Ask::Edit(edit.clone())))
        .await;
    assert_eq!(outcomes(&drain(&mut f.events)), vec![(edit, Ok(()))]);
    assert!(!f.state.clients.contains_key("laptop"));
    assert!(f.state.follow.is_none());
    assert_eq!(f.state.follow_machine, None);
    tokio::time::timeout(Duration::from_secs(5), f.laptop.task.take().unwrap())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(f.state.clients["other"].client.node().id, other);
    assert!(
        f.state
            .client("other")
            .unwrap()
            .sessions()
            .await
            .unwrap()
            .is_empty()
    );
    no_remote_changes(&f.laptop);
    no_remote_changes(&f.other);
}

#[tokio::test]
async fn forgetting_another_machine_preserves_the_active_follow() {
    let mut f = Fixture::new().await;
    f.watch().await;
    let followed = f.state.follow.as_ref().unwrap().id();
    f.state
        .handle(Command::Machines(Ask::Edit(Edit::Forget(name("other")))))
        .await;
    assert!(!f.state.clients.contains_key("other"));
    assert_eq!(f.state.follow.as_ref().unwrap().id(), followed);
    assert_eq!(f.state.follow_machine.as_deref(), Some("laptop"));
    assert!(
        f.state
            .client("laptop")
            .unwrap()
            .sessions()
            .await
            .unwrap()
            .is_empty()
    );
    no_remote_changes(&f.laptop);
    no_remote_changes(&f.other);
}
