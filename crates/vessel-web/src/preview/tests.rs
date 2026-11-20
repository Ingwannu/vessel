//! The store's caps: one document per panel, at most `MAX_DOCUMENTS` and
//! `MAX_BYTES`, each dropped `IDLE` after its last load.

use super::*;

fn bytes(size: usize) -> Bytes {
    Bytes::from(vec![b'x'; size])
}

/// `times` steps of `step` after `start`.
fn after(start: Instant, step: Duration, times: u32) -> Instant {
    start.checked_add(step.checked_mul(times).unwrap()).unwrap()
}

#[test]
fn a_panel_keeps_only_its_last_document() {
    let previews = Previews::default();
    let now = Instant::now();
    let first = previews.put("panel", bytes(3), now).unwrap();
    let second = previews.put("panel", bytes(4), now).unwrap();
    assert_ne!(first, second);
    assert_eq!(first.len(), 32);
    assert!(first.bytes().all(|byte| byte.is_ascii_hexdigit()));
    assert!(previews.show(&first, now).is_none());
    assert_eq!(previews.show(&second, now).map(|body| body.len()), Some(4));
}

#[test]
fn the_least_recently_loaded_goes_first_past_the_count() {
    let previews = Previews::default();
    let start = Instant::now();
    let ids: Vec<String> = (0..MAX_DOCUMENTS)
        .map(|nth| {
            previews
                .put(&format!("panel-{nth}"), bytes(1), start)
                .unwrap()
        })
        .collect();
    let later = after(start, Duration::from_secs(1), 1);
    let first = ids.first().unwrap();
    // The first was loaded again: the second is now the oldest.
    assert!(previews.show(first, later).is_some());
    let _ninth = previews.put("another", bytes(1), later).unwrap();
    assert!(previews.show(first, later).is_some());
    assert!(previews.show(&ids[1], later).is_none());
}

#[test]
fn the_bytes_are_capped() {
    let previews = Previews::default();
    let now = Instant::now();
    assert_eq!(
        previews.put("huge", bytes(MAX_BYTES.saturating_add(1)), now),
        Err(Refused::TooLarge)
    );
    let half = MAX_BYTES.checked_div(2).unwrap().saturating_add(1);
    let first = previews.put("a", bytes(half), now).unwrap();
    let second = previews.put("b", bytes(half), now).unwrap();
    assert!(previews.show(&first, now).is_none());
    assert!(previews.show(&second, now).is_some());
}

#[test]
fn a_document_left_idle_is_dropped() {
    let previews = Previews::default();
    let start = Instant::now();
    let id = previews.put("panel", bytes(1), start).unwrap();
    assert!(previews.show(&id, after(start, IDLE, 1)).is_some());
    // Loaded at `IDLE`: kept until `IDLE` after that.
    let second = after(start, IDLE, 2);
    assert!(previews.show(&id, second).is_some());
    let past = after(second, IDLE, 1)
        .checked_add(Duration::from_secs(1))
        .unwrap();
    assert!(previews.show(&id, past).is_none());
    assert!(
        previews
            .show("0123456789abcdef0123456789abcdef", start)
            .is_none()
    );
}

#[test]
fn a_slot_is_a_short_plain_name() {
    let named = |value: &str| {
        let mut headers = HeaderMap::new();
        let _previous = headers.insert(SLOT, HeaderValue::from_str(value).unwrap());
        slot(&headers).map(str::to_owned)
    };
    assert_eq!(named("3f2a-9c_1").as_deref(), Some("3f2a-9c_1"));
    assert_eq!(named(""), None);
    assert_eq!(named("a b"), None);
    assert_eq!(named(&"a".repeat(65)), None);
    assert_eq!(slot(&HeaderMap::new()), None);
}
