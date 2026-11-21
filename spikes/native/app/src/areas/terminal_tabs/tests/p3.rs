//! P3 in a terminal tab: find, selection and copy, the paste queue, the
//! scrollback setting applied to an open tab.

use gpui::{KeyDownEvent, Keystroke, TestAppContext};
use vessel_app_link::terminals::{Reply, Request};
use vessel_app_view::terminal::grid::Scroll;
use vessel_app_view::terminal::{
    GridSize, MAX_PENDING, Origin, Outgoing, ScrollbackLines, SelectBy, Selection,
};

use super::super::find::Count;
use super::super::select::Selected;
use super::super::{Pane, PaneKey, Stage};
use super::{answer, open, sent};
use crate::shell::Shell;

/// The platform's Mod+F (Cmd on macOS, Ctrl elsewhere).
const MOD_F: &str = if cfg!(target_os = "macos") {
    "cmd-f"
} else {
    "ctrl-f"
};

pub(super) fn down(stroke: &'static str) -> KeyDownEvent {
    KeyDownEvent {
        keystroke: Keystroke::parse(stroke).unwrap(),
        is_held: false,
        prefer_character_input: false,
    }
}

pub(super) fn laptop(terminal: vessel_proto::TerminalId) -> PaneKey {
    PaneKey {
        machine: "laptop".to_owned(),
        terminal,
    }
}

/// A live pane 20 × 4 whose screen shows `bytes`.
pub(super) fn live(
    shell: &mut Shell,
    key: &PaneKey,
    bytes: &[u8],
    cx: &mut gpui::Context<'_, Shell>,
) {
    let mut pane = Pane::new(Origin::Opened, ScrollbackLines::default(), cx, key);
    pane.stage = Stage::Live;
    if let Some(model) = pane.model.as_mut() {
        model.resize(GridSize::new(20, 4).unwrap());
        let mut rest = bytes;
        while !rest.is_empty() {
            let taken = model.replay(rest).consumed;
            rest = rest.get(taken..).unwrap_or_default();
        }
    }
    let _none = shell.areas.terminal_tabs.panes.insert(key.clone(), pane);
}

/// Every send queued, answered one by one, as the keys they carry.
pub(super) fn drained(
    probe: &gpui::Entity<super::Probe>,
    shell: &gpui::Entity<Shell>,
    key: &PaneKey,
    cx: &mut gpui::VisualTestContext,
) -> String {
    let mut keys = String::new();
    loop {
        let requests = sent(probe, cx);
        if requests.is_empty() {
            return keys;
        }
        for request in requests {
            if let Request::Send {
                outgoing: Outgoing::Keys(text),
                ..
            } = request
            {
                keys.push_str(&text);
            }
            shell.update_in(cx, |shell, _window, cx| {
                answer(
                    shell,
                    Reply::Sent {
                        terminal: key.terminal,
                        result: Ok(()),
                    },
                    cx,
                );
            });
        }
    }
}

/// Coordinator, P3 question 3: Mod+F opens the terminal's bar while a
/// terminal tab (or its bar) has the keyboard, and is the conversation's
/// anywhere else.
#[gpui::test]
fn mod_f_is_the_terminals_only_while_a_terminal_has_the_keyboard(cx: &mut TestAppContext) {
    let (probe, cx) = open(cx);
    let session = probe.read_with(cx, |probe, _| probe.2);
    let shell = probe.read_with(cx, |probe, _| probe.0.clone());
    let key = laptop(super::shell_info(session).id);
    shell.update_in(cx, |shell, window, cx| {
        live(shell, &key, b"hello", cx);
        // Elsewhere (the composer): the conversation's.
        shell.areas.session.composer.focus(window, cx);
        assert!(!shell.find_goes_to_terminal(window, cx));
        // The terminal has the keyboard: its own bar opens.
        shell.terminal_tab_focus(&key, window, cx);
        assert!(shell.find_goes_to_terminal(window, cx));
        shell.terminal_tab_key(&key, &down(MOD_F), window, cx);
        assert!(
            shell
                .areas
                .terminal_tabs
                .panes
                .get(&key)
                .is_some_and(|pane| pane.find.is_some())
        );
        // Its bar holds the keyboard now, and Mod+F is still the terminal's.
        assert!(shell.terminal_find_focused(window, cx));
        assert!(shell.find_goes_to_terminal(window, cx));
        // Escape's close gives the terminal the keyboard back.
        shell.terminal_find_close(&key, window, cx);
        assert!(
            shell
                .areas
                .terminal_tabs
                .panes
                .get(&key)
                .is_some_and(|pane| pane.find.is_none())
        );
        assert!(shell.terminal_focused(window));
    });
}

