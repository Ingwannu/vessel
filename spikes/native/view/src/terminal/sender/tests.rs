use super::*;
use vessel_proto::terminals::SendTerminal;

#[test]
fn coalesced_paste_never_exceeds_the_existing_node_call_budget() {
    let text = "한".repeat(SendTerminal::MAX_BYTES);
    let mut sender = TerminalSender::default();
    assert_eq!(sender.keys(&text), Taken::Queued);
    assert_eq!(sender.keys("\r"), Taken::Queued);
    sender.resize(80, 24);
    let mut result = String::new();
    let mut sizes = 0;
    while let Some(send) = sender.take() {
        match send {
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
        sender.answered();
    }
    assert_eq!(result, text + "\r");
    assert_eq!(sizes, 1);
}

#[test]
fn a_closed_owner_does_not_acknowledge_unretained_input() {
    let mut sender = TerminalSender::default();
    sender.close();
    assert_ne!(sender.keys("a"), Taken::Queued);
    assert!(sender.take().is_none());
}
