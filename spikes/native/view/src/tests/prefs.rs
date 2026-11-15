//! The page's `data/prefs.test.ts`, `compose-keys.test.ts` and the
//! bridge's own tests of the same document (`wire/prefs.rs`,
//! `wire/prefs/keys.rs`, `wire/prefs/change.rs`): the same JSON, the
//! same rules, plus the app-only language preference.

use std::collections::BTreeMap;

use vessel_proto::{ChoiceValue, HarnessKind, NodeId, OptionId, OptionValueId};

use crate::Lang;
use crate::files::{FileFault, FileProblem};
use crate::prefs::{
    ActionId, AgentKind, Bindings, Choices, ContentWidth, Grouping, InvalidActionId,
    InvalidKeyBinding, Kept, KeyBinding, LaunchOnly, MAX_ACTIONS, MAX_MACHINES, MachineDefaults,
    Panel, PreferenceChange, Preferences, Refused, SendKey, ShortcutEdit, ShortcutTarget,
    SidebarView, Theme, TurnDelivery,
};

const DEFAULT: &str = r#"{"appearance":{"theme":"system","interface_size":14,"code_size":12,"content_width":"comfortable","conversation":"full"},"layout":{"sidebar":"shown","inspector":"shown"},"composer":{"send":"enter","delivery":"steer"},"shortcuts":{},"notifications":{"alerts":"off","title":true},"sidebar":{"group":"machine","sort":"created","ended":false},"defaults":{}}"#;

fn change(json: &str) -> PreferenceChange {
    serde_json::from_str(json).unwrap()
}

fn apply(prefs: &Preferences, json: &str) -> Preferences {
    prefs.changed(&change(json)).unwrap()
}

