use super::*;
use vessel_proto::terminals::SendTerminal;

#[test]
fn coalesced_paste_never_exceeds_the_existing_node_call_budget() {
    let text = "한".repeat(SendTerminal::MAX_BYTES);
    let mut sender = TerminalSender::default();
    assert_eq!(sender.keys(&text), Taken::Queued);
    assert_eq!(sender.keys("\r"), Taken::Queued);
    assert_eq!(
        sender.resize(super::super::GridSize::STANDARD),
        Taken::Queued
    );
    let mut result = String::new();
    let mut sizes = 0;
    while let Some(send) = sender.take().expect("prepared chunk") {
        let (receipt, outgoing) = send.into_parts();
        match outgoing {
            Outgoing::Keys(text) => {
                assert!(
                    text.len() <= SendTerminal::MAX_BYTES,
                    "oversized serial call"
                );
                result.push_str(&text);
                assert_eq!(
                    sizes, 0,
                    "resize must remain after the entire paste and Enter"
                );
            }
            Outgoing::Resize { cols, rows } => {
                assert_eq!((cols, rows), (80, 24));
                sizes += 1;
            }
        }
        assert_eq!(sender.answered(&receipt), SendSettlement::Settled);
    }
    assert_eq!(result, text + "\r");
    assert_eq!(sizes, 1);
}

#[test]
fn a_closed_owner_does_not_acknowledge_unretained_input() {
    let mut sender = TerminalSender::default();
    let _retired = sender.close();
    assert_ne!(sender.keys("a"), Taken::Queued);
    assert!(sender.take().expect("closed").is_none());
}

#[test]
fn late_duplicate_and_replaced_owner_receipts_cannot_release_current_input() {
    let mut first = TerminalSender::default();
    assert_eq!(first.keys("A"), Taken::Queued);
    let (old, payload) = first.take().expect("first").expect("dispatch").into_parts();
    assert_eq!(payload, Outgoing::Keys("A".to_owned()));
    assert_eq!(first.answered(&old), SendSettlement::Settled);
    assert_eq!(first.keys("B"), Taken::Queued);
    let (current, _) = first
        .take()
        .expect("second")
        .expect("dispatch")
        .into_parts();
    assert_eq!(first.keys("C"), Taken::Queued);
    assert_eq!(first.answered(&old), SendSettlement::Stale);
    assert!(first.busy());
    assert!(first.take().expect("owned").is_none());
    assert_eq!(first.answered(&current), SendSettlement::Settled);
    let (third, payload) = first.take().expect("third").expect("dispatch").into_parts();
    assert_eq!(payload, Outgoing::Keys("C".to_owned()));
    assert_eq!(first.close(), Some(third.clone()));
    assert_eq!(first.answered(&third), SendSettlement::Stale);
    let mut replacement = TerminalSender::default();
    assert_eq!(replacement.keys("A"), Taken::Queued);
    let (fresh, _) = replacement
        .take()
        .expect("replacement")
        .expect("dispatch")
        .into_parts();
    assert_eq!(replacement.answered(&old), SendSettlement::Stale);
    assert_eq!(replacement.answered(&third), SendSettlement::Stale);
    assert!(replacement.busy());
    assert_eq!(replacement.answered(&fresh), SendSettlement::Settled);
    assert_eq!(replacement.answered(&fresh), SendSettlement::Stale);
}

#[test]
fn consuming_a_prefix_does_not_let_new_input_extend_the_old_allocation_forever() {
    let mut sender = TerminalSender::default();
    assert_eq!(
        sender.keys(&"x".repeat(SendTerminal::MAX_BYTES.saturating_mul(2))),
        Taken::Queued
    );
    let (first, _) = sender
        .take()
        .expect("first")
        .expect("dispatch")
        .into_parts();
    assert_eq!(sender.keys("new"), Taken::Queued);
    assert_eq!(
        sender.items.len(),
        2,
        "partially consumed blob is never appended"
    );
    assert_eq!(sender.answered(&first), SendSettlement::Settled);
    let (second, old_tail) = sender.take().expect("tail").expect("dispatch").into_parts();
    assert_eq!(
        old_tail,
        Outgoing::Keys("x".repeat(SendTerminal::MAX_BYTES))
    );
    assert_eq!(sender.answered(&second), SendSettlement::Settled);
    let (_, new) = sender.take().expect("new").expect("dispatch").into_parts();
    assert_eq!(new, Outgoing::Keys("new".to_owned()));
}
