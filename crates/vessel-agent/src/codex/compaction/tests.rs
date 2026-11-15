use super::*;
use serde_json::json;
fn item(state: &mut Compactions, started: bool, turn: &str, id: &str) -> Vec<AgentEvent> {
    state
        .observe(
            "root",
            if started {
                "item/started"
            } else {
                "item/completed"
            },
            &json!({"threadId":"root","turnId":turn,"item":{"type":"contextCompaction","id":id}}),
        )
        .unwrap()
}
fn notification(state: &mut Compactions, turn: &str) -> Vec<AgentEvent> {
    state
        .observe(
            "root",
            "thread/compacted",
            &json!({"threadId":"root","turnId":turn}),
        )
        .unwrap()
}
#[test]
fn both_native_completion_orders_settle_one_loading_row_once() {
    for notice_first in [true, false] {
        let mut state = Compactions::default();
        assert_eq!(
            item(&mut state, true, "turn", "item"),
            [event(CompactionStatus::Loading)]
        );
        assert!(item(&mut state, true, "turn", "item").is_empty());
        let (first, second) = if notice_first {
            (
                notification(&mut state, "turn"),
                item(&mut state, false, "turn", "item"),
            )
        } else {
            (
                item(&mut state, false, "turn", "item"),
                notification(&mut state, "turn"),
            )
        };
        assert_eq!(first, [event(CompactionStatus::Completed)]);
        assert!(second.is_empty());
        assert!(item(&mut state, false, "turn", "item").is_empty());
    }
}
#[test]
fn owned_turn_end_closes_its_rows_and_child_turns_do_not_enter_root() {
    let mut state = Compactions::default();
    assert!(state.observe("root", "item/started", &json!({"threadId":"child","turnId":"turn","item":{"type":"contextCompaction","id":"child-item"}})).unwrap().is_empty());
    let _started = item(&mut state, true, "turn", "a");
    let _other = item(&mut state, true, "other", "a");
    assert_eq!(state.finish("turn"), [event(CompactionStatus::Completed)]);
    assert!(state.finish("turn").is_empty());
    assert_eq!(state.finish("other"), [event(CompactionStatus::Completed)]);
}
#[test]
fn full_retained_state_still_completes_existing_without_eviction_or_new_loading() {
    let mut state = Compactions::default();
    for index in 0..MAX_COMPACTION_OBSERVATIONS {
        assert_eq!(item(&mut state, true, "turn", &format!("{index}")).len(), 1);
    }
    assert!(item(&mut state, true, "turn", "overflow").is_empty());
    assert_eq!(state.finish("turn").len(), MAX_COMPACTION_OBSERVATIONS);
    assert_eq!(state.admitted, MAX_COMPACTION_OBSERVATIONS);
    assert!(item(&mut state, true, "turn", "overflow").is_empty());
    assert!(item(&mut state, false, "turn", "0").is_empty());
}
