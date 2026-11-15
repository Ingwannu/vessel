use super::*;

#[test]
fn first_sight_is_complete_but_growth_is_paced_and_completion_flushes() {
    let mut reveal = TextReveal::begin("a");
    assert!(reveal.settled());
    let full = format!("a{}", "b".repeat(300));
    reveal.retarget(&full);
    assert_eq!(reveal.visible_end(&full, TextPhase::Streaming), 1);
    reveal.advance(Duration::from_millis(16));
    let end = reveal.visible_end(&full, TextPhase::Streaming);
    assert!(end > 1 && end < full.len());
    assert_eq!(full.len(), 301, "the canonical input is untouched");
    reveal.complete();
    assert!(reveal.settled());
    assert_eq!(reveal.visible_end(&full, TextPhase::Complete), full.len());
    reveal.retarget("short");
    assert_eq!(reveal.visible_end("short", TextPhase::Streaming), 5);
}

#[test]
fn proportional_steps_clamp_stalls_and_make_progress_without_overshooting() {
    assert_eq!(step(300, Duration::from_millis(75)), 150);
    assert_eq!(step(100, Duration::ZERO), 0);
    assert_eq!(step(0, Duration::from_millis(16)), 0);
    assert_eq!(step(1, Duration::from_nanos(1)), 1);
    assert_eq!(step(42, Duration::MAX), 42);
    let large = step(usize::MAX, Duration::from_millis(149));
    assert!(large > usize::MAX.checked_div(2).unwrap() && large < usize::MAX);
    let mut left = 500;
    for _ in 0..120 {
        left -= step(left, Duration::from_millis(16));
    }
    assert_eq!(left, 0);
}

#[test]
fn utf16_release_never_cuts_combining_emoji_jamo_or_indic_clusters() {
    for text in ["e\u{301}x", "🚀x", "❤️x", "\u{1100}\u{1161}x", "कःx"] {
        let mut reveal = TextReveal::begin("");
        reveal.retarget(text);
        reveal.released = 1;
        assert_eq!(reveal.visible_end(text, TextPhase::Streaming), 0, "{text}");
    }
    let text = "x👨‍👩‍👧y";
    let mut reveal = TextReveal::begin("");
    reveal.retarget(text);
    for released in 2..9 {
        reveal.released = released;
        assert_eq!(reveal.visible_end(text, TextPhase::Streaming), 1);
    }
    reveal.released = 9;
    assert_eq!(
        text.get(..reveal.visible_end(text, TextPhase::Streaming))
            .unwrap(),
        "x👨‍👩‍👧"
    );
    let text = "👍🏽x";
    reveal.retarget(text);
    reveal.released = 2;
    assert_eq!(reveal.visible_end(text, TextPhase::Streaming), 0);
}

#[test]
fn incomplete_arrival_fragments_wait_only_while_the_turn_is_streaming() {
    for (text, expected) in [("step👨‍", "step👨"), ("flag🇺", "flag"), ("🇺🇸🇨", "🇺🇸")]
    {
        let reveal = TextReveal::begin(text);
        assert_eq!(
            text.get(..reveal.visible_end(text, TextPhase::Streaming))
                .unwrap(),
            expected
        );
        assert_eq!(reveal.visible_end(text, TextPhase::Complete), text.len());
    }
    let reveal = TextReveal::begin("🇺🇸");
    assert_eq!(reveal.visible_end("🇺🇸", TextPhase::Streaming), "🇺🇸".len());
}

#[test]
fn a_long_cluster_cannot_stall_the_monotonic_release_counter() {
    let text = format!("e{}!", "\u{301}".repeat(300));
    let mut reveal = TextReveal::begin("");
    reveal.retarget(&text);
    for _ in 0..120 {
        reveal.advance(Duration::from_millis(16));
        let end = reveal.visible_end(&text, TextPhase::Streaming);
        assert!(text.get(..end).is_some());
    }
    assert!(reveal.settled());
    assert_eq!(reveal.visible_end(&text, TextPhase::Streaming), text.len());
}

#[test]
fn high_refresh_frames_carry_their_remainder_and_reset_after_settling() {
    let mut clock = RevealClock::default();
    assert_eq!(clock.frame(Duration::ZERO), Some(FRAME_INTERVAL));
    assert_eq!(clock.frame(Duration::from_millis(8)), None);
    assert_eq!(
        clock.frame(Duration::from_millis(17)),
        Some(Duration::from_millis(17))
    );
    assert_eq!(clock.frame(Duration::from_millis(25)), None);
    assert!(clock.frame(Duration::from_millis(34)).is_some());
    clock.reset();
    assert_eq!(clock.frame(Duration::from_secs(5)), Some(FRAME_INTERVAL));
}