/// Keys typed in the open bar are the bar's. The bar is drawn inside the
/// terminal's surface, so its field's Enter, Backspace and arrows bubble up
/// to the terminal's handler, which leaves them (a capture found Enter
/// reaching the shell as well as moving to the next match). Once the bar
/// closes, Enter is the shell's again.
#[gpui::test]
fn keys_typed_in_the_find_bar_never_reach_the_shell(cx: &mut TestAppContext) {
    let (probe, cx) = open(cx);
    let session = probe.read_with(cx, |probe, _| probe.2);
    let shell = probe.read_with(cx, |probe, _| probe.0.clone());
    let key = laptop(super::shell_info(session).id);
    shell.update_in(cx, |shell, window, cx| {
        live(shell, &key, b"$ ", cx);
        shell.terminal_tab_focus(&key, window, cx);
        shell.terminal_find_open(&key, window, cx);
        assert!(shell.terminal_find_focused(window, cx));
        for stroke in ["enter", "backspace", "up"] {
            shell.terminal_tab_key(&key, &down(stroke), window, cx);
        }
    });
    assert_eq!(
        drained(&probe, &shell, &key, cx),
        "",
        "the bar's keys stay in the bar"
    );
    shell.update_in(cx, |shell, window, cx| {
        shell.terminal_find_close(&key, window, cx);
        shell.terminal_tab_key(&key, &down("enter"), window, cx);
    });
    assert_eq!(drained(&probe, &shell, &key, cx), "\r");
}

/// Enter and Shift+Enter go round the matches, from the first at or below
/// the view's top; the counter says which.
#[gpui::test]
fn find_counts_and_moves_round_the_matches(cx: &mut TestAppContext) {
    let (probe, cx) = open(cx);
    let session = probe.read_with(cx, |probe, _| probe.2);
    let shell = probe.read_with(cx, |probe, _| probe.0.clone());
    let key = laptop(super::shell_info(session).id);
    shell.update_in(cx, |shell, window, cx| {
        live(shell, &key, b"hit\r\nmiss\r\nHIT\r\nhit", cx);
        shell.terminal_find_open(&key, window, cx);
        let source = shell
            .areas
            .terminal_tabs
            .panes
            .get(&key)
            .and_then(|pane| pane.find.as_ref())
            .map(|find| find.field.id())
            .unwrap();
        shell.terminal_find_changed(source, "hit".to_owned(), cx);
        let count = |shell: &Shell| {
            shell
                .areas
                .terminal_tabs
                .panes
                .get(&key)
                .and_then(|pane| pane.find.as_ref())
                .map(super::super::find::PaneFind::count)
        };
        assert_eq!(
            count(shell),
            Some(Count::At("1".to_owned(), "3".to_owned()))
        );
        shell.terminal_find_move(&key, true, cx);
        assert_eq!(
            count(shell),
            Some(Count::At("2".to_owned(), "3".to_owned()))
        );
        shell.terminal_find_move(&key, false, cx);
        shell.terminal_find_move(&key, false, cx);
        assert_eq!(
            count(shell),
            Some(Count::At("3".to_owned(), "3".to_owned()))
        );
        shell.terminal_find_changed(source, "nothing".to_owned(), cx);
        assert_eq!(count(shell), Some(Count::None));
        shell.terminal_find_changed(source, String::new(), cx);
        assert_eq!(count(shell), Some(Count::Nothing));
    });
}

