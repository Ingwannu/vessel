//! The fake feeds the exact boundary snapshots without network or harnesses.
use super::*;
use vessel_proto::{EventSeq, NodeId, SessionId};

struct FakeNode {
    source: Source,
    epochs: crate::connection::ConnectionEpochs,
    snapshot: Snapshot,
}
impl FakeNode {
    fn new(name: &str) -> Self {
        let mut epochs = crate::connection::ConnectionEpochs::default();
        let context = epochs
            .admit(
                crate::connection::MachineKey::from_book(name),
                NodeId::random(),
            )
            .unwrap();
        Self {
            source: Source::new(context).unwrap(),
            epochs,
            snapshot: Snapshot {
                authority: Authority::Operate,
                attention: None,
                terminals: None,
                counts: Counts::new(42, 7, 9),
                entries: Vec::new(),
                extent: Extent::More,
                next: Some(Position::new(1)),
            },
        }
    }
    fn entry(&self, position: u64) -> Entry {
        Entry {
            position: Position::new(position),
            key: Key {
                node: self.source.node(),
                session: SessionId::random(),
                seq: EventSeq::new(position),
            },
            title: None,
            project: None,
            at: Utc::now(),
            read: Read::Unread,
            content: Content::Finished,
        }
    }
    fn read(&self, inbox: &mut Inbox) -> Vec<Key> {
        let ticket = inbox.begin(self.source.node()).unwrap().unwrap();
        inbox.receive(self.source.node(), ticket, Ok(self.snapshot.clone()))
    }
}

#[test]
fn initial_hydration_and_reconnect_are_silent_but_new_attention_is_delivered() {
    let mut fake = FakeNode::new("laptop");
    fake.snapshot.entries.push(fake.entry(1));
    let mut inbox = Inbox::default();
    inbox.connect(fake.source.clone());
    assert!(fake.read(&mut inbox).is_empty());
    assert!(fake.read(&mut inbox).is_empty());
    let new = fake.entry(2);
    fake.snapshot.entries.insert(0, new.clone());
    assert_eq!(fake.read(&mut inbox), vec![new.key]);
    assert!(fake.read(&mut inbox).is_empty());
    inbox.disconnect(fake.source.node());
    fake.snapshot.entries.insert(0, fake.entry(3));
    inbox.connect(fake.source.clone());
    assert!(fake.read(&mut inbox).is_empty());
}

#[test]
fn late_reads_cannot_revive_removed_or_disconnected_nodes() {
    let fake = FakeNode::new("laptop");
    let mut inbox = Inbox::default();
    inbox.connect(fake.source.clone());
    let stale = inbox.begin(fake.source.node()).unwrap().unwrap();
    assert_eq!(inbox.begin(fake.source.node()).unwrap(), None);
    inbox.disconnect(fake.source.node());
    let current = inbox.begin(fake.source.node()).unwrap().unwrap();
    assert_ne!(stale, current);
    assert!(
        inbox
            .receive(fake.source.node(), stale, Ok(fake.snapshot.clone()))
            .is_empty()
    );
    assert!(inbox.nodes().first().unwrap().snapshot.is_none());
    assert!(
        inbox
            .receive(fake.source.node(), current, Ok(fake.snapshot.clone()))
            .is_empty()
    );
    assert_eq!(inbox.nodes().first().unwrap().listing, Listing::Ready);
    let removed = inbox.begin(fake.source.node()).unwrap().unwrap();
    inbox.retain(&[]);
    assert!(
        inbox
            .receive(fake.source.node(), removed, Ok(fake.snapshot))
            .is_empty()
    );
    assert!(inbox.nodes().is_empty());
}

#[test]
fn partial_pages_keep_whole_node_counts_and_node_identity_separates_equal_sources() {
    let mut left = FakeNode::new("laptop");
    let mut right = FakeNode::new("server");
    let entry = left.entry(1);
    let mut twin = entry.clone();
    twin.key.node = right.source.node();
    left.snapshot.entries.push(entry.clone());
    right.snapshot.entries.push(twin.clone());
    let mut inbox = Inbox::default();
    for fake in [&left, &right] {
        inbox.connect(fake.source.clone());
        assert!(fake.read(&mut inbox).is_empty());
    }
    assert_eq!(inbox.counts(), Counts::new(84, 14, 18));
    assert_eq!(inbox.entry(entry.key), Some(&entry));
    assert_eq!(inbox.entry(twin.key), Some(&twin));
    inbox.select(entry.key);
    inbox.retain(&[right.source.node()]);
    assert_eq!(inbox.selected(), None);
    assert_eq!(inbox.counts(), right.snapshot.counts);
}

