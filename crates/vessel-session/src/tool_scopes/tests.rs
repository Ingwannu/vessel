use super::*;

fn scope(start: u64) -> ToolCallScope {
    ToolCallScope::SessionTurn {
        started: EventSeq::new(start),
    }
}

#[test]
fn reused_ids_become_ambiguous_without_forgetting_unique_old_calls() {
    let mut tracker = ToolScopes::default();
    let old = ToolCallId::new("old");
    let reused = ToolCallId::new("cmd-1");
    tracker.remember(&old, &scope(1));
    tracker.remember(&reused, &scope(1));
    tracker.turn = Some(EventSeq::new(9));
    assert_eq!(tracker.lookup(&old, EventSeq::new(10)), scope(1));
    tracker.remember(&reused, &scope(9));
    assert_eq!(
        tracker.lookup(&reused, EventSeq::new(10)),
        ToolCallScope::Record {
            seq: EventSeq::new(10)
        }
    );
    tracker.remember(&reused, &scope(1));
    assert!(tracker.known[&reused].is_none());
}

#[test]
fn saturation_never_evicts_old_associations_or_creates_a_current_turn_guess() {
    let mut tracker = ToolScopes::default();
    let old = ToolCallId::new("old");
    tracker.remember(&old, &scope(1));
    for index in 0..MAX_TOOL_ASSOCIATIONS {
        tracker.remember(&ToolCallId::new(format!("{index}")), &scope(2));
    }
    assert!(tracker.known.len() <= MAX_TOOL_ASSOCIATIONS);
    assert!(tracker.bytes <= MAX_TOOL_ASSOCIATION_BYTES);
    let unknown = ToolCallId::new("not-retained");
    tracker.remember(&unknown, &scope(8));
    assert_eq!(tracker.lookup(&old, EventSeq::new(10)), scope(1));
    assert_eq!(
        tracker.lookup(&unknown, EventSeq::new(10)),
        ToolCallScope::Record {
            seq: EventSeq::new(10)
        }
    );
    let before = tracker.bytes;
    tracker.remember(
        &ToolCallId::new("x".repeat(MAX_TOOL_ASSOCIATION_BYTES)),
        &scope(8),
    );
    assert_eq!(tracker.bytes, before);
}