/// Ctrl+C copies a selection and sends nothing; without one it is the
/// shell's ^C (Paseo's `handleCtrlClipboardShortcut`, off macOS). Typing
/// clears the selection.
#[gpui::test]
fn ctrl_c_copies_a_selection_and_is_the_shells_without_one(cx: &mut TestAppContext) {
    if cfg!(target_os = "macos") {
        return;
    }
    let (probe, cx) = open(cx);
    let session = probe.read_with(cx, |probe, _| probe.2);
    let shell = probe.read_with(cx, |probe, _| probe.0.clone());
    let key = laptop(super::shell_info(session).id);
    shell.update_in(cx, |shell, window, cx| {
        live(shell, &key, b"copy me", cx);
        let pane = shell.areas.terminal_tabs.panes.get_mut(&key).unwrap();
        let grid = pane.model.as_ref().unwrap().grid();
        let mut selection = Selection::new(grid.text_at(Scroll(0), 0, 0), SelectBy::Cells);
        selection.extend(grid.text_at(Scroll(0), 0, 3));
        pane.selected = Some(Selected::on(selection, grid));
        shell.terminal_tab_key(&key, &down("ctrl-c"), window, cx);
    });
    assert!(sent(&probe, cx).is_empty(), "a copy sends nothing");
    let copied = cx.read_from_clipboard().and_then(|item| item.text());
    assert_eq!(copied.as_deref(), Some("copy"));
    shell.update_in(cx, |shell, window, cx| {
        shell.terminal_tab_type(&key, "x", cx);
        assert!(
            shell
                .areas
                .terminal_tabs
                .panes
                .get(&key)
                .is_some_and(|pane| pane.selected.is_none())
        );
        shell.terminal_tab_key(&key, &down("ctrl-c"), window, cx);
    });
    assert_eq!(drained(&probe, &shell, &key, cx), "x\u{3}");
}

/// Coordinator, P3 question 2: a paste past the typed keys' room, then a
/// key and Ctrl+C. The order holds, the paste arrives whole, and nothing is
/// copied or cancelled while it goes.
#[gpui::test]
fn a_paste_past_the_room_then_a_key_and_ctrl_c_keep_their_order(cx: &mut TestAppContext) {
    let (probe, cx) = open(cx);
    let session = probe.read_with(cx, |probe, _| probe.2);
    let shell = probe.read_with(cx, |probe, _| probe.0.clone());
    let key = laptop(super::shell_info(session).id);
    let paste = "p".repeat(MAX_PENDING.saturating_add(1000));
    shell.update_in(cx, |shell, window, cx| {
        live(shell, &key, b"$ ", cx);
        shell.terminal_tab_send_paste(&key, paste.clone(), cx);
        shell.terminal_tab_type(&key, "k", cx);
        shell.terminal_tab_key(&key, &down("ctrl-c"), window, cx);
    });
    let keys = drained(&probe, &shell, &key, cx);
    assert_eq!(keys.len(), paste.len().saturating_add(2));
    assert!(keys.starts_with(&paste));
    assert!(keys.ends_with("k\u{3}"));
    assert!(cx.read_from_clipboard().is_none(), "nothing was copied");
}

/// The setting applies to an open tab at once: a lower bound drops the
/// oldest lines.
#[gpui::test]
fn a_scrollback_change_applies_to_an_open_tab(cx: &mut TestAppContext) {
    let (probe, cx) = open(cx);
    let session = probe.read_with(cx, |probe, _| probe.2);
    let shell = probe.read_with(cx, |probe, _| probe.0.clone());
    let key = laptop(super::shell_info(session).id);
    shell.update_in(cx, |shell, _window, cx| {
        live(shell, &key, b"1\r\n2\r\n3\r\n4\r\n5\r\n6\r\n7", cx);
        let held = |shell: &Shell| {
            shell
                .areas
                .terminal_tabs
                .panes
                .get(&key)
                .and_then(|pane| pane.model.as_ref())
                .map(|model| model.grid().history_len())
        };
        assert_eq!(held(shell), Some(3));
        shell.terminal_scrollback(ScrollbackLines::clamped(1), cx);
        assert_eq!(held(shell), Some(1));
        assert_eq!(
            shell
                .areas
                .terminal_tabs
                .panes
                .get(&key)
                .map(|pane| pane.scrollback),
            Some(ScrollbackLines::clamped(1))
        );
    });
}