#[test]
fn malformed_pages_and_failures_keep_last_good_snapshot_without_claiming_ready() {
    let mut fake = FakeNode::new("laptop");
    fake.snapshot.entries.push(fake.entry(1));
    let mut inbox = Inbox::default();
    inbox.connect(fake.source.clone());
    assert!(fake.read(&mut inbox).is_empty());
    let saved = fake.snapshot.clone();
    fake.snapshot
        .entries
        .push(fake.snapshot.entries.first().unwrap().clone());
    assert!(fake.read(&mut inbox).is_empty());
    let node = inbox.nodes().first().unwrap();
    assert_eq!(node.listing, Listing::Failed(Problem::InvalidReply));
    assert_eq!(node.snapshot, Some(saved.clone()));
    let ticket = inbox.begin(fake.source.node()).unwrap().unwrap();
    assert!(
        inbox
            .receive(fake.source.node(), ticket, Err(Problem::Offline))
            .is_empty()
    );
    assert_eq!(
        inbox.nodes().first().unwrap().listing,
        Listing::Failed(Problem::Offline)
    );
    assert_eq!(inbox.nodes().first().unwrap().snapshot, Some(saved));
}

#[test]
fn old_pages_and_marking_unread_do_not_replay_a_notice() {
    let mut fake = FakeNode::new("laptop");
    fake.snapshot.entries.push(fake.entry(10));
    let mut inbox = Inbox::default();
    inbox.connect(fake.source.clone());
    assert!(fake.read(&mut inbox).is_empty());
    fake.snapshot.entries.push(fake.entry(5));
    assert!(fake.read(&mut inbox).is_empty());
    fake.snapshot.entries.clear();
    assert!(fake.read(&mut inbox).is_empty());
    fake.snapshot.entries.push(fake.entry(5));
    assert!(fake.read(&mut inbox).is_empty());
    let mut read = fake.entry(11);
    read.read = Read::Read;
    fake.snapshot.entries.insert(0, read);
    assert!(fake.read(&mut inbox).is_empty());
}

#[test]
fn errors_and_unknown_items_are_retained_without_notifications() {
    for content in [Content::Failed, Content::Unknown] {
        assert!(!content.announces());
        assert!(!content.waiting());
    }
    assert!(Content::InSession.waiting());
    assert!(Content::InSession.announces());
    assert!(Content::Limit.announces());
}

#[test]
fn the_first_new_entry_after_an_empty_hydration_still_announces() {
    let mut fake = FakeNode::new("laptop");
    let mut inbox = Inbox::default();
    inbox.connect(fake.source.clone());
    assert!(fake.read(&mut inbox).is_empty());
    let first = fake.entry(1);
    fake.snapshot.entries.push(first.clone());
    assert_eq!(fake.read(&mut inbox), vec![first.key]);
}

#[test]
fn a_detail_answer_failure_retains_the_request_and_a_late_reply_cannot_replace_selection() {
    let fake = FakeNode::new("laptop");
    let mut inbox = Inbox::default();
    let mut detail = Detail::default();
    let first = fake.entry(1);
    let initial = inbox.ticket().unwrap();
    detail.select(first.key, initial);
    assert_eq!(detail.pending(), Some(Operation::Load));
    assert!(detail.receive(first.key, initial, Ok(Some(first.clone()))));
    let answering = inbox.ticket().unwrap();
    assert_eq!(detail.begin(answering, Operation::Answer), Some(first.key));
    assert_eq!(
        detail.begin(inbox.ticket().unwrap(), Operation::Answer),
        None
    );
    assert!(detail.receive(first.key, answering, Err(Problem::Offline)));
    assert_eq!(detail.entry(), Some(&first));
    assert_eq!(detail.problem(), Some(Problem::Offline));
    let second = fake.entry(2);
    let next = inbox.ticket().unwrap();
    detail.select(second.key, next);
    assert!(!detail.receive(first.key, answering, Ok(None)));
    assert_eq!(detail.key(), Some(second.key));
    assert_eq!(detail.pending(), Some(Operation::Load));
    assert!(detail.receive(second.key, next, Ok(None)));
    assert_eq!(detail.entry(), None);
    assert_eq!(detail.pending(), None);
}

