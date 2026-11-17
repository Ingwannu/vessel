use vessel_proto::HarnessKind;

use super::*;
use crate::wire::{
    Choices, InterfaceSize, KeyBinding, Notifications, Panel, SendKey, ShortcutEdit, ShortcutEdits,
    Theme, TurnDelivery,
};

fn read(text: &str) -> PrefsDoc {
    match PrefsDoc::decode(text.as_bytes()).unwrap() {
        Decoded::Current(doc) => doc,
        Decoded::Newer { version, .. } => panic!("read as newer ({version})"),
    }
}

fn written(doc: &PrefsDoc) -> String {
    String::from_utf8(doc.encode().unwrap()).unwrap()
}

/// `text` without its whitespace (outside strings: the tests' texts have
/// none inside), to compare what is written with what was meant.
fn compact(text: &str) -> String {
    text.chars().filter(|c| !c.is_whitespace()).collect()
}

const NODE: &str = "0d4b6a35-37d2-4d5c-9f3e-6f1a2b7c8d90";

#[test]
fn an_empty_or_missing_file_is_every_default() {
    assert_eq!(read("{}").values(), Preferences::default());
    assert_eq!(read(r#"{"version":1}"#).values(), Preferences::default());
    assert_eq!(compact(&written(&PrefsDoc::default())), r#"{"version":1}"#);
    for bad in [
        "",
        "[]",
        "3",
        "{",
        r#"{"version":"1"}"#,
        r#"{"version":1.5}"#,
    ] {
        assert!(PrefsDoc::decode(bad.as_bytes()).is_err(), "{bad:?}");
    }
    assert!(matches!(
        PrefsDoc::decode(br#"{"version":0}"#),
        Err(DecodeError::Version(0))
    ));
}

#[test]
fn unknown_keys_at_every_level_are_kept_on_rewrite() {
    let text = format!(
        r#"{{"version":1,"later":{{"x":[1,2]}},
            "appearance":{{"theme":"dark","glow":true}},
            "defaults":{{"{NODE}":{{"agent":"codex","pinned":3}}}}}}"#
    );
    let mut doc = read(&text);
    assert_eq!(doc.values().appearance.theme, Theme::Dark);
    doc = doc
        .changed(&PreferenceChange::Send(SendKey::ModEnter))
        .unwrap();
    assert_eq!(
        compact(&written(&doc)),
        compact(&format!(
            r#"{{"version":1,"appearance":{{"theme":"dark","glow":true}},"composer":{{"send":"mod_enter"}},
                "defaults":{{"{NODE}":{{"agent":"codex","pinned":3}}}},"later":{{"x":[1,2]}}}}"#
        ))
    );
}

