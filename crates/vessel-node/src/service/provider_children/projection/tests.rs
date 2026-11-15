use super::*;
use vessel_proto::{
    ContentBlock,
    subagents::{ChildActivity, ChildLimit, ChildStatus},
};

fn child(id: &str) -> ChildDescriptor {
    identity(id.parse().unwrap())
}
fn record(seq: u64, observation: ProviderObservation) -> EventRecord {
    EventRecord {
        seq: EventSeq::new(seq),
        at: chrono::Utc::now(),
        event: SessionEvent::Agent {
            event: AgentEvent::ProviderChild(observation),
        },
    }
}
fn activity(seq: u64, id: &str) -> EventRecord {
    record(
        seq,
        ProviderObservation::Activity {
            child: id.parse().unwrap(),
            activity: Box::new(
                ChildActivity::try_from(AgentEvent::MessageChunk {
                    content: ContentBlock::Text {
                        text: format!("reply{seq}"),
                    },
                })
                .unwrap(),
            ),
        },
    )
}
fn request(cursor: Option<ChildCursor>, limit: u32) -> ReadProviderChild {
    ReadProviderChild {
        session: vessel_proto::SessionId::random(),
        child: "child".parse().unwrap(),
        cursor,
        limit: ChildLimit::try_from(limit).unwrap(),
    }
}
#[test]
fn absent_evidence_is_missing_and_explicit_empty_support_is_complete() {
    let absent = Projection::new(None, EventSeq::ZERO, EventSeq::ZERO)
        .list()
        .unwrap();
    assert_eq!(absent.history, ChildHistory::Missing);
    assert!(absent.children.as_slice().is_empty());
    let mut supported = Projection::new(None, EventSeq::ZERO, EventSeq::new(1));
    assert!(supported.observe(record(0, ProviderObservation::Supported)));
    assert_eq!(supported.list().unwrap().history, ChildHistory::Complete);
}
#[test]
fn descriptors_keep_unknown_fields_and_actual_lifecycle_updates() {
    let mut p = Projection::new(None, EventSeq::ZERO, EventSeq::new(3));
    let mut d = child("child");
    d.status = Some(ChildStatus::Running);
    p.observe(record(
        0,
        ProviderObservation::Descriptor {
            descriptor: d.clone(),
        },
    ));
    d.status = Some(ChildStatus::Completed);
    p.observe(record(1, ProviderObservation::Descriptor { descriptor: d }));
    let rows = p.list().unwrap();
    assert_eq!(rows.children.as_slice().len(), 1);
    let d = &rows.children.as_slice()[0];
    assert_eq!(d.status, Some(ChildStatus::Completed));
    assert_eq!(d.created_at, None);
    assert_eq!(d.title, None);
}
#[test]
fn child_timeline_never_mixes_siblings_and_keeps_sparse_parent_positions() {
    let mut p = Projection::new(Some(request(None, 2)), EventSeq::ZERO, EventSeq::new(20));
    for (seq, id) in [(0, "child"), (3, "sibling"), (7, "child"), (19, "child")] {
        p.observe(activity(seq, id));
    }
    p.finish(true);
    let page = p.page().unwrap();
    assert_eq!(
        page.records
            .as_slice()
            .iter()
            .map(|r| r.seq.get())
            .collect::<Vec<_>>(),
        vec![7, 19]
    );
    assert_eq!(page.next, EventSeq::new(20));
    assert!(page.at_end);
}
#[test]
fn forward_limit_stops_at_a_real_parent_continuation() {
    let mut p = Projection::new(
        Some(request(Some(ChildCursor::After(EventSeq::new(2))), 1)),
        EventSeq::new(2),
        EventSeq::new(20),
    );
    assert!(!p.observe(activity(7, "child")));
    let page = p.page().unwrap();
    assert_eq!(page.next, EventSeq::new(8));
    assert!(!page.at_end);
    assert!(
        matches!(page.history,ChildHistory::Partial {reason:ChildGap::Retention,from} if from==EventSeq::new(7))
    );
}
#[test]
fn gaps_and_descriptor_saturation_are_honest_and_never_evict_existing_children() {
    let mut p = Projection::new(None, EventSeq::ZERO, EventSeq::new(999));
    for i in 0..=vessel_proto::subagents::bounds::MAX_CHILDREN {
        p.observe(record(
            u64::try_from(i).unwrap(),
            ProviderObservation::Descriptor {
                descriptor: child(&format!("id{i}")),
            },
        ));
    }
    let list = p.list().unwrap();
    assert_eq!(
        list.children.as_slice().len(),
        vessel_proto::subagents::bounds::MAX_CHILDREN
    );
    assert_eq!(list.children.as_slice()[0].id.as_str(), "id0");
    assert!(matches!(
        list.history,
        ChildHistory::Partial {
            reason: ChildGap::IngestionBudget,
            ..
        }
    ));
    let mut p = Projection::new(None, EventSeq::new(50), EventSeq::new(90));
    p.observe(record(
        50,
        ProviderObservation::Gap {
            reason: ChildGap::Nesting,
        },
    ));
    assert!(
        matches!(p.list().unwrap().history,ChildHistory::Partial {reason:ChildGap::Nesting,from} if from==EventSeq::new(50))
    );
}