#[test]
fn defaults_are_the_contracts() {
    let values = Preferences::default();
    // Keep the page/bridge fixture unchanged. The native document adds two
    // explicit fields (the language, and Appearance's Animations, word 39)
    // and differs in one stated default: ended sessions are listed
    // (`SidebarView::default`, 2026-11-14; the page and the bridge still
    // write `"ended":false` until the node line's slice moves them, and
    // then this exception goes). Every other byte of its defaults still
    // comes from that fixture.
    let native_default = DEFAULT
        .replacen('{', r#"{"language":"system","#, 1)
        .replacen(
            r#""conversation":"full""#,
            r#""conversation":"full","animations":"on""#,
            1,
        )
        .replacen(r#""ended":false"#, r#""ended":true"#, 1);
    assert_eq!(serde_json::to_string(&values).unwrap(), native_default);
    assert_eq!(
        serde_json::from_str::<Preferences>(&native_default).unwrap(),
        values
    );
    // The page's document reads as the defaults with its own stated
    // `"ended":false`: the one temporary difference, kept as written.
    let page_default = Preferences {
        sidebar: SidebarView {
            ended: false,
            ..values.sidebar
        },
        ..values.clone()
    };
    assert_eq!(
        serde_json::from_str::<Preferences>(DEFAULT).unwrap(),
        page_default
    );
    assert_eq!(serde_json::from_str::<Preferences>("{}").unwrap(), values);
}

/// Ended sessions: a document without the field lists them (Paseo's, and
/// 0.0.0.0.16's); one the person turned off stays off; an unreadable value
/// lists them as a missing one does.
#[test]
fn ended_sessions_are_listed_unless_turned_off() {
    let missing: Preferences =
        serde_json::from_str(r#"{"sidebar":{"group":"status","sort":"activity"}}"#).unwrap();
    assert!(missing.sidebar.ended);
    assert_eq!(missing.sidebar.group, Grouping::Status);
    let off: Preferences = serde_json::from_str(r#"{"sidebar":{"ended":false}}"#).unwrap();
    assert!(!off.sidebar.ended);
    let unreadable: Preferences = serde_json::from_str(r#"{"sidebar":{"ended":"yes"}}"#).unwrap();
    assert!(unreadable.sidebar.ended);
    assert!(Preferences::default().sidebar.ended);
}

/// "the first paint": the values written, and the defaults for the rest.
#[test]
fn the_values_written_and_the_defaults_for_the_rest() {
    let values: Preferences = serde_json::from_str(
        r#"{"appearance":{"theme":"dark","interface_size":16,"code_size":13,"content_width":"wide"}}"#,
    )
    .unwrap();
    let appearance = values.appearance;
    assert_eq!(appearance.theme, Theme::Dark);
    assert_eq!(appearance.interface_size.get(), 16);
    assert_eq!(appearance.code_size.get(), 13);
    assert_eq!(appearance.content_width, ContentWidth::Wide);
    assert_eq!(values.composer, Preferences::default().composer);
}

/// "a value out of range or missing is the default": one setting, not the
/// document.
#[test]
fn a_value_out_of_range_or_unknown_is_the_default() {
    let values: Preferences = serde_json::from_str(
        r#"{"appearance":{"interface_size":40,"theme":"sepia","code_size":11},"layout":"wide","notifications":{"title":"yes"}}"#,
    )
    .unwrap();
    assert_eq!(values.appearance.interface_size.get(), 14);
    assert_eq!(values.appearance.theme, Theme::System);
    assert_eq!(values.appearance.code_size.get(), 11);
    assert_eq!(values.layout, Preferences::default().layout);
    assert!(values.notifications.title);
    // A map is read entry by entry (the bridge's `Entries`).
    let values: Preferences = serde_json::from_str(
        r#"{"shortcuts":{"find":["Mod+K"],"palette":["mod+k"],"Bad":["/"]},"defaults":{"not-a-node":{}}}"#,
    )
    .unwrap();
    assert_eq!(
        serde_json::to_string(&values.shortcuts).unwrap(),
        r#"{"find":["Mod+K"]}"#
    );
    assert!(values.defaults.is_empty());
}

#[test]
fn one_setting_the_others_kept() {
    let next = apply(
        &Preferences::default(),
        r#"{"set":"send","value":"mod_enter"}"#,
    );
    assert_eq!(next.composer.send, SendKey::ModEnter);
    assert_eq!(next.appearance, Preferences::default().appearance);
}

#[test]
fn the_send_key_and_the_delivery_are_the_composers_two_settings() {
    let steer = apply(
        &Preferences::default(),
        r#"{"set":"delivery","value":"steer"}"#,
    );
    let both = apply(&steer, r#"{"set":"send","value":"mod_enter"}"#);
    assert_eq!(both.composer.send, SendKey::ModEnter);
    assert_eq!(both.composer.delivery, TurnDelivery::Steer);
}

#[test]
fn shortcut_edits_together_default_removes_the_override() {
    let set = apply(
        &Preferences::default(),
        r#"{"set":"shortcuts","value":{"edits":[
            {"action":"find","to":{"kind":"bindings","keys":["Mod+K"]}},
            {"action":"palette","to":{"kind":"bindings","keys":["/"]}}]}}"#,
    );
    assert_eq!(
        serde_json::to_string(&set.shortcuts).unwrap(),
        r#"{"find":["Mod+K"],"palette":["/"]}"#
    );
    let back = apply(
        &set,
        r#"{"set":"shortcuts","value":{"edits":[{"action":"find","to":{"kind":"default"}}]}}"#,
    );
    assert_eq!(
        serde_json::to_string(&back.shortcuts).unwrap(),
        r#"{"palette":["/"]}"#
    );
    assert!(
        apply(&back, r#"{"set":"reset_shortcuts"}"#)
            .shortcuts
            .is_empty()
    );
}

#[test]
fn a_machines_defaults_and_forgetting_them() {
    let node = NodeId::random();
    let agent = apply(
        &Preferences::default(),
        &format!(r#"{{"set":"default_agent","value":{{"node":"{node}","agent":"codex"}}}}"#),
    );
    let choices = apply(
        &agent,
        &format!(
            r#"{{"set":"agent_defaults","value":{{"node":"{node}","agent":"codex","profile":null,
                "choices":{{"model":{{"kind":"select","value":"gpt"}}}}}}}}"#
        ),
    );
    let machine = &choices.defaults[&node];
    assert_eq!(machine.agent, Some(HarnessKind::Codex));
    assert_eq!(
        serde_json::to_string(machine).unwrap(),
        r#"{"agent":"codex","agents":{"codex":{"profile":null,"choices":{"model":{"kind":"select","value":"gpt"}}}}}"#
    );
    let forgot = apply(
        &choices,
        &format!(r#"{{"set":"forget_defaults","value":{{"node":"{node}"}}}}"#),
    );
    assert!(forgot.defaults.is_empty());
}

/// The bridge's `changed`: a change that grows past a bound is refused;
/// one that does not grow always applies.
#[test]
fn a_change_past_a_bound_is_refused() {
    let mut full = Preferences::default();
    for n in 0..MAX_ACTIONS {
        let _old = full.shortcuts.insert(
            format!("a{n}").parse::<ActionId>().unwrap(),
            Bindings::default(),
        );
    }
    let one_more = PreferenceChange::Shortcuts {
        edits: vec![ShortcutEdit {
            action: "more".parse().unwrap(),
            to: ShortcutTarget::Default,
        }]
        .try_into()
        .unwrap(),
    };
    // A default edit of an action not kept does not grow it.
    assert!(full.changed(&one_more).is_ok());
    let grow = PreferenceChange::Shortcuts {
        edits: vec![ShortcutEdit {
            action: "more".parse().unwrap(),
            to: ShortcutTarget::Bindings {
                keys: Bindings::default(),
            },
        }]
        .try_into()
        .unwrap(),
    };
    assert_eq!(full.changed(&grow), Err(Refused::Actions));
    for _ in 0..MAX_MACHINES {
        let _old = full
            .defaults
            .insert(NodeId::random(), MachineDefaults::default());
    }
    let another = PreferenceChange::DefaultAgent {
        node: NodeId::random(),
        agent: None,
    };
    assert_eq!(full.changed(&another), Err(Refused::Machines));
    let mine = *full.defaults.keys().next().unwrap();
    let again = PreferenceChange::DefaultAgent {
        node: mine,
        agent: Some(HarnessKind::Gemini),
    };
    assert!(full.changed(&again).is_ok());
}

/// The bridge's `changes_are_tagged_by_setting_with_their_value`.
#[test]
fn changes_are_tagged_by_setting_with_their_value() {
    assert_eq!(
        change(r#"{"set":"theme","value":"dark"}"#),
        PreferenceChange::Theme(Theme::Dark)
    );
    assert_eq!(
        change(r#"{"set":"inspector_panel","value":"hidden"}"#),
        PreferenceChange::InspectorPanel(Panel::Hidden)
    );
    for refused in [
        r#"{"set":"interface_size","value":21}"#,
        r#"{"set":"code_size","value":9}"#,
        r#"{"set":"theme","value":"sepia"}"#,
        r#"{"set":"delivery","value":"start"}"#,
        r#"{"set":"shortcuts","value":{"edits":[{"action":"palette","to":{"kind":"bindings","keys":["Mod+K","/","N"]}}]}}"#,
        r#"{"set":"shortcuts","value":{"edits":[{"action":"palette","to":{"kind":"bindings","keys":["mod+k"]}}]}}"#,
        r#"{"set":"what","value":1}"#,
    ] {
        assert!(
            serde_json::from_str::<PreferenceChange>(refused).is_err(),
            "{refused}"
        );
    }
    let seventeen: Vec<String> = (0..17)
        .map(|n| format!(r#"{{"action":"a{n}","to":{{"kind":"default"}}}}"#))
        .collect();
    let text = format!(
        r#"{{"set":"shortcuts","value":{{"edits":[{}]}}}}"#,
        seventeen.join(",")
    );
    assert!(serde_json::from_str::<PreferenceChange>(&text).is_err());
}

#[test]
fn an_agents_choices_are_bounded() {
    let choice = |n: usize| {
        (
            OptionId::new(format!("o{n}")),
            ChoiceValue::Select {
                value: OptionValueId::new("v"),
            },
        )
    };
    let eight: BTreeMap<_, _> = (0..8).map(choice).collect();
    assert!(Choices::try_from(eight).is_ok());
    let nine: BTreeMap<_, _> = (0..9).map(choice).collect();
    assert!(Choices::try_from(nine).is_err());
    // Provider choice text now has its explicitly assigned256-byte bound.
    // Keep literal boundary cases here: a shared constant must not let the
    // production limit and its only expectation drift together unnoticed.
    for bytes in [65, 67, 256] {
        let fits: BTreeMap<_, _> = [(
            OptionId::new("x".repeat(bytes)),
            ChoiceValue::Toggle { on: true },
        )]
        .into_iter()
        .collect();
        assert!(Choices::try_from(fits).is_ok(), "{bytes} bytes must fit");
    }
    let long: BTreeMap<_, _> = [(
        OptionId::new("x".repeat(257)),
        ChoiceValue::Toggle { on: true },
    )]
    .into_iter()
    .collect();
    assert!(Choices::try_from(long).is_err());
    assert!(AgentKind(HarnessKind::ClaudeCode) < AgentKind(HarnessKind::Codex));
}

#[test]
fn where_they_are_kept_said_only_when_it_is_not_a_file() {
    let words = Lang::En.settings();
    assert_eq!(
        words.kept_why(&Kept::File {
            path: "/x".to_owned()
        }),
        None
    );
    assert_eq!(
        words
            .kept_why(&Kept::ThisLaunch {
                why: LaunchOnly::NoPlace
            })
            .as_deref(),
        Some("Kept until Vessel quits: this computer gives Vessel no place to keep them.")
    );
    let unreadable = words
        .kept_why(&Kept::ThisLaunch {
            why: LaunchOnly::Unreadable {
                fault: FileFault {
                    path: "/c/app-prefs.json".to_owned(),
                    at: Some((1, 2)),
                    problem: FileProblem::NotJson,
                    field: None,
                },
            },
        })
        .unwrap();
    assert!(unreadable.contains("Fix or remove it, then restart Vessel."));
    assert!(
        unreadable.contains("line 1, column 2: this is not JSON"),
        "{unreadable}"
    );
    assert_eq!(
        serde_json::to_string(&Kept::ThisLaunch {
            why: LaunchOnly::Unreadable {
                fault: FileFault {
                    path: "/c".to_owned(),
                    at: None,
                    problem: FileProblem::WrongKind,
                    field: None,
                },
            }
        })
        .unwrap(),
        r#"{"kind":"this_launch","why":{"kind":"unreadable","fault":{"path":"/c","at":null,"problem":"wrong_kind","field":null}}}"#
    );
}

/// The bridge's `bindings_have_one_spelling`.
#[test]
fn bindings_have_one_spelling() {
    for good in [
        "Mod+K",
        "Alt+[",
        "G S",
        "/",
        "Shift+/",
        "Mod+Shift+B",
        "Ctrl+Alt+Shift+Meta+F12",
        "Mod+,",
        "Alt+1",
        "N",
        "Mod+-",
        "Mod+=",
        "Mod+\\",
        "Mod+'",
        "`",
        "Mod+Enter",
        "Shift+PageDown",
        "G M",
        "Mod+K Mod+S",
    ] {
        let binding: KeyBinding = good.parse().unwrap();
        assert_eq!(binding.as_str(), good);
    }
    for (bad, why) in [
        ("k", InvalidKeyBinding::Key("k".to_owned())),
        ("?", InvalidKeyBinding::Key("?".to_owned())),
        ("Shift+Mod+K", InvalidKeyBinding::Order),
        ("Mod+", InvalidKeyBinding::NoKey),
        ("Mod+Mod+K", InvalidKeyBinding::Order),
        ("A B C", InvalidKeyBinding::Chords),
        ("Plus", InvalidKeyBinding::Key("Plus".to_owned())),
        ("Mod+K ", InvalidKeyBinding::NoKey),
        ("", InvalidKeyBinding::NoKey),
        (" ", InvalidKeyBinding::NoKey),
        ("G  S", InvalidKeyBinding::NoKey),
        ("Cmd+K", InvalidKeyBinding::Modifier("Cmd".to_owned())),
        ("mod+K", InvalidKeyBinding::Modifier("mod".to_owned())),
        ("Mod+F13", InvalidKeyBinding::Key("F13".to_owned())),
        ("Mod+Shift+?", InvalidKeyBinding::Key("?".to_owned())),
        ("Mod++", InvalidKeyBinding::NoKey),
        (
            "Ctrl+Alt+Shift+Meta+F12 Mod+Shift+B",
            InvalidKeyBinding::TooLong,
        ),
    ] {
        assert_eq!(bad.parse::<KeyBinding>(), Err(why), "{bad:?}");
    }
    let binding: KeyBinding = serde_json::from_str("\"Alt+]\"").unwrap();
    assert_eq!(serde_json::to_string(&binding).unwrap(), "\"Alt+]\"");
    assert!(serde_json::from_str::<KeyBinding>("\"Alt+}\"").is_err());
}

/// The bridge's `action_ids_are_the_pages_own_shape`.
#[test]
fn action_ids_are_the_pages_own_shape() {
    for good in ["palette", "go-sessions", "session-9", &"a".repeat(48)] {
        assert_eq!(good.parse::<ActionId>().unwrap().as_str(), good);
    }
    for bad in [
        "",
        "9-lives",
        "-x",
        "Palette",
        "go_sessions",
        "a b",
        &"a".repeat(49),
    ] {
        assert_eq!(bad.parse::<ActionId>(), Err(InvalidActionId), "{bad:?}");
    }
}

/// Word 39: Animations is kept, changed alone, and an unknown value reads
/// as the default (on) rather than failing the whole document.
#[test]
fn animations_are_kept_changed_alone_and_lenient() {
    use crate::prefs::Animations;
    let values = Preferences::default();
    assert_eq!(values.appearance.animations, Animations::On);
    let off = values
        .changed(&PreferenceChange::Animations(Animations::Off))
        .unwrap();
    assert_eq!(off.appearance.animations, Animations::Off);
    assert!(!off.appearance.animations.on());
    assert_eq!(off.appearance.theme, values.appearance.theme);
    let read: Preferences = serde_json::from_str(r#"{"appearance":{"animations":"off"}}"#).unwrap();
    assert_eq!(read.appearance.animations, Animations::Off);
    let unknown: Preferences =
        serde_json::from_str(r#"{"appearance":{"animations":"sometimes"}}"#).unwrap();
    assert_eq!(unknown.appearance.animations, Animations::On);
}
