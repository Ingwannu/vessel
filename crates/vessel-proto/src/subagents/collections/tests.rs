use super::*;
use crate::subagents::{ChildActivity, ProviderObservation, ProviderText};
use crate::{AgentEvent, ContentBlock, ProtocolVersion};

fn descriptor(id: &str) -> ChildDescriptor {
    ChildDescriptor {
        id: id.parse().unwrap(),
        parent: None,
        title: None,
        description: None,
        subtitle: None,
        status: None,
        created_at: None,
        updated_at: None,
    }
}
fn record(seq: u64) -> EventRecord {
    EventRecord {
        seq: EventSeq::new(seq),
        at: chrono::Utc::now(),
        event: SessionEvent::Agent {
            event: AgentEvent::MessageChunk {
                content: ContentBlock::Text {
                    text: "child".into(),
                },
            },
        },
    }
}

#[test]
fn descriptor_count_identity_and_actual_escaped_bytes_are_bounded() {
    assert!(ChildDescriptors::try_from(vec![descriptor("one"), descriptor("one")]).is_err());
    let too_many: Vec<_> = (0..=bounds::MAX_CHILDREN)
        .map(|i| descriptor(&format!("id{i}")))
        .collect();
    assert_eq!(
        ChildDescriptors::try_from(too_many.clone()),
        Err(InvalidChildCollection::Count)
    );
    assert!(
        serde_json::from_str::<ChildDescriptors>(&serde_json::to_string(&too_many).unwrap())
            .is_err()
    );
    let mut rows: Vec<_> = (0..bounds::MAX_CHILDREN)
        .map(|i| descriptor(&format!("id{i}")))
        .collect();
    for row in &mut rows {
        row.description = Some("\t".repeat(ProviderText::MAX_BYTES).parse().unwrap());
    }
    assert_eq!(
        ChildDescriptors::try_from(rows.clone()),
        Err(InvalidChildCollection::Bytes)
    );
    assert!(
        serde_json::from_str::<ChildDescriptors>(&serde_json::to_string(&rows).unwrap()).is_err()
    );
    let actual = serde_json::to_vec(&vec![descriptor("한글\"escaped")]).unwrap();
    assert_eq!(
        serialized_bytes(&vec![descriptor("한글\"escaped")], actual.len()).unwrap(),
        actual.len()
    );
    assert_eq!(
        serialized_bytes(&vec![descriptor("한글\"escaped")], actual.len() - 1),
        Err(InvalidChildCollection::Bytes)
    );
}

#[test]
fn cyclic_and_excessive_nesting_are_refused_but_missing_parents_remain_honest() {
    let mut a = descriptor("a");
    let mut b = descriptor("b");
    a.parent = Some(b.id.clone());
    b.parent = Some(a.id.clone());
    assert_eq!(
        ChildDescriptors::try_from(vec![a.clone(), b]),
        Err(InvalidChildCollection::Identity)
    );
    assert!(ChildDescriptors::try_from(vec![a]).is_ok());
    let rows: Vec<_> = (0..=bounds::MAX_CHILD_DEPTH)
        .map(|i| {
            let mut row = descriptor(&format!("child{i}"));
            if i > 0 {
                row.parent = Some(format!("child{}", i - 1).parse().unwrap());
            }
            row
        })
        .collect();
    assert_eq!(
        ChildDescriptors::try_from(rows),
        Err(InvalidChildCollection::Identity)
    );
}

#[test]
fn child_activity_has_no_recursive_wrapper_or_managed_control_rights() {
    let control = AgentEvent::Ready {
        harness_session: crate::HarnessSessionId::new("native"),
        abilities: None,
    };
    assert!(ChildActivity::try_from(control.clone()).is_err());
    assert!(
        serde_json::from_str::<ChildActivity>(&serde_json::to_string(&control).unwrap()).is_err()
    );
    let recursive = AgentEvent::ProviderChild(ProviderObservation::Supported);
    assert!(ChildActivity::try_from(recursive.clone()).is_err());
    assert!(
        serde_json::from_str::<ChildActivity>(&serde_json::to_string(&recursive).unwrap()).is_err()
    );
    assert!(
        ChildActivity::try_from(AgentEvent::from(
            ChildActivity::try_from(AgentEvent::HarnessLog {
                line: "fact".into()
            })
            .unwrap()
        ))
        .is_ok()
    );
}

#[test]
fn child_pages_keep_sparse_parent_sequence_and_reject_control_or_reordering() {
    let rows = vec![record(3), record(17)];
    let valid = ChildRecords::try_from(rows.clone()).unwrap();
    assert_eq!(valid.as_slice(), rows);
    assert_eq!(
        serde_json::from_str::<ChildRecords>(&serde_json::to_string(&valid).unwrap()).unwrap(),
        valid
    );
    assert_eq!(
        ChildRecords::try_from(vec![record(17), record(3)]),
        Err(InvalidChildCollection::Timeline)
    );
    assert_eq!(
        ChildRecords::try_from(vec![record(3), record(3)]),
        Err(InvalidChildCollection::Timeline)
    );
    let mut control = record(1);
    control.event = SessionEvent::Woken;
    assert_eq!(
        ChildRecords::try_from(vec![control]),
        Err(InvalidChildCollection::Timeline)
    );
    let rows: Vec<_> = (0..=u64::from(bounds::MAX_TIMELINE_ITEMS))
        .map(record)
        .collect();
    assert_eq!(
        ChildRecords::try_from(rows),
        Err(InvalidChildCollection::Count)
    );
}

#[test]
fn pending_provider_events_are_withheld_without_losing_parent_page_cursors() {
    let mut original = record(7);
    original.event = SessionEvent::Agent {
        event: AgentEvent::ProviderChild(ProviderObservation::Descriptor {
            descriptor: descriptor("opaque-child"),
        }),
    };
    assert_eq!(
        original.event.required_protocol(),
        super::super::SUBAGENT_RELATIONS
    );
    let page = crate::EventPage {
        records: vec![original.clone()],
        next: EventSeq::new(11),
        at_end: true,
    };
    for peer in [ProtocolVersion::new(2), crate::PROTOCOL_VERSION] {
        let projected = page.for_protocol(peer).unwrap();
        assert_eq!(projected.next, page.next);
        assert!(projected.at_end);
        assert_eq!(projected.records[0].seq, original.seq);
        assert_eq!(projected.records[0].at, original.at);
        assert!(
            !serde_json::to_string(&projected)
                .unwrap()
                .contains("opaque-child")
        );
    }
}
