//! The row menu's Labels page (TRACKER 791): on a wide window the
//! workspace picker beside the menu, the keys moved in only by a press, its
//! dismissal closing the menu; on a phone the picker's sheet replacing the
//! menu's and leading back to it.
use std::time::Duration;

use super::*;
use crate::areas::labels::fixtures::{Sent, followed_session, picker_query};
use gpui::{Entity, Modifiers, TestAppContext, VisualTestContext, size};
use vessel_app_kit::submenu::{HOVER_CLOSE, HOVER_OPEN};
use vessel_app_kit::ui::{Bounds, point, px};
use vessel_app_link::Link;
use vessel_app_view::board::CardDetail;
use vessel_app_view::{Attention, Phrase};
use vessel_proto::{SessionId, WorkspaceId};

const COMMANDS: usize = 8;
const HEIGHT: f32 = 844.0;
/// Half its height: a centred dialog covers it.
const MIDDLE: f32 = 422.0;

fn card(session: SessionId) -> Card {
    Card {
        session,
        structured: true,
        live: true,
        title: "Row".into(),
        machine: "fixture".into(),
        agent: "Fixture".into(),
        lane: None,
        phrase: Phrase::Starting,
        attention: Attention::AtRest,
        at: chrono::Utc::now(),
        detail: CardDetail::default(),
    }
}

/// A shell `width` wide whose followed session's row menu is open; below
/// the compact width the sidebar is its sheet, so the menu is a phone's.
fn opened(cx: &mut TestAppContext, width: f32) -> (Entity<Shell>, &mut VisualTestContext, Sent) {
    crate::shell::language_tests::init(cx);
    let (link, mut sent) = Link::detached(COMMANDS).unwrap();
    let (shell, cx) = cx.add_window_view(move |window, cx| Shell::with_test_link(window, cx, link));
    cx.simulate_resize(size(px(width), px(HEIGHT)));
    cx.run_until_parked();
    let session = SessionId::random();
    shell.update_in(cx, |shell, window, cx| {
        let _connection = followed_session(shell, &mut sent, session, WorkspaceId::random(), cx);
        if width < vessel_app_kit::picker::COMPACT {
            shell.toggle_sidebar(window, cx);
        }
        shell.row_menu_open(&card(session), point(px(40.0), px(120.0)), window, cx);
    });
    cx.run_until_parked();
    (shell, cx, sent)
}

fn labels_row(shell: &Entity<Shell>, cx: &mut VisualTestContext) -> Bounds<Pixels> {
    shell
        .read_with(cx, |shell, _| {
            shell.areas.sidebar.rows.pages.trigger(LABELS)
        })
        .unwrap()
}

/// The menu open, its Labels page open, and the picker's place.
fn state(shell: &Entity<Shell>, cx: &mut VisualTestContext) -> (bool, bool, RowPicker) {
    shell.read_with(cx, |shell, _| {
        let rows = &shell.areas.sidebar.rows;
        (
            rows.menu.is_open(),
            rows.pages.is_open(LABELS, 0),
            shell.areas.labels.row_picker(),
        )
    })
}

fn query(shell: &Entity<Shell>, cx: &mut VisualTestContext) -> String {
    cx.update(|_, cx| picker_query(shell.read(cx), cx))
}

fn wait(cx: &VisualTestContext, time: Duration) {
    cx.executor().advance_clock(time);
    cx.run_until_parked();
}

#[gpui::test]
fn a_press_on_labels_opens_the_picker_beside_with_the_keys_and_escape_ends_both(
    cx: &mut TestAppContext,
) {
    let (shell, cx, _sent) = opened(cx, 1280.0);
    let row = labels_row(&shell, cx);
    cx.simulate_click(row.center(), Modifiers::none());
    cx.run_until_parked();
    assert_eq!(state(&shell, cx), (true, true, RowPicker::Open));
    // Holding the keys, the page outlasts the pointer's leaving.
    cx.simulate_mouse_move(point(px(1200.0), px(800.0)), None, Modifiers::none());
    wait(cx, HOVER_CLOSE);
    assert_eq!(state(&shell, cx), (true, true, RowPicker::Open));
    cx.simulate_input("Re");
    assert_eq!(query(&shell, cx), "Re");
    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    assert_eq!(state(&shell, cx), (false, false, RowPicker::None));
}

#[gpui::test]
fn resting_on_labels_shows_the_picker_beside_and_leaves_the_keys_with_the_menu(
    cx: &mut TestAppContext,
) {
    let (shell, cx, _sent) = opened(cx, 1280.0);
    let row = labels_row(&shell, cx);
    cx.simulate_mouse_move(row.center(), None, Modifiers::none());
    wait(cx, HOVER_OPEN);
    assert_eq!(state(&shell, cx), (true, true, RowPicker::Open));
    cx.simulate_input("Re");
    assert_eq!(query(&shell, cx), "");
    // Leaving closes the page after the grace, the picker with it.
    cx.simulate_mouse_move(point(px(1200.0), px(800.0)), None, Modifiers::none());
    wait(cx, HOVER_CLOSE);
    assert_eq!(state(&shell, cx), (true, false, RowPicker::None));
}

#[gpui::test]
fn the_create_dialog_keeps_the_menu_waiting_undrawn_behind_it(cx: &mut TestAppContext) {
    let (shell, cx, _sent) = opened(cx, 1280.0);
    let row = labels_row(&shell, cx);
    cx.simulate_click(row.center(), Modifiers::none());
    cx.run_until_parked();
    shell.update_in(cx, Shell::labels_picker_create_now);
    cx.run_until_parked();
    assert_eq!(state(&shell, cx), (true, true, RowPicker::Dialog));
    // A press in the dialog is not a press outside the menu.
    cx.simulate_click(point(px(640.0), px(MIDDLE)), Modifiers::none());
    cx.run_until_parked();
    assert_eq!(state(&shell, cx), (true, true, RowPicker::Dialog));
}

#[gpui::test]
fn on_a_phone_the_labels_sheet_replaces_the_menu_and_leads_back(cx: &mut TestAppContext) {
    let (shell, cx, _sent) = opened(cx, 390.0);
    let row = labels_row(&shell, cx);
    assert!(row.bottom() > px(MIDDLE), "the menu is the bottom sheet");
    cx.simulate_click(row.center(), Modifiers::none());
    cx.run_until_parked();
    assert_eq!(state(&shell, cx), (true, true, RowPicker::Open));
    cx.simulate_input("Re");
    assert_eq!(query(&shell, cx), "Re");
    shell.update_in(cx, Shell::row_menu_labels_back);
    cx.run_until_parked();
    assert_eq!(state(&shell, cx), (true, false, RowPicker::None));
    // Again, then a press on its dimmed page: the menu goes with it.
    let row = labels_row(&shell, cx);
    cx.simulate_click(row.center(), Modifiers::none());
    cx.run_until_parked();
    assert_eq!(state(&shell, cx), (true, true, RowPicker::Open));
    cx.simulate_click(point(px(195.0), px(20.0)), Modifiers::none());
    cx.run_until_parked();
    assert_eq!(state(&shell, cx), (false, false, RowPicker::None));
}