#[test]
fn each_notification_switch_preserves_the_other_channel() {
    use crate::prefs::Alerts;
    use delivery::{Channel, Delivery, Toggle};
    for alerts in [
        Alerts::Off,
        Alerts::System,
        Alerts::Sound,
        Alerts::SystemAndSound,
    ] {
        let before = Delivery::of(alerts);
        let desktop = Delivery::of(before.toggled(Toggle::Desktop));
        assert_eq!(desktop.sound, before.sound);
        assert_ne!(desktop.desktop, before.desktop);
        let sound = Delivery::of(before.toggled(Toggle::Sound));
        assert_eq!(sound.desktop, before.desktop);
        assert_ne!(sound.sound, before.sound);
    }
    assert_eq!(Delivery::of(Alerts::System).sound, Channel::Disabled);
}

#[test]
fn active_source_suppression_is_node_and_session_scoped() {
    use delivery::Visibility;
    let fake = FakeNode::new("laptop");
    let key = fake.entry(1).key;
    assert!(Visibility::Away.permits(key));
    assert!(
        !Visibility::Viewing {
            node: key.node,
            session: key.session
        }
        .permits(key)
    );
    assert!(
        Visibility::Viewing {
            node: NodeId::random(),
            session: key.session
        }
        .permits(key)
    );
    assert!(
        Visibility::Viewing {
            node: key.node,
            session: SessionId::random()
        }
        .permits(key)
    );
}

#[test]
fn a_foreign_node_entry_is_refused_before_it_can_be_drawn_or_announced() {
    let mut fake = FakeNode::new("laptop");
    let foreign = FakeNode::new("server").entry(1);
    fake.snapshot.entries.push(foreign);
    let mut inbox = Inbox::default();
    inbox.connect(fake.source.clone());
    assert!(fake.read(&mut inbox).is_empty());
    assert_eq!(
        inbox.nodes().first().unwrap().listing,
        Listing::Failed(Problem::InvalidReply)
    );
    assert!(inbox.nodes().first().unwrap().snapshot.is_none());
}

#[test]
fn page_expansion_preserves_its_bound_and_does_not_run_beside_an_inflight_read() {
    let fake = FakeNode::new("laptop");
    let mut inbox = Inbox::default();
    inbox.connect(fake.source.clone());
    let ticket = inbox.begin(fake.source.node()).unwrap().unwrap();
    assert_eq!(inbox.expand(fake.source.node()), None);
    assert!(
        inbox
            .receive(fake.source.node(), ticket, Ok(fake.snapshot))
            .is_empty()
    );
    assert_eq!(inbox.expand(fake.source.node()), Some(fake.source.clone()));
    let mut pages = Pages::default();
    for cursor in (1..10_000).rev() {
        pages = pages.more(Position::new(cursor)).unwrap();
        assert_eq!(pages.get(), MAX_PAGES);
    }
    assert_eq!(pages.more(Position::new(1)), None);
    assert_eq!(inbox.newest(fake.source.node()), Some(fake.source.clone()));
    assert_eq!(inbox.nodes()[0].pages.before(), None);
}

#[test]
fn readonly_or_stale_authority_never_enables_answer_controls() {
    use vessel_proto::Scope;
    assert_eq!(Authority::of(&[Scope::Read]), Authority::ReadOnly);
    assert_eq!(Authority::of(&[Scope::Admin]), Authority::ReadOnly);
    assert_eq!(
        Authority::of(&[Scope::Read, Scope::Operate]),
        Authority::Operate
    );
    let mut fake = FakeNode::new("laptop");
    let entry = fake.entry(1);
    fake.snapshot.entries.push(entry.clone());
    let mut inbox = Inbox::default();
    inbox.connect(fake.source.clone());
    assert!(!inbox.may_answer(entry.key));
    assert!(fake.read(&mut inbox).is_empty());
    assert!(inbox.may_answer(entry.key));
    fake.snapshot.authority = Authority::ReadOnly;
    assert!(fake.read(&mut inbox).is_empty());
    assert!(!inbox.may_answer(entry.key));
    fake.snapshot.authority = Authority::Operate;
    assert!(fake.read(&mut inbox).is_empty());
    inbox.disconnect(fake.source.node());
    assert!(!inbox.may_answer(entry.key));
}

