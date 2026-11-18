//! The row menu's Labels page (TRACKER 791): on a wide window the
//! workspace picker beside the menu, the keys moved in only by a press, its
//! dismissal closing the menu; on a phone the picker's sheet replacing the
//! menu's and leading back to it. Its Create label page (TRACKER 792):
//! beside the list on a press only, as a sheet on a phone.
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
    let away = point(px(1200.0), px(800.0));
    cx.simulate_mouse_move(away, None, Modifiers::none());
    wait(cx, HOVER_CLOSE);
    assert_eq!(state(&shell, cx), (true, false, RowPicker::None));
    // One move from the row into the flyout keeps it; a press there is not
    // outside the menu and holds the page past the pointer's leaving.
    cx.simulate_mouse_move(row.center(), None, Modifiers::none());
    wait(cx, HOVER_OPEN);
    let inside = point(
        px(f32::from(row.right()).mul_add(1.0, 20.0)),
        row.center().y,
    );
    cx.simulate_mouse_move(inside, None, Modifiers::none());
    wait(cx, HOVER_CLOSE);
    assert_eq!(state(&shell, cx), (true, true, RowPicker::Open));
    cx.simulate_click(inside, Modifiers::none());
    cx.run_until_parked();
    cx.simulate_mouse_move(away, None, Modifiers::none());
    wait(cx, HOVER_CLOSE);
    assert_eq!(state(&shell, cx), (true, true, RowPicker::Open));
}

/// The Labels list's Create label row, as drawn.
fn create_row(shell: &Entity<Shell>, cx: &mut VisualTestContext) -> Bounds<Pixels> {
    shell
        .read_with(cx, |shell, _| {
            shell.areas.sidebar.rows.pages.trigger(CREATE_PAGE)
        })
        .unwrap()
}

/// The Create label page open beside the list, and the picker's draft.
fn creating(shell: &Entity<Shell>, cx: &mut VisualTestContext) -> (bool, RowCreate) {
    shell.read_with(cx, |shell, _| {
        (
            shell.areas.sidebar.rows.pages.is_open(CREATE_PAGE, 1),
            shell.areas.labels.row_create(),
        )
    })
}

fn create_name(shell: &Entity<Shell>, cx: &mut VisualTestContext) -> Option<String> {
    cx.update(|_, cx| shell.read(cx).labels_create_name(cx))
}

/// The Labels list beside the menu, opened by a press, then its Create
/// label row pressed (TRACKER 792).
fn create_page(cx: &mut TestAppContext) -> (Entity<Shell>, &mut VisualTestContext, Sent) {
    let (shell, cx, sent) = opened(cx, 1280.0);
    let row = labels_row(&shell, cx);
    cx.simulate_click(row.center(), Modifiers::none());
    cx.run_until_parked();
    let create = create_row(&shell, cx);
    // A page you type into does not open under a resting pointer.
    cx.simulate_mouse_move(create.center(), None, Modifiers::none());
    wait(cx, HOVER_OPEN);
    assert_eq!(creating(&shell, cx), (false, RowCreate::None));
    cx.simulate_click(create.center(), Modifiers::none());
    cx.run_until_parked();
    (shell, cx, sent)
}

#[gpui::test]
fn create_label_opens_its_page_beside_the_list_on_a_press_with_the_name_holding_the_keys(
    cx: &mut TestAppContext,
) {
    let (shell, cx, _sent) = create_page(cx);
    assert_eq!(creating(&shell, cx), (true, RowCreate::Draft));
    // The menu and the list stay; the page is beside the list.
    assert_eq!(state(&shell, cx), (true, true, RowPicker::Open));
    let page = cx.debug_bounds("workspace-label-create").unwrap();
    assert!(page.left() >= create_row(&shell, cx).center().x);
    cx.simulate_input("Ops");
    assert_eq!(create_name(&shell, cx).as_deref(), Some("Ops"));
    // A press in the page is not outside the menu, and the pointer's
    // leaving closes nothing.
    let swatches = cx.debug_bounds("label-swatches").unwrap();
    cx.simulate_click(swatches.center(), Modifiers::none());
    cx.simulate_mouse_move(point(px(1200.0), px(800.0)), None, Modifiers::none());
    wait(cx, HOVER_CLOSE);
    assert_eq!(creating(&shell, cx), (true, RowCreate::Draft));
    assert_eq!(state(&shell, cx), (true, true, RowPicker::Open));
}

#[gpui::test]
fn escape_in_the_page_goes_back_to_the_list(cx: &mut TestAppContext) {
    let (shell, cx, _sent) = create_page(cx);
    cx.simulate_input("Ops");
    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    assert_eq!(creating(&shell, cx), (false, RowCreate::None));
    assert_eq!(state(&shell, cx), (true, true, RowPicker::Open));
}

#[gpui::test]
fn enter_creates_the_label_and_the_list_comes_back(cx: &mut TestAppContext) {
    let (shell, cx, mut sent) = create_page(cx);
    cx.simulate_input("Ops");
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    assert_eq!(creating(&shell, cx), (true, RowCreate::Saving));
    let request = crate::areas::labels::fixtures::sent(&mut sent);
    let answer = crate::areas::labels::fixtures::assigned(request);
    shell.update(cx, |shell, cx| shell.labels_answer(answer, cx));
    cx.run_until_parked();
    // Paseo's `goBack`: the list again, the menu still open.
    assert_eq!(creating(&shell, cx), (false, RowCreate::None));
    assert_eq!(state(&shell, cx), (true, true, RowPicker::Open));
}

#[gpui::test]
fn on_a_phone_create_label_is_a_sheet_leading_back_to_the_list(cx: &mut TestAppContext) {
    let (shell, cx, _sent) = opened(cx, 390.0);
    let row = labels_row(&shell, cx);
    cx.simulate_click(row.center(), Modifiers::none());
    cx.run_until_parked();
    shell.update_in(cx, Shell::labels_picker_create_now);
    cx.run_until_parked();
    assert_eq!(state(&shell, cx), (true, true, RowPicker::Dialog));
    assert!(cx.debug_bounds("workspace-label-create").is_some());
    let back = cx.debug_bounds("dialog-header-back").unwrap();
    cx.simulate_click(back.center(), Modifiers::none());
    cx.run_until_parked();
    assert_eq!(state(&shell, cx), (true, true, RowPicker::Open));
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

#[gpui::test]
fn copy_path_copies_the_folder_and_says_so_in_paseos_words(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (link, _sent) = Link::detached(COMMANDS).unwrap();
    let (shell, cx) = cx.add_window_view(move |window, cx| Shell::with_test_link(window, cx, link));
    cx.run_until_parked();
    let mut row = card(SessionId::random());
    row.detail.cwd = Some("/work/shop".to_owned());
    shell.update_in(cx, |shell, window, cx| {
        shell.row_menu_open(&row, point(px(40.0), px(120.0)), window, cx);
        // A row whose folder is known leads with Copy path.
        shell.row_menu_chose(0, window, cx);
        let said = shell.areas.toasts.shown().cloned();
        let said = said.map(|toast| (toast.text, toast.tone));
        // Paseo's sidebar copy (sidebar.workspace.toasts.pathCopied).
        assert_eq!(said, Some(("Path copied".to_owned(), ToastTone::Copied)));
    });
    let copied = cx.read_from_clipboard().and_then(|item| item.text());
    assert_eq!(copied.as_deref(), Some("/work/shop"));
}
