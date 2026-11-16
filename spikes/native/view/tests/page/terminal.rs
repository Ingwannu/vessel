//! Terminal transport ordering and the existing resize/frame rules.
use crate::support::{EN, Log};
use vessel_app_view::is_web_url;
use vessel_app_view::terminal::{
    GridSize, MAX_PENDING, Outgoing, ResizeRule, SendReceipt, SendSettlement, Taken, TerminalLog,
    TerminalSender,
};
use vessel_proto::terminals::SendTerminal;

fn keys(text: &str) -> Outgoing {
    Outgoing::Keys(text.to_owned())
}
fn next(sender: &mut TerminalSender) -> (SendReceipt, Outgoing) {
    sender
        .take()
        .expect("bounded preparation")
        .expect("queued dispatch")
        .into_parts()
}
fn size(cols: u16, rows: u16) -> GridSize {
    GridSize::new(cols, rows).expect("valid size")
}

#[test]
fn is_sent_one_call_at_a_time_in_the_order_typed() {
    let mut sender = TerminalSender::default();
    assert_eq!(sender.resize(size(120, 40)), Taken::Queued);
    let (resize, outgoing) = next(&mut sender);
    assert_eq!(
        outgoing,
        Outgoing::Resize {
            cols: 120,
            rows: 40
        }
    );
    assert_eq!(sender.keys("pasted line"), Taken::Queued);
    assert_eq!(sender.keys("\r"), Taken::Queued);
    assert_eq!(sender.take().expect("lane"), None, "one in flight");
    assert_eq!(sender.answered(&resize), SendSettlement::Settled);
    let (input, outgoing) = next(&mut sender);
    assert_eq!(outgoing, keys("pasted line\r"));
    assert_eq!(sender.answered(&input), SendSettlement::Settled);
    assert_eq!(sender.take().expect("drained"), None);
    assert!(!sender.busy());
}

#[test]
fn keeps_keys_and_sizes_in_order_the_latest_size_only() {
    let mut sender = TerminalSender::default();
    assert_eq!(sender.keys("a"), Taken::Queued);
    let (mut receipt, first) = next(&mut sender);
    let mut sent = vec![first];
    assert_eq!(sender.keys("b"), Taken::Queued);
    assert_eq!(sender.resize(size(80, 24)), Taken::Queued);
    assert_eq!(sender.resize(size(100, 30)), Taken::Queued);
    assert_eq!(sender.keys("c"), Taken::Queued);
    for _ in 0..3 {
        assert_eq!(sender.answered(&receipt), SendSettlement::Settled);
        let (next_receipt, outgoing) = next(&mut sender);
        receipt = next_receipt;
        sent.push(outgoing);
    }
    assert_eq!(
        sent,
        [
            keys("a"),
            keys("b"),
            Outgoing::Resize {
                cols: 100,
                rows: 30
            },
            keys("c")
        ]
    );
}

#[test]
fn goes_on_after_a_failed_send_without_repeating_it() {
    let mut sender = TerminalSender::default();
    assert_eq!(sender.keys("one"), Taken::Queued);
    let (receipt, first) = next(&mut sender);
    assert_eq!(sender.keys("two"), Taken::Queued);
    assert_eq!(sender.answered(&receipt), SendSettlement::Settled); // failure is never retried
    assert_eq!((first, next(&mut sender).1), (keys("one"), keys("two")));
}

#[test]
fn drops_keys_past_its_bound_and_says_so_once() {
    let mut sender = TerminalSender::default();
    assert_eq!(sender.keys("x"), Taken::Queued);
    let (first, outgoing) = next(&mut sender);
    assert_eq!(outgoing, keys("x"));
    assert_eq!(sender.keys(&"y".repeat(MAX_PENDING)), Taken::Queued);
    assert_eq!(sender.keys("z"), Taken::DroppedFirst);
    assert_eq!(sender.keys("w"), Taken::Dropped);
    assert_eq!(sender.answered(&first), SendSettlement::Settled);
    let (receipt, Outgoing::Keys(long)) = next(&mut sender) else {
        panic!("the long keys next");
    };
    assert_eq!(long.len(), SendTerminal::MAX_BYTES);
    // The original million-character stream remains exact; only each RPC is bounded.
    let mut combined = long;
    assert_eq!(sender.keys("v"), Taken::Queued);
    assert_eq!(sender.answered(&receipt), SendSettlement::Settled);
    while let Some(dispatch) = sender.take().expect("bounded stream") {
        let (receipt, Outgoing::Keys(part)) = dispatch.into_parts() else {
            panic!("keys only");
        };
        assert!(part.len() <= SendTerminal::MAX_BYTES);
        combined.push_str(&part);
        assert_eq!(sender.answered(&receipt), SendSettlement::Settled);
    }
    assert_eq!(combined, "y".repeat(MAX_PENDING) + "v");
}

#[test]
fn sends_nothing_after_close() {
    let mut sender = TerminalSender::default();
    assert_eq!(sender.keys("a"), Taken::Queued);
    let (receipt, first) = next(&mut sender);
    assert_eq!(sender.keys("b"), Taken::Queued);
    assert_eq!(sender.close(), Some(receipt.clone()));
    assert_eq!(sender.answered(&receipt), SendSettlement::Stale);
    assert_eq!(sender.keys("c"), Taken::Closed);
    assert_eq!(sender.resize(size(80, 24)), Taken::Closed);
    assert_eq!((first, sender.take().expect("closed")), (keys("a"), None));
}

#[test]
fn only_a_focused_view_that_has_typed_resizes() {
    let mut rule = ResizeRule::default();
    assert!(!rule.may_resize(true), "watching only");
    assert!(rule.typed(), "the first key sends the size first");
    assert!(!rule.typed());
    assert!(rule.may_resize(true));
    assert!(!rule.may_resize(false));
}

#[test]
fn replays_terminal_bytes_once_and_sums_model_calls() {
    let mut log = Log::default();
    let records = [
        log.record(&serde_json::json!({ "type": "terminal", "data": "aGk=" })),
        log.model(serde_json::json!({
            "route": "r", "model": "m", "api": "anthropic_messages",
            "usage": { "input": 1200, "output": 300 }, "stop": "end_turn", "duration_ms": 5
        })),
        log.record(&serde_json::json!({ "type": "terminal", "data": "IQ==" })),
    ];
    let mut terminal = TerminalLog::default();
    let bytes: Vec<Vec<u8>> = terminal
        .apply(&records)
        .into_iter()
        .map(<[u8]>::to_vec)
        .collect();
    assert_eq!(bytes, [b"hi".to_vec(), b"!".to_vec()]);
    assert!(
        terminal.apply(&records).is_empty(),
        "a restarted stream draws nothing twice"
    );
    let usage = terminal.usage().unwrap();
    assert_eq!(usage.calls, 1);
    assert_eq!(EN.usage_line(usage), "1 model call · 1.2k in · 300 out");
}

#[test]
fn links_are_only_absolute_http_urls() {
    assert!(is_web_url("https://example.com/a?b=c"));
    assert!(is_web_url("HTTP://example.com"));
    for href in [
        "javascript:alert(1)",
        "mailto:someone@example.com",
        "//example.com/path",
        "/sessions/box/3f2a",
        "sessions/box",
        "data:text/html,<script>",
        "vessel://pair#abc",
        " https://example.com",
        "https://",
    ] {
        assert!(!is_web_url(href), "{href}");
    }
    assert!(
        EN.open_link("https://example.com/x")
            .contains("https://example.com/x")
    );
}