#[test]
fn reconnecting_the_same_node_without_an_offline_frame_still_invalidates_older_reads() {
    let mut fake = FakeNode::new("laptop");
    let mut inbox = Inbox::default();
    inbox.connect(fake.source.clone());
    let old = inbox.begin(fake.source.node()).unwrap().unwrap();
    fake.source = Source::new(
        fake.epochs
            .admit(fake.source.context().machine().clone(), fake.source.node())
            .unwrap(),
    )
    .unwrap();
    inbox.connect(fake.source.clone());
    let current = inbox.begin(fake.source.node()).unwrap().unwrap();
    assert!(
        inbox
            .receive(fake.source.node(), old, Ok(fake.snapshot.clone()))
            .is_empty()
    );
    assert!(inbox.nodes().first().unwrap().snapshot.is_none());
    assert!(
        inbox
            .receive(fake.source.node(), current, Ok(fake.snapshot))
            .is_empty()
    );
    assert_eq!(inbox.nodes().first().unwrap().listing, Listing::Ready);
}

#[test]
fn unchanged_polling_and_repeated_failure_do_not_draw_a_frame() {
    let fake = FakeNode::new("idle");
    let mut inbox = Inbox::default();
    inbox.connect(fake.source.clone());
    let id = fake.source.node();
    let ticket = inbox.begin(id).unwrap().unwrap();
    assert!(
        inbox
            .receive_visible(id, ticket, Ok(fake.snapshot.clone()))
            .0
    );
    let ticket = inbox.begin(id).unwrap().unwrap();
    assert!(
        !inbox
            .receive_visible(id, ticket, Ok(fake.snapshot.clone()))
            .0
    );
    let ticket = inbox.begin(id).unwrap().unwrap();
    assert!(inbox.receive_visible(id, ticket, Err(Problem::Offline)).0);
    let ticket = inbox.begin(id).unwrap().unwrap();
    assert!(!inbox.receive_visible(id, ticket, Err(Problem::Offline)).0);
}

#[test]
fn retirement_keeps_form_but_requires_fresh_confirmation_before_actions() {
    let fake = FakeNode::new("draft");
    let entry = fake.entry(1);
    let mut inbox = Inbox::default();
    let mut detail = Detail::default();
    let first = inbox.ticket().unwrap();
    detail.select(entry.key, first);
    assert!(detail.receive(entry.key, first, Ok(Some(entry.clone()))));
    detail.retire();
    assert_eq!(detail.entry(), Some(&entry));
    assert!(!detail.confirmed());
    assert_eq!(
        detail.begin(inbox.ticket().unwrap(), Operation::Answer),
        None
    );
    let second = inbox.ticket().unwrap();
    detail.select(entry.key, second);
    assert_eq!(detail.entry(), Some(&entry));
    assert!(!detail.receive(entry.key, first, Ok(None)));
    assert!(detail.receive(entry.key, second, Err(Problem::Offline)));
    assert!(!detail.confirmed());
    let third = inbox.ticket().unwrap();
    detail.select(entry.key, third);
    assert!(detail.receive(entry.key, third, Ok(Some(entry.clone()))));
    assert_eq!(
        detail.begin(inbox.ticket().unwrap(), Operation::Answer),
        Some(entry.key)
    );
}

