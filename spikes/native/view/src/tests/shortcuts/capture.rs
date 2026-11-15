//! Settings › Shortcuts' recorder (Paseo's flow, the page's rules), GOAT's
//! two queue keys in the registry, and telling a repeat from a press.

use super::{OTHER, Overrides, chord, edit, keys, overrides};
use crate::prefs::ShortcutTarget;
use crate::shortcuts::{
    Action, Capture, Clash, Done, Held, Refusal, ShortcutGroup, Verdict, bindings_of, clear, reset,
};

fn recording(action: Action, pressed: &str) -> Capture {
    let mut capture = Capture::start(action).unwrap();
    capture.press(chord(pressed));
    capture
}

#[test]
fn goat_queue_keys_are_registered_in_the_session_group_with_goat_defaults() {
    let none = Overrides::new();
    for (action, id, default) in [
        (Action::SteerQueued, "steer-queued", "Mod+Shift+Enter"),
        (Action::EditQueued, "edit-queued", "Alt+Up"),
    ] {
        assert_eq!(action.id(), id);
        assert_eq!(action.group(), ShortcutGroup::Session);
        assert_eq!(action.defaults(), [default]);
        let bound = bindings_of(action, &none);
        assert_eq!(bound.len(), 1);
        assert_eq!(bound.first().unwrap().text(), default);
    }
}

#[test]
fn a_fixed_action_cannot_record_and_nothing_pressed_waits() {
    assert!(Capture::start(Action::JumpSession).is_none());
    let none = Overrides::new();
    let mut capture = Capture::start(Action::Find).unwrap();
    assert_eq!(capture.verdict(&none, OTHER), Verdict::Waiting);
    assert_eq!(capture.done(&none, OTHER), Done::Nothing);
    capture.press(chord("Mod+J"));
    capture.back();
    assert_eq!(capture.verdict(&none, OTHER), Verdict::Waiting);
}

#[test]
fn the_pages_refusals_and_clashes_keep_done_off() {
    let none = Overrides::new();
    for (pressed, verdict) in [
        ("G", Verdict::Refused(Refusal::NoModifier(OTHER))),
        ("Mod+W", Verdict::Refused(Refusal::Kept)),
        ("Mod+C", Verdict::Refused(Refusal::Editing)),
        ("Mod+F", Verdict::Clashes(Clash::Same(Action::Find))),
        ("Alt+3", Verdict::Clashes(Clash::Fixed(Action::JumpSession))),
    ] {
        let capture = recording(Action::Find, pressed);
        assert_eq!(capture.verdict(&none, OTHER), verdict, "{pressed}");
        assert_eq!(capture.done(&none, OTHER), Done::Nothing, "{pressed}");
    }
}

#[test]
fn done_replaces_the_actions_keys_whole_as_paseos_rebind() {
    let none = Overrides::new();
    let capture = recording(Action::Palette, "Mod+J");
    assert_eq!(capture.verdict(&none, OTHER), Verdict::Ready);
    // Paseo's row has one binding: the palette's `/` goes with Mod+K.
    assert_eq!(
        capture.done(&none, OTHER),
        Done::Save(vec![edit(Action::Palette, keys(&["Mod+J"]))])
    );
}

#[test]
fn a_key_another_action_has_is_moved_only_after_asking() {
    let none = Overrides::new();
    let capture = recording(Action::Find, "Mod+K");
    assert_eq!(
        capture.verdict(&none, OTHER),
        Verdict::Reassign(Action::Palette)
    );
    assert_eq!(
        capture.done(&none, OTHER),
        Done::Ask {
            from: Action::Palette,
            edits: vec![
                edit(Action::Find, keys(&["Mod+K"])),
                edit(Action::Palette, keys(&["/"])),
            ],
        }
    );
    // A key the person gave earlier is found as well as a default.
    let moved = overrides(&[("settings", &["Mod+J"])]);
    let capture = recording(Action::Find, "Mod+J");
    assert_eq!(
        capture.verdict(&moved, OTHER),
        Verdict::Reassign(Action::Settings)
    );
}

#[test]
fn clear_leaves_no_key_and_reset_restores_the_default() {
    assert_eq!(clear(Action::Find), [edit(Action::Find, keys(&[]))]);
    assert_eq!(
        reset(Action::Find),
        [edit(Action::Find, ShortcutTarget::Default)]
    );
}

#[test]
fn a_press_of_the_held_chord_is_a_repeat_until_a_key_comes_up() {
    let mut held = Held::default();
    let steer = chord("Mod+Shift+Enter");
    assert!(!held.press(&steer));
    assert!(held.press(&steer));
    assert!(held.press(&steer));
    held.release();
    assert!(!held.press(&steer));
    assert!(!held.press(&chord("Alt+Up")));
}
