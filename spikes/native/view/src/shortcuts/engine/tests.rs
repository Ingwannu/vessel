use super::*;
use crate::shortcuts::Platform;
use crate::shortcuts::context::{CommandCenterState, FocusScope, HostKind};

const CONTEXT: ShortcutContext = ShortcutContext {
    platform: Platform::Other,
    host: HostKind::Desktop,
    focus: FocusScope::Other,
    command_center: CommandCenterState::Closed,
    admission: Admission::Available,
};

fn binding(id: &str, text: &str, action: u8) -> EffectiveBinding<u8> {
    EffectiveBinding {
        id: id.parse().unwrap(),
        action,
        payload: Payload::None,
        keys: KeySequence::parse(text).unwrap(),
        when: When::ANY,
        repeat: Repeat::Allow,
        prevent_default: Consume::Yes,
        stop_propagation: Consume::Yes,
    }
}

fn chord(text: &str) -> Chord {
    KeySequence::parse(text)
        .unwrap()
        .chords()
        .next()
        .unwrap()
        .clone()
}

fn press(
    engine: &mut Engine,
    text: &str,
    at: u64,
    bindings: &[EffectiveBinding<u8>],
) -> Resolution<u8> {
    engine.resolve(
        &chord(text),
        KeyRepeat::Initial,
        Duration::from_millis(at),
        CONTEXT,
        bindings,
    )
}

fn action(resolution: Resolution<u8>) -> Option<u8> {
    match resolution {
        Resolution::Run { action, .. } => Some(action),
        Resolution::Waiting | Resolution::Pass => None,
    }
}

#[test]
fn prefix_candidates_beat_a_single_binding_and_three_steps_narrow_in_order() {
    let bindings = [
        binding("single", "Ctrl+K", 0),
        binding("one", "Ctrl+K Ctrl+J Ctrl+S", 1),
        binding("two", "Ctrl+K Ctrl+L Ctrl+S", 2),
    ];
    let mut engine = Engine::default();
    assert_eq!(
        press(&mut engine, "Ctrl+K", 0, &bindings),
        Resolution::Waiting
    );
    assert_eq!(
        press(&mut engine, "Ctrl+J", 1, &bindings),
        Resolution::Waiting
    );
    assert_eq!(action(press(&mut engine, "Ctrl+S", 2, &bindings)), Some(1));
}

#[test]
fn failed_pending_step_does_not_become_a_new_shortcut_and_timeout_does() {
    let bindings = [
        binding("sequence", "Ctrl+K Ctrl+S", 1),
        binding("single", "Ctrl+J", 2),
    ];
    let mut engine = Engine::default();
    assert_eq!(
        press(&mut engine, "Ctrl+K", 0, &bindings),
        Resolution::Waiting
    );
    assert_eq!(press(&mut engine, "Ctrl+J", 1, &bindings), Resolution::Pass);
    assert_eq!(
        press(&mut engine, "Ctrl+K", 2, &bindings),
        Resolution::Waiting
    );
    assert_eq!(
        action(press(&mut engine, "Ctrl+J", 1502, &bindings)),
        Some(2)
    );
}

#[test]
fn every_step_rechecks_focus_admission_and_exact_binding_keys() {
    let mut bindings = [binding("sequence", "Ctrl+K Ctrl+S", 1)];
    bindings[0].when = When::ANY.outside_terminal();
    let mut engine = Engine::default();
    assert_eq!(
        press(&mut engine, "Ctrl+K", 0, &bindings),
        Resolution::Waiting
    );
    assert_eq!(
        engine.resolve(
            &chord("Ctrl+S"),
            KeyRepeat::Initial,
            Duration::from_millis(1),
            ShortcutContext {
                focus: FocusScope::Terminal,
                ..CONTEXT
            },
            &bindings
        ),
        Resolution::Pass
    );
    assert_eq!(
        press(&mut engine, "Ctrl+K", 2, &bindings),
        Resolution::Waiting
    );
    bindings[0].keys = KeySequence::parse("Ctrl+K Ctrl+J").unwrap();
    assert_eq!(press(&mut engine, "Ctrl+J", 3, &bindings), Resolution::Pass);
    for admission in [
        Admission::Composing,
        Admission::CharacterInput,
        Admission::Capturing,
        Admission::Inactive,
        Admission::Overlay,
    ] {
        assert_eq!(
            press(&mut engine, "Ctrl+K", 4, &bindings),
            Resolution::Waiting
        );
        assert_eq!(
            engine.resolve(
                &chord("Ctrl+J"),
                KeyRepeat::Initial,
                Duration::from_millis(5),
                ShortcutContext {
                    admission,
                    ..CONTEXT
                },
                &bindings
            ),
            Resolution::Pass
        );
        assert_eq!(press(&mut engine, "Ctrl+J", 6, &bindings), Resolution::Pass);
    }
}

#[test]
fn digit_templates_supply_validated_indices_and_reject_zero() {
    let mut bindings = [binding("jump", "Ctrl+Digit", 1)];
    bindings[0].payload = Payload::Index;
    let mut engine = Engine::default();
    assert_eq!(press(&mut engine, "Ctrl+0", 0, &bindings), Resolution::Pass);
    for index in 1..=9 {
        let Resolution::Run { index: found, .. } =
            press(&mut engine, &format!("Ctrl+{index}"), 1, &bindings)
        else {
            panic!("expected digit match")
        };
        assert_eq!(found.unwrap().get(), index);
    }
}

#[test]
fn repeat_policy_and_propagation_are_independent_and_retained() {
    let mut bindings = [binding("interrupt", "Escape", 1)];
    bindings[0].repeat = Repeat::Deny;
    bindings[0].prevent_default = Consume::No;
    bindings[0].stop_propagation = Consume::No;
    let mut engine = Engine::default();
    assert_eq!(
        engine.resolve(
            &chord("Escape"),
            KeyRepeat::Repeated,
            Duration::ZERO,
            CONTEXT,
            &bindings
        ),
        Resolution::Pass
    );
    assert_eq!(
        engine.resolve(
            &chord("Escape"),
            KeyRepeat::Initial,
            Duration::ZERO,
            CONTEXT,
            &bindings
        ),
        Resolution::Run {
            action: 1,
            index: None,
            prevent_default: Consume::No,
            stop_propagation: Consume::No
        }
    );
}

#[test]
fn native_editing_is_kept_even_if_a_preference_file_rebinds_copy() {
    let bindings = [binding("copy", "Mod+C", 1)];
    let mut engine = Engine::default();
    for focus in [FocusScope::Editable, FocusScope::MessageInput] {
        assert_eq!(
            engine.resolve(
                &chord("Mod+C"),
                KeyRepeat::Initial,
                Duration::ZERO,
                ShortcutContext { focus, ..CONTEXT },
                &bindings
            ),
            Resolution::Pass
        );
    }
    assert_eq!(action(press(&mut engine, "Mod+C", 0, &bindings)), Some(1));
}

#[test]
fn index_payload_survives_a_concrete_digit_override() {
    let mut bindings = [binding("jump", "Ctrl+3", 1)];
    bindings[0].payload = Payload::Index;
    let mut engine = Engine::default();
    let Resolution::Run { index, .. } = press(&mut engine, "Ctrl+3", 0, &bindings) else {
        panic!("expected index action")
    };
    assert_eq!(index.unwrap().get(), 3);
}
