//! P4 in a terminal tab: the virtual key bar at compact widths, its keys
//! and its sticky modifiers (Paseo's `terminal-pane.tsx` 795-830,
//! 1052-1083).

use gpui::{TestAppContext, size};
use vessel_app_kit::ui::px;
use vessel_app_kit::{Scheme, Tokens};
use vessel_app_view::terminal::key_bar::{BarKey, Control, Modifier, Sticky};

use super::p3::{down, drained, laptop, live};
use super::{open, sent};

fn sticky(shell: &crate::shell::Shell, key: &super::super::PaneKey) -> Sticky {
    shell
        .areas
        .terminal_tabs
        .panes
        .get(key)
        .map(|pane| pane.sticky)
        .unwrap_or_default()
}

#[gpui::test]
fn a_bar_key_goes_out_with_the_set_modifier_which_then_clears(cx: &mut TestAppContext) {
    let (probe, cx) = open(cx);
    let session = probe.read_with(cx, |probe, _| probe.2);
    let shell = probe.read_with(cx, |probe, _| probe.0.clone());
    let key = laptop(super::shell_info(session).id);
    shell.update_in(cx, |shell, window, cx| {
        live(shell, &key, b"$ ", cx);
        shell.terminal_key_bar_press(&key, Control::Modifier(Modifier::Ctrl), window, cx);
        assert!(sticky(shell, &key).is_set(Modifier::Ctrl));
        shell.terminal_key_bar_press(&key, Control::Key(BarKey::Up), window, cx);
        assert!(sticky(shell, &key).is_empty());
        shell.terminal_key_bar_press(&key, Control::Key(BarKey::Escape), window, cx);
        shell.terminal_key_bar_press(&key, Control::Key(BarKey::Enter), window, cx);
    });
    assert_eq!(drained(&probe, &shell, &key, cx), "\x1b[1;5A\x1b\r");
}

#[gpui::test]
fn a_set_modifier_takes_the_next_character_typed_or_pressed(cx: &mut TestAppContext) {
    let (probe, cx) = open(cx);
    let session = probe.read_with(cx, |probe, _| probe.2);
    let shell = probe.read_with(cx, |probe, _| probe.0.clone());
    let key = laptop(super::shell_info(session).id);
    shell.update_in(cx, |shell, window, cx| {
        live(shell, &key, b"$ ", cx);
        // Typed (the IME's commit): Ctrl then `c` is ^C; the next `c` is
        // a `c` again.
        shell.terminal_key_bar_press(&key, Control::Modifier(Modifier::Ctrl), window, cx);
        shell.terminal_tab_type_text(&key, "c", cx);
        shell.terminal_tab_type_text(&key, "c", cx);
        // Pressed on the keyboard: Alt then `x` is ESC x.
        shell.terminal_key_bar_press(&key, Control::Modifier(Modifier::Alt), window, cx);
        shell.terminal_tab_key(&key, &down("x"), window, cx);
        assert!(sticky(shell, &key).is_empty());
    });
    assert_eq!(drained(&probe, &shell, &key, cx), "\u{3}c\x1bx");
}

#[gpui::test]
fn without_a_set_modifier_typing_is_as_before(cx: &mut TestAppContext) {
    let (probe, cx) = open(cx);
    let session = probe.read_with(cx, |probe, _| probe.2);
    let shell = probe.read_with(cx, |probe, _| probe.0.clone());
    let key = laptop(super::shell_info(session).id);
    shell.update_in(cx, |shell, window, cx| {
        live(shell, &key, b"$ ", cx);
        // A plain letter key is left to the text input.
        shell.terminal_tab_key(&key, &down("x"), window, cx);
        shell.terminal_tab_type_text(&key, "ls", cx);
    });
    assert_eq!(drained(&probe, &shell, &key, cx), "ls");
    assert!(sent(&probe, cx).is_empty());
}

#[gpui::test]
fn the_bar_is_drawn_only_when_the_window_is_compact(cx: &mut TestAppContext) {
    let (probe, cx) = open(cx);
    let session = probe.read_with(cx, |probe, _| probe.2);
    let shell = probe.read_with(cx, |probe, _| probe.0.clone());
    let key = laptop(super::shell_info(session).id);
    let tokens = Tokens::of(Scheme::Dark);
    cx.simulate_resize(size(px(1280.0), px(800.0)));
    shell.update_in(cx, |_shell, window, cx| {
        let bar = super::super::key_bar::bar(Sticky::default(), &key, &tokens, window, cx);
        assert!(bar.is_none(), "wide: no bar");
    });
    cx.simulate_resize(size(px(390.0), px(800.0)));
    shell.update_in(cx, |_shell, window, cx| {
        let bar = super::super::key_bar::bar(Sticky::default(), &key, &tokens, window, cx);
        assert!(bar.is_some(), "compact: the bar");
    });
}
