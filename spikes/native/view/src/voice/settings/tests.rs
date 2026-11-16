use super::*;
use crate::{devices::Tickets, voice::Applied};

fn toggle(page: &mut Page, feature: Feature, ticket: Ticket) -> Option<Write> {
    let action = page.toggle_action(feature)?;
    page.toggle(action, ticket)
}

fn snapshot() -> Snapshot {
    Snapshot {
        dictation: DictationSettings::default(),
        conversation: VoiceSettings::default(),
    }
}

#[test]
fn failed_reads_are_not_defaults_and_foreign_or_consumed_replies_are_stale() {
    let node = NodeId::random();
    let mut tickets = Tickets::default();
    let ticket = tickets.next_ticket().unwrap();
    let mut page = Page::reading(node, ticket);
    assert!(matches!(page.status(), Status::Loading));
    assert!(
        toggle(
            &mut page,
            Feature::Conversation,
            tickets.next_ticket().unwrap()
        )
        .is_none()
    );
    assert_eq!(
        page.answered(NodeId::random(), ticket, Ok(snapshot())),
        Applied::Stale
    );
    assert_eq!(
        page.answered(node, ticket, Err(Problem::Refused(ErrorCode::Forbidden))),
        Applied::Current
    );
    assert!(matches!(
        page.status(),
        Status::Failed(Problem::Refused(ErrorCode::Forbidden))
    ));
    assert_eq!(page.answered(node, ticket, Ok(snapshot())), Applied::Stale);
}

#[test]
fn fake_save_refusal_preserves_values_and_only_the_requested_switch_is_sent() {
    let node = NodeId::random();
    let mut tickets = Tickets::default();
    let read = tickets.next_ticket().unwrap();
    let mut page = Page::reading(node, read);
    let before = snapshot();
    assert_eq!(
        page.answered(node, read, Ok(before.clone())),
        Applied::Current
    );
    let write = tickets.next_ticket().unwrap();
    let request = toggle(&mut page, Feature::Conversation, write).unwrap();
    assert_eq!(request.node, node);
    let Change::Conversation(change) = request.change else {
        panic!("conversation switch only");
    };
    assert_eq!(
        change,
        SetVoice {
            switch: Some(DictationChange::Set(DictationSwitch::On)),
            ..SetVoice::default()
        }
    );
    assert!(
        toggle(
            &mut page,
            Feature::Dictation,
            tickets.next_ticket().unwrap()
        )
        .is_none()
    );
    assert!(!page.can_edit(Site::Synthesis));
    assert_eq!(page.answered(node, read, Ok(snapshot())), Applied::Stale);
    assert_eq!(
        page.answered(node, write, Err(Problem::Offline)),
        Applied::Current
    );
    let Status::Ready(confirmed, Editing::Idle, Some(Problem::Offline)) = page.status() else {
        panic!("refusal retains snapshot");
    };
    assert_eq!(confirmed, &before);
    assert_eq!(page.answered(node, write, Ok(snapshot())), Applied::Stale);
}

#[test]
fn future_provider_is_read_only_but_an_independent_known_switch_can_change() {
    let node = NodeId::random();
    let mut tickets = Tickets::default();
    let read = tickets.next_ticket().unwrap();
    let mut values = snapshot();
    values.conversation.tts.provider = DictationProvider::Unknown;
    values.dictation.switch = DictationSwitch::Unknown;
    let mut page = Page::reading(node, read);
    assert_eq!(page.answered(node, read, Ok(values)), Applied::Current);
    assert!(!page.can_edit(Site::Synthesis));
    assert!(page.can_edit(Site::Recognition));
    assert!(
        toggle(
            &mut page,
            Feature::Dictation,
            tickets.next_ticket().unwrap()
        )
        .is_none()
    );
    assert!(
        toggle(
            &mut page,
            Feature::Conversation,
            tickets.next_ticket().unwrap()
        )
        .is_some()
    );
}

#[test]
fn replacing_a_page_fences_an_old_save_even_if_the_host_is_the_same() {
    let node = NodeId::random();
    let mut tickets = Tickets::default();
    let old = tickets.next_ticket().unwrap();
    let current = tickets.next_ticket().unwrap();
    let mut page = Page::reading(node, current);
    assert_eq!(page.answered(node, old, Ok(snapshot())), Applied::Stale);
    assert!(matches!(page.status(), Status::Loading));
    assert_eq!(
        page.answered(node, current, Ok(snapshot())),
        Applied::Current
    );
    for site in [Site::Dictation, Site::Recognition, Site::Synthesis] {
        assert!(page.can_edit(site));
    }
}

/// The fake owns canonical patches, not a second production settings store.
pub(super) struct Node {
    pub(super) id: NodeId,
    pub(super) dictation: SetDictation,
    pub(super) conversation: SetVoice,
}
impl Node {
    pub(super) fn snapshot(&self) -> Snapshot {
        use vessel_proto::dictation::KeyPresence::Absent;
        Snapshot {
            dictation: self.dictation.resolve(Absent),
            conversation: self.conversation.resolve(Absent, Absent),
        }
    }
    pub(super) fn apply(&mut self, request: Write) -> Result<Snapshot, Problem> {
        if request.node != self.id {
            return Err(Problem::Offline);
        }
        match request.change {
            Change::Dictation(change) => {
                self.dictation = change
                    .over(self.dictation.clone())
                    .map_err(|_private| Problem::Refused(ErrorCode::InvalidRequest))?;
            }
            Change::Conversation(change) => {
                self.conversation = change
                    .over(self.conversation.clone())
                    .map_err(|_private| Problem::Refused(ErrorCode::InvalidRequest))?;
            }
        }
        Ok(self.snapshot())
    }
}

#[test]
fn typed_fake_applies_one_switch_and_a_retained_click_cannot_toggle_it_again() {
    let mut node = Node {
        id: NodeId::random(),
        dictation: SetDictation::default(),
        conversation: SetVoice::default(),
    };
    let mut tickets = Tickets::default();
    let read = tickets.next_ticket().unwrap();
    let mut page = Page::reading(node.id, read);
    let before = node.snapshot();
    assert_eq!(
        page.answered(node.id, read, Ok(before.clone())),
        Applied::Current
    );
    let action = page.toggle_action(Feature::Conversation).unwrap();
    assert!(page.toggle(action, read).is_none());
    let save = tickets.next_ticket().unwrap();
    let request = page.toggle(action, save).unwrap();
    let result = node.apply(request).unwrap();
    assert_eq!(result.dictation, before.dictation);
    assert_eq!(result.conversation.stt, before.conversation.stt);
    assert_eq!(result.conversation.tts, before.conversation.tts);
    assert_eq!(result.conversation.switch, DictationSwitch::On);
    assert_eq!(page.answered(node.id, save, Ok(result)), Applied::Current);
    assert!(
        page.toggle(action, tickets.next_ticket().unwrap())
            .is_none()
    );
    assert!(page.toggle_action(Feature::Conversation).is_some());
}
