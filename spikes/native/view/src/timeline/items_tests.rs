//! Direct typed projection fixtures before canonical `AgentEvent` activation.
//! The parent module admits the exact source cursor without a fake wire event.

use super::*;
use chrono::DateTime;
use vessel_proto::timeline::{
    Compaction, CompactionStatus, CompactionTrigger, NotificationLevel, PluginTimelineData,
    PluginTimelineItem, PluginTimelineVersion, TimelineNotification,
};
use vessel_proto::{AgentEvent, RawJson, TokenCount};

fn project(timeline: &mut Timeline, seq: u64, kind: EntryKind) {
    let seq = EventSeq::new(seq);
    timeline.next = seq.next().unwrap();
    timeline.batch.added_from = timeline.entries.len();
    // Record metadata is passed directly to the projection; this HarnessLog
    // discriminant is never run through admission or claimed as a canonical wire item.
    let record = EventRecord {
        seq,
        at: DateTime::UNIX_EPOCH,
        event: SessionEvent::Agent {
            event: AgentEvent::HarnessLog {
                line: String::new(),
            },
        },
    };
    timeline.push(&record, kind);
}

fn compact(
    status: CompactionStatus,
    trigger: Option<CompactionTrigger>,
    tokens: Option<u64>,
) -> EntryKind {
    EntryKind::Compaction(Compaction {
        status,
        trigger,
        pre_tokens: tokens.map(TokenCount::new),
    })
}

#[test]
fn completion_upgrades_earliest_loading_across_other_rows_and_loading_always_appends() {
    let mut timeline = Timeline::default();
    project(
        &mut timeline,
        1,
        compact(
            CompactionStatus::Loading,
            Some(CompactionTrigger::Manual),
            Some(11000),
        ),
    );
    project(
        &mut timeline,
        2,
        EntryKind::Notification(TimelineNotification {
            level: NotificationLevel::Info,
            message: "Between".to_owned(),
        }),
    );
    project(
        &mut timeline,
        3,
        compact(
            CompactionStatus::Loading,
            Some(CompactionTrigger::Auto),
            Some(22000),
        ),
    );
    project(
        &mut timeline,
        4,
        compact(CompactionStatus::Completed, None, None),
    );
    assert_eq!(timeline.entries.len(), 3);
    assert_eq!(timeline.entries[0].seq, EventSeq::new(1));
    assert_eq!(timeline.entries[0].through, EventSeq::new(5));
    assert!(
        matches!(&timeline.entries[0].kind,EntryKind::Compaction(item) if item.status==CompactionStatus::Completed && item.trigger==Some(CompactionTrigger::Manual) && item.pre_tokens==Some(TokenCount::new(11000)))
    );
    assert!(
        matches!(&timeline.entries[2].kind,EntryKind::Compaction(item) if item.status==CompactionStatus::Loading)
    );
    project(
        &mut timeline,
        5,
        compact(CompactionStatus::Completed, None, Some(0)),
    );
    project(
        &mut timeline,
        6,
        compact(CompactionStatus::Loading, None, None),
    );
    assert_eq!(timeline.entries.len(), 4);
    assert!(
        matches!(&timeline.entries[2].kind,EntryKind::Compaction(item) if item.pre_tokens==Some(TokenCount::new(0)))
    );
    assert!(
        matches!(&timeline.entries[3].kind,EntryKind::Compaction(item) if item.status==CompactionStatus::Loading)
    );
}

#[test]
fn abandoned_loading_rows_do_not_absorb_completion_after_rewind() {
    let mut timeline = Timeline::default();
    project(
        &mut timeline,
        1,
        compact(CompactionStatus::Loading, None, None),
    );
    timeline.rewound.push(0..1);
    project(
        &mut timeline,
        2,
        compact(CompactionStatus::Completed, None, None),
    );
    assert_eq!(timeline.entries.len(), 2);
    assert!(
        matches!(&timeline.entries[0].kind,EntryKind::Compaction(item) if item.status==CompactionStatus::Loading)
    );
    assert!(
        matches!(&timeline.entries[1].kind,EntryKind::Compaction(item) if item.status==CompactionStatus::Completed)
    );
}

fn plugin(name: &str, data: &str) -> EntryKind {
    EntryKind::Plugin(PluginTimelineItem {
        plugin: name.parse().unwrap(),
        id: "same-id".parse().unwrap(),
        kind: "panel".parse().unwrap(),
        version: PluginTimelineVersion::new(1).unwrap(),
        data: PluginTimelineData::try_from(RawJson::from_serialize(&data).unwrap()).unwrap(),
    })
}

#[test]
fn plugin_update_identity_is_pair_scoped_and_preserves_the_original_source_anchor() {
    let mut timeline = Timeline::default();
    project(&mut timeline, 1, plugin("first", "old"));
    project(&mut timeline, 2, plugin("second", "other"));
    project(&mut timeline, 7, plugin("first", "new"));
    assert_eq!(timeline.entries.len(), 2);
    assert_eq!(timeline.entries[0].seq, EventSeq::new(1));
    assert_eq!(timeline.entries[0].through, EventSeq::new(8));
    assert_eq!(timeline.index_of(&timeline.entries[0].key), Some(0));
    assert_eq!(timeline.index_of(&timeline.entries[1].key), Some(1));
    let EntryKind::Plugin(item) = &timeline.entries[0].kind else {
        panic!("expected plugin item")
    };
    assert_eq!(item.data.raw().as_text(), "\"new\"");
}
