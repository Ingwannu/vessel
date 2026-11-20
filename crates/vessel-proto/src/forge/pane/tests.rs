use super::*;

#[test]
fn a_run_id_is_positive_and_fits_a_page() {
    assert!(serde_json::from_str::<RunId>("0").is_err());
    assert!(serde_json::from_str::<RunId>("9007199254740992").is_err());
    assert_eq!(serde_json::from_str::<RunId>("42").unwrap().get(), 42);
}

#[test]
fn a_log_tail_keeps_the_last_two_hundred_lines() {
    let lines: Vec<String> = (1..=250).map(|line| format!("line {line}")).collect();
    let log = lines.join("\n") + "\n";
    let tail = LogTail::of(log.as_bytes(), false);
    assert!(tail.cut);
    let lines: Vec<&str> = tail.text.as_str().lines().collect();
    assert_eq!(lines.len(), LogTail::MAX_LINES);
    assert_eq!(lines.first(), Some(&"line 51"));
    assert_eq!(lines.last(), Some(&"line 250"));
}

#[test]
fn a_log_tail_keeps_sixteen_kib_from_a_line_start_and_drops_control_bytes() {
    let long = format!("{}\nend \u{1b}[31mred\u{1b}[0m\ttab\n", "한".repeat(9000));
    let tail = LogTail::of(long.as_bytes(), false);
    assert!(tail.cut, "the first line did not fit");
    assert_eq!(tail.text.as_str(), "end [31mred[0m\ttab");
    let short = LogTail::of(b"one\ntwo\n", true);
    assert!(short.cut, "a read that already left the start out says so");
    assert_eq!(short.text.as_str(), "one\ntwo");
    let whole = LogTail::of(b"one\ntwo", false);
    assert!(!whole.cut);
}

#[test]
fn timeline_items_are_tagged_and_an_unknown_review_state_still_decodes() {
    let item: TimelineItem = serde_json::from_value(serde_json::json!({
        "kind": "comment",
        "id": "PRRC_1",
        "body": "Why?",
        "created_at": "2026-11-19T00:00:00Z",
        "review": "PRR_1",
        "thread": {"thread": "PRRT_1", "path": "src/a.rs", "line": 3,
                   "resolved": false, "outdated": true},
    }))
    .unwrap();
    assert_eq!(item.id().as_str(), "PRRC_1");
    let TimelineItem::Comment {
        thread: Some(place),
        ..
    } = &item
    else {
        panic!("a thread comment: {item:?}");
    };
    assert!(place.outdated && !place.resolved);
    let review: TimelineItem = serde_json::from_value(serde_json::json!({
        "kind": "review", "id": "PRR_2", "body": "", "created_at": "2026-11-19T00:00:00Z",
        "state": "dismissed",
    }))
    .unwrap();
    assert!(matches!(
        review,
        TimelineItem::Review {
            state: ReviewState::Unknown,
            ..
        }
    ));
}

#[test]
fn forge_values_are_made_bounded_wire_values() {
    assert!(ForgeItemId::lossy("PRR_kwDO 1").is_none(), "no space");
    assert!(ForgeItemId::lossy(&"x".repeat(129)).is_none());
    assert_eq!(ForgePath::lossy("a\nb.rs").unwrap().as_str(), "a b.rs");
    let body = PullBody::lossy(&format!("a\0b{}", "c".repeat(70_000)));
    assert!(body.as_str().starts_with("ab"));
    assert_eq!(body.as_str().len(), 64 * 1024);
}
