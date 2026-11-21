use super::super::tests::{context, loaded, rebound, snapshot};
use crate::schedules::{Action, tests::info};

#[test]
fn queued_activation_does_not_rebind_after_a_same_node_reconnect() {
    let (mut page, key) = loaded();
    let intent = page.row(&key).unwrap().intent().unwrap();
    assert!(intent.allows(&page, Action::RunNow));
    page.connected(&key.machine);
    assert!(!intent.is_current(&page));
    assert!(page.act_intent(&intent, Action::RunNow).is_none());
    assert!(page.admitted(rebound(&key.machine)));
    let read = page.due().remove(0);
    page.listed(&read, Ok(snapshot(vec![info()])));
    assert!(!intent.is_current(&page));
    assert!(page.act_intent(&intent, Action::Delete).is_none());
    let current = page.row(&key).unwrap().intent().unwrap();
    assert!(page.act_intent(&current, Action::RunNow).is_some());
}
#[test]
fn an_edited_definition_requires_a_new_confirmation_even_on_the_same_connection() {
    let (mut page, key) = loaded();
    let intent = page.row(&key).unwrap().intent().unwrap();
    page.refresh();
    let read = page.due().remove(0);
    let mut edited = info();
    edited.revision = edited.revision.checked_add(1).unwrap();
    page.listed(&read, Ok(snapshot(vec![edited])));
    assert!(!intent.is_current(&page));
    assert!(page.act_intent(&intent, Action::Delete).is_none());
    let current = page.row(&key).unwrap().intent().unwrap();
    let request = page.act_intent(&current, Action::Delete).unwrap();
    assert_eq!(request.connection(), &context(&key.machine));
    assert!(current.is_current(&page));
    assert!(!current.allows(&page, Action::Delete));
    assert!(page.act_intent(&current, Action::Delete).is_none());
}
#[test]
fn an_offline_cached_row_has_no_intent() {
    let (mut page, key) = loaded();
    page.connected(&key.machine);
    assert!(page.row(&key).unwrap().intent().is_none());
}