#[test]
fn terminal_revisions_share_hydration_retirement_and_once_only_delivery() {
    use vessel_proto::notifications::{
        TerminalActivity, TerminalAttention, TerminalAttentionReason, TerminalAttentionRevision,
    };
    let mut fake = FakeNode::new("terminal");
    let terminal = vessel_proto::TerminalId::random();
    let mut state = TerminalAttention {
        terminal,
        revision: TerminalAttentionRevision::new(1),
        activity: TerminalActivity::Working,
        changed_at: DateTime::UNIX_EPOCH,
    };
    fake.snapshot.terminals = Some(vec![terminals::Row {
        terminal,
        title: None,
        plain_shell: true,
        attention: Some(state),
        failed: false,
    }]);
    let mut inbox = Inbox::default();
    inbox.connect(fake.source.clone());
    assert!(fake.read(&mut inbox).is_empty());
    assert!(inbox.take_terminal_notices().0.is_empty());
    state.revision = state.revision.next().unwrap();
    state.activity = TerminalActivity::Attention {
        reason: TerminalAttentionReason::NeedsInput,
    };
    fake.snapshot.terminals.as_mut().unwrap()[0].attention = Some(state);
    assert!(fake.read(&mut inbox).is_empty());
    assert_eq!(
        inbox.take_terminal_notices().0,
        vec![(fake.source.clone(), state)]
    );
    assert!(fake.read(&mut inbox).is_empty());
    assert!(inbox.take_terminal_notices().0.is_empty());
    inbox.disconnect(fake.source.node());
    state.revision = state.revision.next().unwrap();
    fake.snapshot.terminals.as_mut().unwrap()[0].attention = Some(state);
    inbox.connect(fake.source.clone());
    assert!(fake.read(&mut inbox).is_empty());
    assert!(inbox.take_terminal_notices().0.is_empty());
}

#[test]
fn aliases_of_one_node_keep_one_stable_read_admission_without_double_counting() {
    let mut fake = FakeNode::new("first");
    let second = Source::new(
        fake.epochs
            .admit(
                crate::connection::MachineKey::from_book("second"),
                fake.source.node(),
            )
            .unwrap(),
    )
    .unwrap();
    let mut inbox = Inbox::default();
    inbox.connect(fake.source.clone());
    let sources = inbox.read_routes([second.clone(), fake.source.clone()]);
    assert_eq!(sources, vec![fake.source.clone()]);
    assert!(fake.read(&mut inbox).is_empty());
    assert_eq!(inbox.counts(), fake.snapshot.counts);
    let old_ticket = inbox.begin(fake.source.node()).unwrap().unwrap();
    let replacement = inbox.read_routes([second.clone()]);
    assert_eq!(replacement, vec![second.clone()]);
    inbox.connect(second);
    assert!(
        !inbox
            .receive_visible(fake.source.node(), old_ticket, Ok(fake.snapshot))
            .0
    );
}

#[test]
fn bounded_background_admission_rotates_every_machine_to_the_head() {
    let sources: Vec<_> = ["one", "two", "three", "four"]
        .into_iter()
        .map(|name| FakeNode::new(name).source)
        .collect();
    let mut inbox = Inbox::default();
    for expected in &sources {
        let order = inbox.poll_routes(sources.clone());
        assert_eq!(order.first(), Some(expected));
        assert_eq!(order.len(), sources.len());
        assert!(sources.iter().all(|source| order.contains(source)));
    }
    assert_eq!(inbox.poll_routes(sources.clone()).first(), sources.first());
    assert!(inbox.poll_routes(Vec::new()).is_empty());
    assert_eq!(inbox.poll_routes(sources.clone()).first(), sources.first());
}

#[test]
fn shared_marks_allow_readonly_people_but_reject_retained_stale_snapshots() {
    let mut fake = FakeNode::new("laptop");
    let entry = fake.entry(1);
    fake.snapshot.entries.push(entry.clone());
    fake.snapshot.authority = Authority::ReadOnly;
    let mut inbox = Inbox::default();
    inbox.connect(fake.source.clone());
    assert!(!inbox.may_mark(entry.key));
    assert!(fake.read(&mut inbox).is_empty());
    assert!(inbox.may_mark(entry.key));
    assert!(!inbox.may_answer(entry.key));
    let ticket = inbox.begin(fake.source.node()).unwrap().unwrap();
    let _notices = inbox.receive(fake.source.node(), ticket, Err(Problem::Offline));
    assert!(inbox.entry(entry.key).is_some());
    assert!(!inbox.may_mark(entry.key));
    assert!(fake.read(&mut inbox).is_empty());
    assert!(inbox.may_mark(entry.key));
    inbox.disconnect(fake.source.node());
    assert!(!inbox.may_mark(entry.key));
}
