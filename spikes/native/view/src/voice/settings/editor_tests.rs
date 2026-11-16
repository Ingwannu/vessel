//! A form is a fresh callback scope, never a saved credential or a dialog engine.
use super::*;
use crate::voice::Applied;

fn ready(node: NodeId, read: Ticket) -> Page {
    let mut page = Page::reading(node, read);
    assert_eq!(
        page.answered(
            node,
            read,
            Ok(Snapshot {
                dictation: DictationSettings::default(),
                conversation: VoiceSettings::default(),
            })
        ),
        Applied::Current
    );
    page
}

#[test]
fn each_reopening_is_fresh_and_a_retained_close_cannot_close_its_successor() {
    let node = NodeId::random();
    let mut requests = Tickets::default();
    let mut page = ready(node, requests.next_ticket().unwrap());
    for site in [Site::Dictation, Site::Recognition, Site::Synthesis] {
        let action = page.edit_action(site).unwrap();
        let old = page.open_editor(action).unwrap().unwrap();
        assert_eq!(old.site(), site);
        assert!(page.editor_values(old).is_some());
        assert!(page.open_editor(action).unwrap().is_none());
        assert!(page.edit_action(site).is_none());
        assert!(page.toggle_action(Feature::Conversation).is_none());
        assert_eq!(page.close_editor(old), Applied::Current);
        let next = page.open_editor(action).unwrap().unwrap();
        assert_ne!(old, next);
        assert_eq!(page.close_editor(old), Applied::Stale);
        assert!(page.editor_values(old).is_none());
        assert!(page.editor_values(next).is_some());
        assert_eq!(page.close_editor(next), Applied::Current);
        assert!(page.toggle_action(Feature::Conversation).is_some());
    }
}

#[test]
fn host_or_snapshot_replacement_refuses_old_form_reads_and_close_callbacks() {
    let node = NodeId::random();
    let mut requests = Tickets::default();
    let mut old_page = ready(node, requests.next_ticket().unwrap());
    let old = old_page
        .open_editor(old_page.edit_action(Site::Recognition).unwrap())
        .unwrap()
        .unwrap();
    for next_node in [node, NodeId::random()] {
        let mut current = ready(next_node, requests.next_ticket().unwrap());
        let action = current.edit_action(Site::Recognition).unwrap();
        let next = current.open_editor(action).unwrap().unwrap();
        assert!(current.editor_values(old).is_none());
        assert_eq!(current.close_editor(old), Applied::Stale);
        assert!(current.editor_values(next).is_some());
    }
}
