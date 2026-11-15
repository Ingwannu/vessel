//! Portable admission races: canonical context, never alias/node inference.
use super::tests::{context, key, loaded, now, rebound, snapshot};
use super::*;
use crate::connection::{ConnectionEpochs, MachineKey as AdmittedKey};
use crate::schedules::{Action, tests::info};

#[test]
fn absent_or_legacy_admission_never_dispatches_a_read() {
    let mut page = Page::default();
    page.machines(&[key("laptop")]);
    assert!(page.due(now()).is_empty());
    let mut machine = crate::Machine::unreachable("laptop".into(), "synthetic");
    machine.node = Some(context(&key("laptop")).node());
    let fleet = crate::Fleet {
        machines: vec![machine],
        read_at: Some(now()),
    };
    assert!(page.admissions(&fleet));
    assert_eq!(page.body(), Body::Failed);
    assert_eq!(page.problems(), [(&key("laptop"), &Failure::Offline)]);
    assert!(page.due(now()).is_empty());
    assert!(!page.admitted(context(&key("other"))));
}
#[test]
fn another_node_reusing_the_alias_clears_old_rows_and_refuses_late_read_and_write() {
    let (mut page, row) = loaded();
    let old_write = page.act(&row, Action::RunNow).unwrap();
    page.refresh();
    let old_read = page.due(now()).remove(0);
    let mut epochs = ConnectionEpochs::default();
    let replacement = epochs
        .admit(
            AdmittedKey::from_book("laptop"),
            "00000000-0000-4000-8000-000000000002".parse().unwrap(),
        )
        .unwrap();
    assert!(page.admitted(replacement.clone()));
    assert!(page.visible_rows().is_empty());
    assert!(page.row(&row).is_none());
    assert!(!page.accept_list(&old_read, Ok(snapshot(vec![info()]))));
    page.written(&old_write, Ok(WriteValue::Deleted));
    assert!(page.visible_rows().is_empty());
    let current = page.due(now()).remove(0);
    assert_eq!(current.connection(), &replacement);
    assert!(current.ticket > old_read.ticket);
    let mut new_info = info();
    new_info.definition.name = Some("Replacement machine data".parse().unwrap());
    assert!(page.accept_list(&current, Ok(snapshot(vec![new_info.clone()]))));
    assert_eq!(page.row(&row).unwrap().schedule, &new_info);
    page.written(&old_write, Ok(WriteValue::Deleted));
    assert_eq!(page.row(&row).unwrap().schedule, &new_info);
}
#[test]
fn same_node_new_epoch_keeps_cached_facts_but_blocks_actions_until_admitted() {
    let (mut page, row) = loaded();
    let old = page.act(&row, Action::RunNow).unwrap();
    page.connected(&row.machine);
    assert!(page.row(&row).is_some());
    assert!(page.row(&row).unwrap().menu_entries().is_empty());
    assert!(page.due(now()).is_empty());
    let fresh = rebound(&row.machine);
    assert_eq!(fresh.node(), old.connection().node());
    assert_ne!(fresh.epoch(), old.connection().epoch());
    assert!(page.admitted(fresh.clone()));
    assert!(page.act(&row, Action::RunNow).is_none());
    let read = page.due(now()).remove(0);
    assert_eq!(read.connection(), &fresh);
    page.listed(&read, Ok(snapshot(vec![info()])));
    page.written(&old, Ok(WriteValue::Deleted));
    assert!(page.row(&row).is_some());
    assert_eq!(
        page.row(&row).unwrap().mutation.failure(),
        Some(&Failure::Unconfirmed)
    );
}
#[test]
fn a_forged_context_echo_cannot_release_the_expected_tickets_capacity() {
    let mut page = Page::default();
    tests::bound(&mut page, &[key("laptop")]);
    let original = page.due(now()).remove(0);
    let wrong = Request::on(rebound(&key("laptop")), original.ticket, original.ask);
    assert!(!page.accept_list(&wrong, Ok(snapshot(vec![info()]))));
    assert_eq!(page.pending(), 1);
    assert!(page.visible_rows().is_empty());
    assert!(page.accept_list(&original, Ok(snapshot(vec![info()]))));
    assert_eq!(page.pending(), 0);
}