#[test]
fn the_delivery_is_the_composers_and_keeps_the_send_key() {
    let mut doc = read(r#"{"version":1,"composer":{"send":"mod_enter"}}"#);
    // Never set: the default (ADR 0108).
    assert_eq!(doc.values().composer.delivery, TurnDelivery::Steer);
    doc = doc
        .changed(&PreferenceChange::Delivery(TurnDelivery::Interrupt))
        .unwrap();
    assert_eq!(doc.values().composer.send, SendKey::ModEnter);
    assert_eq!(doc.values().composer.delivery, TurnDelivery::Interrupt);
    assert_eq!(
        compact(&written(&doc)),
        compact(r#"{"version":1,"composer":{"send":"mod_enter","delivery":"interrupt"}}"#)
    );
    // A value a newer page wrote reads as the default and is kept.
    let doc = read(r#"{"version":1,"composer":{"delivery":"later"}}"#);
    assert_eq!(doc.values().composer.delivery, TurnDelivery::Steer);
}

/// ADR 0108: the default moved from Queue to Steer. A document that never
/// set the delivery reads as Steer and is not rewritten for it; a Queue
/// someone chose stays Queue, written as it was.
#[test]
fn a_chosen_queue_survives_the_steer_default() {
    let unset = read(r#"{"version":1,"composer":{"send":"enter"}}"#);
    assert_eq!(unset.values().composer.delivery, TurnDelivery::Steer);
    assert_eq!(
        compact(&written(&unset)),
        compact(r#"{"version":1,"composer":{"send":"enter"}}"#)
    );
    let chosen = read(r#"{"version":1,"composer":{"delivery":"queue"}}"#);
    assert_eq!(chosen.values().composer.delivery, TurnDelivery::Queue);
    assert_eq!(
        compact(&written(&chosen)),
        compact(r#"{"version":1,"composer":{"delivery":"queue"}}"#)
    );
}

#[test]
fn an_unreadable_value_is_kept_raw_and_read_as_its_default() {
    let doc = read(
        r#"{"appearance":{"theme":"sepia","interface_size":40,"code_size":13},
            "notifications":"loud","layout":{"sidebar":"hidden"}}"#,
    );
    let values = doc.values();
    assert_eq!(values.appearance.theme, Theme::System);
    assert_eq!(values.appearance.interface_size, InterfaceSize::default());
    assert_eq!(values.appearance.code_size.get(), 13);
    assert_eq!(values.notifications, Notifications::default());
    assert!(
        values.notifications.title,
        "the title count is on by default"
    );
    assert_eq!(values.layout.sidebar, Panel::Hidden);
    // Written back as it was.
    let text = compact(&written(&doc));
    assert!(text.contains(r#""theme":"sepia""#), "{text}");
    assert!(text.contains(r#""interface_size":40"#), "{text}");
    assert!(text.contains(r#""notifications":"loud""#), "{text}");
    // A change to the setting replaces it; one to a section that did not
    // read makes it an object holding the change.
    let doc = doc
        .changed(&PreferenceChange::Theme(Theme::Light))
        .unwrap()
        .changed(&PreferenceChange::TitleCount(false))
        .unwrap();
    let text = compact(&written(&doc));
    assert!(text.contains(r#""theme":"light""#), "{text}");
    assert!(text.contains(r#""interface_size":40"#), "{text}");
    assert!(
        text.contains(r#""notifications":{"title":false}"#),
        "{text}"
    );
    assert!(!doc.values().notifications.title);
}

#[test]
fn an_unknown_agent_kind_or_choice_kind_is_kept_whole() {
    let text = format!(
        r#"{{"defaults":{{
            "{NODE}":{{"agent":"cursor","agents":{{
                "cursor":{{"profile":null,"choices":{{}}}},
                "codex":{{"profile":"work","choices":{{"model":{{"kind":"select","value":"o5"}}}}}},
                "gemini":{{"profile":null,"choices":{{"depth":{{"kind":"slider","value":3}}}}}}
            }}}},
            "not-a-node":{{"agent":"codex"}}
        }}}}"#
    );
    let doc = read(&text);
    let values = doc.values();
    let node: NodeId = NODE.parse().unwrap();
    let machine = &values.defaults[&node];
    // `cursor` is not a kind this build knows: the page sees no default
    // agent, and only the agent it can read.
    assert_eq!(machine.agent, None);
    let agents: Vec<_> = machine.agents.keys().map(|kind| kind.0).collect();
    assert_eq!(agents, [HarnessKind::Codex]);
    assert_eq!(values.defaults.len(), 1);
    // A change to another agent keeps them all.
    let change = PreferenceChange::AgentDefaults {
        node,
        agent: HarnessKind::ClaudeCode,
        profile: None,
        choices: Choices::new(),
    };
    let text = compact(&written(&doc.changed(&change).unwrap()));
    for kept in [
        r#""agent":"cursor""#,
        r#""cursor":{"profile":null,"choices":{}}"#,
        r#""kind":"slider""#,
        r#""not-a-node":{"agent":"codex"}"#,
        r#""claude-code":{"profile":null,"choices":{}}"#,
    ] {
        assert!(text.contains(kept), "{kept} in {text}");
    }
    // Forgetting the machine forgets what was kept for it too.
    let forgotten = doc
        .changed(&PreferenceChange::ForgetDefaults { node })
        .unwrap();
    let text = compact(&written(&forgotten));
    assert!(
        !text.contains("cursor") && text.contains("not-a-node"),
        "{text}"
    );
}

#[test]
fn a_newer_file_is_read_for_this_launch() {
    let decoded = PrefsDoc::decode(br#"{"version":2,"appearance":{"theme":"dark"}}"#).unwrap();
    let Decoded::Newer { version, doc } = decoded else {
        panic!("{decoded:?}");
    };
    assert_eq!(version, 2);
    assert_eq!(doc.values().appearance.theme, Theme::Dark);
}

fn edit(action: &str, keys: &[&str]) -> ShortcutEdit {
    let keys = keys.iter().map(|key| key.parse().unwrap()).collect();
    ShortcutEdit {
        action: action.parse().unwrap(),
        to: ShortcutTarget::Bindings {
            keys: Bindings::new(keys).unwrap(),
        },
    }
}

fn shortcuts(edits: Vec<ShortcutEdit>) -> PreferenceChange {
    PreferenceChange::Shortcuts {
        edits: ShortcutEdits::new(edits).unwrap(),
    }
}

#[test]
fn shortcut_edits_apply_together_and_reset_clears_unknown_actions() {
    let doc = read(r#"{"shortcuts":{"later-action":"Mod+J","palette":["Mod+K"]}}"#);
    // A reassignment moves a key in one change.
    let doc = doc
        .changed(&shortcuts(vec![
            edit("palette", &["Mod+P"]),
            edit("find", &["Mod+K"]),
            edit("sidebar", &[]),
        ]))
        .unwrap();
    let values = doc.values();
    let keys = |action: &str| {
        values.shortcuts[&action.parse::<ActionId>().unwrap()]
            .items()
            .iter()
            .map(KeyBinding::to_string)
            .collect::<Vec<_>>()
    };
    assert_eq!(keys("palette"), ["Mod+P"]);
    assert_eq!(keys("find"), ["Mod+K"]);
    assert!(keys("sidebar").is_empty(), "an empty list is no keys");
    assert!(compact(&written(&doc)).contains(r#""later-action":"Mod+J""#));
    let reset = doc.changed(&PreferenceChange::ResetShortcuts).unwrap();
    assert!(reset.values().shortcuts.is_empty());
    assert_eq!(compact(&written(&reset)), r#"{"version":1}"#);
}

#[test]
fn counts_are_refused_only_when_a_change_grows_past_them() {
    let mut doc = PrefsDoc::default();
    for n in 0..64 {
        doc = doc
            .changed(&shortcuts(vec![edit(&format!("a{n}"), &["N"])]))
            .unwrap();
    }
    assert_eq!(
        doc.changed(&shortcuts(vec![edit("one-more", &["N"])])),
        Err(Refused::Actions)
    );
    // Changing an action already kept does not grow the count.
    assert!(doc.changed(&shortcuts(vec![edit("a3", &["M"])])).is_ok());

    let mut doc = PrefsDoc::default();
    let agent = |node: NodeId| PreferenceChange::DefaultAgent {
        node,
        agent: Some(HarnessKind::Codex),
    };
    for _ in 0..32 {
        doc = doc.changed(&agent(NodeId::random())).unwrap();
    }
    assert_eq!(
        doc.changed(&agent(NodeId::random())),
        Err(Refused::Machines)
    );
    // Forgetting a default agent leaves nothing for the machine: it goes.
    let node = *doc.values().defaults.keys().next().unwrap();
    let doc = doc
        .changed(&PreferenceChange::DefaultAgent { node, agent: None })
        .unwrap();
    assert_eq!(doc.values().defaults.len(), 31);
}

#[test]
fn a_document_past_its_size_takes_only_changes_that_do_not_grow_it() {
    let bound = usize::try_from(MAX_FILE).unwrap();
    // A newer page's unknown keys fill the file past the bound.
    let text = format!(
        r#"{{"later":"{}","appearance":{{"theme":"light"}}}}"#,
        "x".repeat(bound)
    );
    let doc = read(&text);
    assert!(doc.encode().unwrap().len() > bound);
    // The same size ("light" again) and smaller ("dark") are taken.
    assert!(doc.changed(&PreferenceChange::Theme(Theme::Light)).is_ok());
    assert!(doc.changed(&PreferenceChange::Theme(Theme::Dark)).is_ok());
    // Larger is refused, and nothing changed.
    assert_eq!(
        doc.changed(&PreferenceChange::Send(SendKey::ModEnter)),
        Err(Refused::Size)
    );
}

#[test]
fn missing_ended_preference_lists_ended_sessions_without_rewriting_the_document() {
    for text in [
        "{}",
        r#"{"sidebar":{}}"#,
        r#"{"sidebar":{"group":"status"}}"#,
    ] {
        let doc = read(text);
        assert!(doc.values().sidebar.ended, "{text}");
        assert!(read(&written(&doc)).values().sidebar.ended);
        assert!(
            !written(&doc).contains("ended"),
            "a missing choice stays absent"
        );
    }
}

#[test]
fn an_explicit_false_ended_preference_survives_reads_and_other_changes() {
    let doc = read(r#"{"sidebar":{"ended":false}}"#);
    assert!(!doc.values().sidebar.ended);
    let changed = doc.changed(&PreferenceChange::Theme(Theme::Dark)).unwrap();
    assert!(!read(&written(&changed)).values().sidebar.ended);
    assert!(compact(&written(&changed)).contains(r#""ended":false"#));
    let on = changed
        .changed(&PreferenceChange::SidebarEnded(true))
        .unwrap();
    assert!(read(&written(&on)).values().sidebar.ended);
}
