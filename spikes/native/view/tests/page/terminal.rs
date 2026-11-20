//! The page's `data/terminal-sender.test.ts` and `data/links.test.ts`, and
//! the terminal view's resize rule and frame.

use vessel_app_view::is_web_url;
use vessel_app_view::terminal::{
    MAX_PENDING, Outgoing, ResizeRule, Taken, TerminalLog, TerminalSender,
};

use vessel_proto::ResizeIntent;

use crate::support::{EN, Log};

fn keys(text: &str) -> Outgoing {
    Outgoing::Keys(text.to_owned())
}

#[test]
fn is_sent_one_call_at_a_time_in_the_order_typed() {
    let mut sender = TerminalSender::default();
    sender.resize(120, 40, ResizeIntent::Claim);
    assert_eq!(
        sender.take(),
        Some(Outgoing::Resize {
            cols: 120,
            rows: 40,
            intent: ResizeIntent::Claim,
        })
    );
    let _ = sender.keys("pasted line");
    let _ = sender.keys("\r");
    assert_eq!(sender.take(), None, "one in flight");
    sender.answered();
    // Keys typed during the resize go out together, Enter last.
    assert_eq!(sender.take(), Some(keys("pasted line\r")));
    sender.answered();
    assert_eq!(sender.take(), None);
    assert!(!sender.busy());
}

/// ADR 0116: a fit queued right after a claim (focus, then the pane's
/// size settling) goes out as one claim of the later size; an update after
/// it was sent stays an update.
#[test]
fn a_size_replacing_a_queued_claim_keeps_the_claim() {
    let mut sender = TerminalSender::default();
    let _ = sender.keys("a");
    let first = sender.take();
    sender.resize(80, 24, ResizeIntent::Claim);
    sender.resize(100, 30, ResizeIntent::Update);
    sender.answered();
    assert_eq!(first, Some(keys("a")));
    assert_eq!(
        sender.take(),
        Some(Outgoing::Resize {
            cols: 100,
            rows: 30,
            intent: ResizeIntent::Claim,
        })
    );
    sender.answered();
    sender.resize(90, 30, ResizeIntent::Update);
    assert_eq!(
        sender.take(),
        Some(Outgoing::Resize {
            cols: 90,
            rows: 30,
            intent: ResizeIntent::Update,
        })
    );
}

#[test]
fn keeps_keys_and_sizes_in_order_the_latest_size_only() {
    let mut sender = TerminalSender::default();
    let _ = sender.keys("a");
    let mut sent = vec![sender.take()];
    let _ = sender.keys("b");
    sender.resize(80, 24, ResizeIntent::Update);
    sender.resize(100, 30, ResizeIntent::Update);
    let _ = sender.keys("c");
    for _ in 0..3 {
        sender.answered();
        sent.push(sender.take());
    }
    assert_eq!(
        sent,
        [
            Some(keys("a")),
            Some(keys("b")),
            Some(Outgoing::Resize {
                cols: 100,
                rows: 30,
                intent: ResizeIntent::Update,
            }),
            Some(keys("c"))
        ]
    );
}

#[test]
fn goes_on_after_a_failed_send_without_repeating_it() {
    let mut sender = TerminalSender::default();
    let _ = sender.keys("one");
    let first = sender.take();
    let _ = sender.keys("two");
    sender.answered(); // failed: not repeated
    assert_eq!(
        (first, sender.take()),
        (Some(keys("one")), Some(keys("two")))
    );
}

#[test]
fn drops_keys_past_its_bound_and_says_so_once() {
    let mut sender = TerminalSender::default();
    assert_eq!(sender.keys("x"), Taken::Queued);
    assert_eq!(sender.take(), Some(keys("x")));
    assert_eq!(sender.keys(&"y".repeat(MAX_PENDING)), Taken::Queued);
    assert_eq!(sender.keys("z"), Taken::DroppedFirst);
    assert_eq!(sender.keys("w"), Taken::Dropped);
    sender.answered();
    let Some(Outgoing::Keys(long)) = sender.take() else {
        panic!("the long keys next");
    };
    assert_eq!(long.len(), MAX_PENDING);
    // Room again: keys are taken.
    assert_eq!(sender.keys("v"), Taken::Queued);
    sender.answered();
    assert_eq!(sender.take(), Some(keys("v")));
}

#[test]
fn sends_nothing_after_close() {
    let mut sender = TerminalSender::default();
    let _ = sender.keys("a");
    let first = sender.take();
    let _ = sender.keys("b");
    sender.close();
    sender.answered();
    let _ = sender.keys("c");
    assert_eq!((first, sender.take()), (Some(keys("a")), None));
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
