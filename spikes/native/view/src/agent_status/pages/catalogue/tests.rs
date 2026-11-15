//! Catalog correlation is the existing page lifecycle, not a second store.
use super::*;
use crate::agent_status::pages::{Administration, StartProblem};
use crate::agent_status::{
    catalogue::Read,
    pages::tests::{inventory, key},
};

fn fetched() -> Catalogue {
    Catalogue {
        fetched_at: Some(chrono::DateTime::UNIX_EPOCH),
        ..Catalogue::default()
    }
}
fn loaded(pages: &mut Pages<&'static str>, machine: &MachineKey) -> NodeId {
    let inventory = inventory(Administration::Supported);
    let node = inventory.node;
    let ticket = pages.begin(machine.clone(), Operation::Discovery).unwrap();
    assert!(pages.loaded(machine, ticket, inventory));
    node
}
#[test]
fn catalogue_keeps_facts_for_refresh_and_its_failure_only() {
    let mut pages = Pages::default();
    let machine = key("one");
    let node = loaded(&mut pages, &machine);
    assert!(matches!(
        pages.get("one").unwrap().catalogue(),
        Read::NotRequested
    ));
    let ticket = pages
        .begin(machine.clone(), Operation::Catalogue { node })
        .unwrap();
    assert!(matches!(
        pages.get("one").unwrap().catalogue(),
        Read::Pending { previous: None }
    ));
    assert!(pages.catalogue_loaded(&machine, ticket, node, fetched()));
    assert!(
        matches!(pages.get("one").unwrap().catalogue(), Read::Ready(copy) if copy.fetched_at.is_some())
    );
    let refresh = pages
        .begin(machine.clone(), Operation::Catalogue { node })
        .unwrap();
    assert!(
        matches!(pages.get("one").unwrap().catalogue(), Read::Pending { previous: Some(copy) } if copy.fetched_at.is_some())
    );
    pages.failed(&machine, refresh, "registry failure");
    assert!(matches!(
        pages.get("one").unwrap().catalogue(),
        Read::Failed {
            error: &"registry failure",
            previous: Some(_)
        }
    ));
    let discovery = pages.begin(machine.clone(), Operation::Discovery).unwrap();
    pages.failed(&machine, discovery, "other failure");
    assert!(matches!(
        pages.get("one").unwrap().catalogue(),
        Read::Ready(_)
    ));
}
#[test]
fn catalogue_ack_requires_book_ticket_operation_and_both_node_identities() {
    let mut pages = Pages::default();
    let machine = key("one");
    let node = loaded(&mut pages, &machine);
    let other = key("two");
    let _other_node = loaded(&mut pages, &other);
    let old = pages
        .begin(machine.clone(), Operation::Catalogue { node })
        .unwrap();
    pages.failed(&machine, old, "first failure");
    let current = pages
        .begin(machine.clone(), Operation::Catalogue { node })
        .unwrap();
    assert!(!pages.catalogue_loaded(&machine, old, node, fetched()));
    assert!(!pages.catalogue_loaded(&other, current, node, fetched()));
    let different: NodeId = "00000000-0000-4000-8000-000000000002".parse().unwrap();
    assert!(!pages.catalogue_loaded(&machine, current, different, fetched()));
    assert_eq!(pages.get("one").unwrap().pending().unwrap().id, current);
    assert!(pages.catalogue_loaded(&machine, current, node, fetched()));
    assert!(!pages.catalogue_loaded(&machine, current, node, fetched()));
    let discovery = pages.begin(machine.clone(), Operation::Discovery).unwrap();
    assert!(!pages.catalogue_loaded(&machine, discovery, node, fetched()));
    pages.failed(&machine, discovery, "finish discovery");
    let wrong = pages
        .begin(machine.clone(), Operation::Catalogue { node: different })
        .unwrap();
    assert!(!pages.catalogue_loaded(&machine, wrong, different, fetched()));
    assert_eq!(pages.get("one").unwrap().pending().unwrap().id, wrong);
}
#[test]
fn new_physical_inventory_cannot_display_the_previous_nodes_registry() {
    let mut pages = Pages::default();
    let machine = key("one");
    let node = loaded(&mut pages, &machine);
    let ticket = pages
        .begin(machine.clone(), Operation::Catalogue { node })
        .unwrap();
    assert!(pages.catalogue_loaded(&machine, ticket, node, fetched()));
    let ticket = pages.begin(machine.clone(), Operation::Discovery).unwrap();
    let mut changed = inventory(Administration::Supported);
    changed.node = "00000000-0000-4000-8000-000000000002".parse().unwrap();
    assert!(pages.loaded(&machine, ticket, changed));
    assert!(matches!(
        pages.get("one").unwrap().catalogue(),
        Read::NotRequested
    ));
    let ticket = pages.begin(machine.clone(), Operation::Discovery).unwrap();
    assert!(pages.loaded(&machine, ticket, inventory(Administration::Supported)));
    assert!(matches!(
        pages.get("one").unwrap().catalogue(),
        Read::NotRequested
    ));
}
#[test]
fn registry_reads_share_busy_and_never_reused_tickets() {
    let mut pages = Pages::default();
    let machine = key("one");
    let node = loaded(&mut pages, &machine);
    let ticket = pages
        .begin(machine.clone(), Operation::Catalogue { node })
        .unwrap();
    assert_eq!(
        pages.begin(machine.clone(), Operation::Catalogue { node }),
        Err(StartProblem::Busy)
    );
    pages.failed(&machine, ticket, "read failed");
    let next = pages.begin(machine, Operation::Discovery).unwrap();
    assert_eq!(ticket.next(), Some(next));
}

