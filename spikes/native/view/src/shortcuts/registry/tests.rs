//! Product-default regressions are separate from legacy API fixtures. These
//! exercise actual resolution, platform predicates and editable exclusions.
use super::*;
use crate::shortcuts::KeyPress;
use crate::shortcuts::engine::{Engine, KeyRepeat, Resolution};
use std::time::Duration;

fn resolve(
    keys: &str,
    context: ShortcutContext,
    overrides: &BTreeMap<ActionId, Bindings>,
) -> Resolution<Action> {
    let chord = KeySequence::parse(keys)
        .unwrap()
        .chords()
        .next()
        .unwrap()
        .clone();
    Engine::default().resolve(
        &chord,
        KeyRepeat::Initial,
        Duration::ZERO,
        context,
        &effective_bindings(overrides, context).unwrap(),
    )
}
fn action(resolution: Resolution<Action>) -> Option<Action> {
    if let Resolution::Run { action, .. } = resolution {
        Some(action)
    } else {
        None
    }
}

#[test]
fn shipped_defaults_use_paseo_keys_and_old_page_keys_are_unbound() {
    assert_eq!(PASEO_BINDING_COUNT, 76);
    let context = default_context(Platform::Other);
    let none = BTreeMap::new();
    for keys in ["N", "/", "G"] {
        assert_eq!(resolve(keys, context, &none), Resolution::Pass, "{keys}");
    }
    assert_eq!(
        action(resolve("Alt+3", context, &none)),
        Some(Action::JumpTab)
    );
    assert_eq!(
        action(resolve("Ctrl+K", context, &none)),
        Some(Action::Palette)
    );
    assert_eq!(
        action(resolve("Ctrl+Alt+T", context, &none)),
        Some(Action::CycleTheme)
    );
    assert!(matches!(
        resolve("Ctrl+3", context, &none),
        Resolution::Run {
            action: Action::JumpSession,
            index: Some(_),
            ..
        }
    ));
    let definitions = definitions().unwrap();
    let ids: BTreeSet<_> = definitions
        .iter()
        .map(|definition| &definition.id)
        .collect();
    assert_eq!(ids.len(), definitions.len());
}

#[test]
fn help_question_mark_uses_the_logical_character_and_stands_down_in_editing_scopes() {
    let mut context = default_context(Platform::Other);
    let bindings = effective_bindings(&BTreeMap::new(), context).unwrap();
    let press = KeyPress {
        key: "?".to_owned(),
        code: "Slash".to_owned(),
        shift: true,
        ..KeyPress::default()
    };
    for focus in [
        FocusScope::Other,
        FocusScope::Editable,
        FocusScope::MessageInput,
        FocusScope::Terminal,
    ] {
        context.focus = focus;
        let result = Engine::default().resolve_keypress(
            &press,
            KeyRepeat::Initial,
            Duration::ZERO,
            context,
            &bindings,
        );
        assert_eq!(
            action(result),
            (focus == FocusScope::Other).then_some(Action::Help)
        );
    }
}

#[test]
fn non_mac_terminal_predicate_survives_rebinding() {
    for platform in [Platform::Mac, Platform::Other] {
        let mut context = default_context(platform);
        let id = binding_id(Action::Palette, context).unwrap();
        let overrides = BTreeMap::from([(
            id,
            Bindings::try_from(vec!["Mod+J".parse().unwrap()]).unwrap(),
        )]);
        context.focus = FocusScope::Terminal;
        assert_eq!(
            action(resolve("Mod+J", context, &overrides)),
            (platform == Platform::Mac).then_some(Action::Palette)
        );
    }
}