/// P3 app review note 1: a paste refused past the pastes' bound is said,
/// once for a run of refusals, and nothing of it is sent.
#[gpui::test]
fn a_paste_past_its_bound_is_said_once(cx: &mut TestAppContext) {
    let (probe, cx) = open(cx);
    let session = probe.read_with(cx, |probe, _| probe.2);
    let shell = probe.read_with(cx, |probe, _| probe.0.clone());
    let key = laptop(super::shell_info(session).id);
    let said = |shell: &Shell| shell.areas.toasts.shown().map(|toast| toast.text.clone());
    shell.update_in(cx, |shell, _window, cx| {
        live(shell, &key, b"$ ", cx);
        // A paste waits behind one in flight: fill the bound, then pass it.
        shell.terminal_tab_send_paste(&key, "a".to_owned(), cx);
        shell.terminal_tab_send_paste(
            &key,
            "p".repeat(vessel_app_view::terminal::MAX_PASTE_PENDING),
            cx,
        );
        assert_eq!(said(shell), None);
        shell.terminal_tab_send_paste(&key, "q".to_owned(), cx);
        let refused = shell.words.terminal_tabs().paste_too_large().to_owned();
        assert_eq!(said(shell), Some(refused));
        shell.areas.toasts = vessel_app_view::toasts::Toasts::default();
        shell.terminal_tab_send_paste(&key, "r".to_owned(), cx);
        assert_eq!(said(shell), None, "said once for a run of refusals");
    });
}

/// TRACKER 836: a press whose release was never heard (it came before the
/// frame that listens window-wide) ends at the next move with the button
/// up. A plain click so ended selects nothing, and a later move extends
/// nothing; with the button down, the drag goes on.
#[gpui::test]
fn a_move_with_the_button_up_ends_a_press_that_missed_its_release(cx: &mut TestAppContext) {
    let (probe, cx) = open(cx);
    let session = probe.read_with(cx, |probe, _| probe.2);
    let shell = probe.read_with(cx, |probe, _| probe.0.clone());
    let key = laptop(super::shell_info(session).id);
    let moved = |pressed_button| gpui::MouseMoveEvent {
        position: gpui::point(gpui::px(5.0), gpui::px(5.0)),
        pressed_button,
        modifiers: gpui::Modifiers::none(),
    };
    shell.update_in(cx, |shell, _window, cx| {
        live(shell, &key, b"select me", cx);
        let pane = shell.areas.terminal_tabs.panes.get_mut(&key).unwrap();
        let grid = pane.model.as_ref().unwrap().grid();
        let at = grid.text_at(Scroll(0), 0, 2);
        pane.selected = Some(Selected::on(Selection::new(at, SelectBy::Cells), grid));
        pane.selecting = true;
        shell.terminal_tab_pointer_moved(&key, &moved(Some(gpui::MouseButton::Left)), cx);
        let panes = &shell.areas.terminal_tabs.panes;
        assert!(panes.get(&key).is_some_and(|pane| pane.selecting));
        shell.terminal_tab_pointer_moved(&key, &moved(None), cx);
        let pane = shell.areas.terminal_tabs.panes.get(&key);
        assert!(pane.is_some_and(|pane| !pane.selecting), "the press ended");
        assert!(
            pane.is_some_and(|pane| pane.selected.is_none()),
            "a click selects nothing"
        );
    });
}
