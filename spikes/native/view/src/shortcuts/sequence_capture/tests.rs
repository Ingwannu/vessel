use super::*;
use crate::shortcuts::engine::{Consume, Payload, Repeat};

const CONTEXT: ShortcutContext = ShortcutContext {
    platform: Platform::Other,
    host: HostKind::Desktop,
    focus: FocusScope::Other,
    command_center: CommandCenterState::Closed,
    admission: Admission::Capturing,
};

fn chord(text: &str) -> Chord {
    KeySequence::parse(text)
        .unwrap()
        .chords()
        .next()
        .unwrap()
        .clone()
}

fn binding(id: &str, text: &str, when: When) -> EffectiveBinding<u8> {
    EffectiveBinding {
        id: id.parse().unwrap(),
        action: 1,
        keys: KeySequence::parse(text).unwrap(),
        payload: Payload::None,
        when,
        repeat: Repeat::Allow,
        prevent_default: Consume::Yes,
        stop_propagation: Consume::Yes,
    }
}

#[test]
fn capture_appends_chords_backspace_removes_one_and_escape_is_recordable() {
    let mut capture = SequenceCapture::new("target".parse().unwrap(), CaptureRole::Ordinary);
    assert_eq!(
        capture.verdict::<u8>(When::ANY, CONTEXT, &[]),
        CaptureVerdict::Waiting
    );
    capture.press(chord("Ctrl+K"), CONTEXT);
    capture.press(chord("Ctrl+J"), CONTEXT);
    capture.press(chord("Escape"), CONTEXT);
    let CaptureVerdict::Ready { keys, .. } = capture.verdict::<u8>(When::ANY, CONTEXT, &[]) else {
        panic!("expected ready")
    };
    assert_eq!(keys.text(), "Ctrl+K Ctrl+J Escape");
    capture.back();
    assert_eq!(capture.chords().count(), 2);
}

#[test]
fn index_capture_requires_a_digit_and_normalizes_the_whole_index_group() {
    let mut capture = SequenceCapture::new("jump".parse().unwrap(), CaptureRole::Index);
    capture.press(chord("Ctrl+K"), CONTEXT);
    assert_eq!(
        capture.verdict::<u8>(When::ANY, CONTEXT, &[]),
        CaptureVerdict::IndexRequired
    );
    capture.press(chord("Alt+3"), CONTEXT);
    let CaptureVerdict::Ready { keys, .. } = capture.verdict::<u8>(When::ANY, CONTEXT, &[]) else {
        panic!("expected index group")
    };
    assert_eq!(keys.text(), "Ctrl+K Alt+Digit");
}

#[test]
fn conflict_projection_includes_every_alias_and_prefix_but_not_disjoint_scopes() {
    let mut capture = SequenceCapture::new("target".parse().unwrap(), CaptureRole::Ordinary);
    capture.press(chord("Ctrl+K"), CONTEXT);
    let bindings = [
        binding("one", "Ctrl+K Ctrl+S", When::ANY),
        binding("two", "Mod+K", When::ANY),
        binding("target", "Ctrl+K", When::ANY),
        binding(
            "terminal",
            "Ctrl+K",
            When::ANY.scopes(&[FocusScope::Terminal]),
        ),
    ];
    let CaptureVerdict::Ready { conflicts, .. } =
        capture.verdict(When::ANY.outside_terminal(), CONTEXT, &bindings)
    else {
        panic!("expected conflicts")
    };
    assert_eq!(
        conflicts.iter().map(ActionId::as_str).collect::<Vec<_>>(),
        ["one", "two"]
    );
}

#[test]
fn native_editing_refusal_keeps_the_last_capture_and_backspace_clears_it() {
    let mut capture = SequenceCapture::new("target".parse().unwrap(), CaptureRole::Ordinary);
    capture.press(chord("Ctrl+K"), CONTEXT);
    capture.press(chord("Ctrl+C"), CONTEXT);
    assert_eq!(
        capture.verdict::<u8>(When::ANY, CONTEXT, &[]),
        CaptureVerdict::Refused(CaptureFault::Editing)
    );
    assert_eq!(capture.chords().count(), 1);
    capture.back();
    assert_eq!(
        capture.verdict::<u8>(When::ANY, CONTEXT, &[]),
        CaptureVerdict::Waiting
    );
    // Native workspace/tab shortcuts are intentionally bindable.
    capture.press(chord("Ctrl+N"), CONTEXT);
    assert!(matches!(
        capture.verdict::<u8>(When::ANY, CONTEXT, &[]),
        CaptureVerdict::Ready { .. }
    ));
}

#[test]
fn wildcard_and_concrete_digits_overlap_but_different_sequence_tails_do_not() {
    let overlap = |left, right| {
        sequences_overlap(
            &KeySequence::parse(left).unwrap(),
            &KeySequence::parse(right).unwrap(),
            Platform::Other,
        )
    };
    assert!(overlap("Alt+Digit", "Alt+4"));
    assert!(!overlap("Alt+Digit", "Alt+0"));
    assert!(overlap("Ctrl+K", "Ctrl+K Ctrl+S"));
    assert!(!overlap("Ctrl+K Ctrl+S", "Ctrl+K Ctrl+J"));
}

#[test]
fn composing_character_input_and_inactive_windows_do_not_record_chords() {
    let mut capture = SequenceCapture::new("target".parse().unwrap(), CaptureRole::Ordinary);
    for admission in [
        Admission::Composing,
        Admission::CharacterInput,
        Admission::Inactive,
        Admission::Overlay,
    ] {
        capture.press(
            chord("Ctrl+K"),
            ShortcutContext {
                admission,
                ..CONTEXT
            },
        );
        assert_eq!(capture.chords().count(), 0);
    }
}