#[test]
fn actual_reconnect_invalidates_same_id_reads_without_reusing_the_ticket() {
    let mut pages = Pages::default();
    let machine = key("one");
    let node = loaded(&mut pages, &machine);
    let ticket = pages
        .begin(machine.clone(), Operation::Catalogue { node })
        .unwrap();
    assert!(pages.connected(&machine, node));
    assert!(pages.get("one").unwrap().inventory().is_none());
    assert!(pages.get("one").unwrap().pending().is_none());
    assert!(!pages.catalogue_loaded(&machine, ticket, node, fetched()));
    let next = pages.begin(machine.clone(), Operation::Discovery).unwrap();
    assert_eq!(ticket.next(), Some(next));
    assert!(!pages.loaded(&machine, ticket, inventory(Administration::Supported)));
    assert!(pages.get("one").unwrap().inventory().is_none());
    assert!(pages.loaded(&machine, next, inventory(Administration::Supported)));
    assert!(matches!(
        pages.get("one").unwrap().catalogue(),
        Read::NotRequested
    ));
    assert!(!pages.connected(&key("unrelated"), node));
    assert!(pages.get("one").unwrap().inventory().is_some());
}

#[test]
fn reconnect_keeps_a_known_same_nodes_interrupted_output_but_never_another_nodes() {
    use crate::agent_status::pages::{
        InstallProgress,
        tests::{install, job, output},
    };
    let mut pages = Pages::default();
    let machine = key("one");
    let node = loaded(&mut pages, &machine);
    let ticket = pages
        .begin(machine.clone(), Operation::Install(install()))
        .unwrap();
    pages.installing(&machine, ticket, job());
    pages.output(&machine, ticket, &output());
    assert!(pages.connected(&machine, node));
    let InstallProgress::Unavailable(log) = pages.get("one").unwrap().install() else {
        panic!("the known same-node log was discarded or still claims to be followed")
    };
    assert_eq!(log.transcript.text(), "output");
    pages.output(&machine, ticket, &output());
    assert!(!pages.finished(
        &machine,
        ticket,
        crate::agent_status::install::Completion::Succeeded
    ));
    let read = pages.begin(machine.clone(), Operation::Discovery).unwrap();
    assert!(pages.loaded(&machine, read, inventory(Administration::Supported)));
    let different = "00000000-0000-4000-8000-000000000002".parse().unwrap();
    assert!(pages.connected(&machine, different));
    assert!(matches!(
        pages.get("one").unwrap().install(),
        InstallProgress::None
    ));
}
#[test]
fn same_node_reconnect_keeps_navigation_only_until_fresh_definitions_refute_it() {
    use vessel_proto::HarnessRef;
    let mut pages = Pages::default();
    let machine = key("one");
    let node = loaded(&mut pages, &machine);
    let agent = HarnessRef::Added("example".parse().unwrap());
    pages.select("one", Some(agent.clone()));
    assert!(pages.connected(&machine, node));
    assert_eq!(pages.get("one").unwrap().selected(), Some(&agent));
    let ticket = pages.begin(machine.clone(), Operation::Discovery).unwrap();
    assert!(pages.loaded(&machine, ticket, inventory(Administration::Supported)));
    assert!(pages.get("one").unwrap().selected().is_none());
}

#[test]
fn install_completion_refresh_cannot_erase_the_finished_same_node_log() {
    use crate::agent_status::{
        install::Completion,
        pages::{
            InstallProgress,
            tests::{install, job, output},
        },
    };
    let mut pages = Pages::default();
    let machine = key("one");
    let node = loaded(&mut pages, &machine);
    let ticket = pages
        .begin(machine.clone(), Operation::Install(install()))
        .unwrap();
    pages.installing(&machine, ticket, job());
    pages.output(&machine, ticket, &output());
    assert!(pages.finished(&machine, ticket, Completion::Failed(Some(7))));
    assert!(pages.get("one").unwrap().inventory().is_none());
    assert!(pages.connected(&machine, node));
    let InstallProgress::Finished { log, completion } = pages.get("one").unwrap().install() else {
        panic!("the completion-triggered reconnect erased the finished log")
    };
    assert_eq!(*completion, Completion::Failed(Some(7)));
    assert_eq!(log.transcript.text(), "output");
}
