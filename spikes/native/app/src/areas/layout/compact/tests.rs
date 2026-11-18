//! At 390 one compact header replaces the tab head and the bar; its switcher
//! lists the tabs, and each row's menu has Paseo's items, then the
//! session's.
use super::*;
use crate::areas::layout::menu::{Action, Copied, Surface};
use crate::areas::session::Open;
use crate::routes::Route;
use crate::shell::language_tests;
use gpui::{AppContext as _, Modifiers, Render, TestAppContext};
use vessel_app_kit::MenuItem;
use vessel_app_kit::ui::{Entity, InteractiveElement as _, IntoElement, px};
use vessel_app_view::layout::{
    AgentRole, CloseScope, LayoutMode, OpenIntent, PaneId, Placement, Position, Target,
};
use vessel_app_view::toasts::ToastTone;
use vessel_proto::{HarnessKind, SessionId, SessionInfo, SessionMode, SessionSpec};

struct Probe(Entity<Shell>);

fn retained_body(
    _shell: &Shell,
    _tokens: &Tokens,
    _window: &mut Window,
    _cx: &mut Context<'_, Shell>,
) -> Div {
    div()
}

impl Render for Probe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        self.0.update(cx, |shell, cx| {
            let tokens = vessel_app_kit::tokens(cx);
            super::super::draw::frame(shell, retained_body, &tokens, window, cx)
                .debug_selector(|| "compact-frame".to_owned())
        })
    }
}

/// The session's own description, in a folder.
fn info(session: SessionId) -> SessionInfo {
    let mut spec = SessionSpec::new(
        HarnessKind::ClaudeCode,
        SessionMode::Structured {
            model: None,
            resume: None,
            options: Vec::new(),
        },
    );
    spec.cwd = Some("/work/shop".into());
    SessionInfo::new(session, spec, chrono::DateTime::UNIX_EPOCH)
}

/// An open session with its agent tab and a passive New tab after it; the
/// agent tab shown.
fn fixture(window: &mut Window, cx: &mut Context<'_, Probe>) -> Probe {
    Probe(cx.new(|cx| {
        let mut shell = Shell::without_io(window, cx);
        shell.route = Route::Session;
        let session = SessionId::random();
        shell.areas.session.open = Some(Open {
            machine: "laptop".to_owned(),
            session,
            title: "Fix the cart".to_owned(),
            agent: "Claude".to_owned(),
            structured: true,
            live: true,
            archived_at: None,
            binding: None,
        });
        with_layout(&shell, |layout| {
            let agent = layout.open(
                Target::Agent(session),
                OpenIntent::Reveal,
                Placement::Focused,
                AgentRole::Unknown,
                LayoutMode::Compact,
            )?;
            let _new = layout.open(
                Target::NewTab,
                OpenIntent::New,
                Placement::Focused,
                AgentRole::Unknown,
                LayoutMode::Compact,
            )?;
            layout.select(agent)
        })
        .unwrap();
        shell
    }))
}

fn compact(cx: &mut TestAppContext) -> (Entity<Probe>, &mut gpui::VisualTestContext) {
    language_tests::init(cx);
    let (probe, cx) = cx.add_window_view(fixture);
    cx.simulate_resize(gpui::size(px(390.0), px(844.0)));
    cx.run_until_parked();
    (probe, cx)
}

fn tab_ids(probe: &Entity<Probe>, cx: &mut gpui::VisualTestContext) -> Vec<TabId> {
    probe.read_with(cx, |probe, cx| {
        with_layout(probe.0.read(cx), |layout| Ok(tabs(layout))).unwrap()
    })
}

/// A per-tab selector: the test context takes only static ones.
fn selector(text: String) -> &'static str {
    text.leak()
}

/// A switcher row's menu: the shared tab menu (TRACKER 797) on its surface.
fn row_actions(shell: &Shell, tab: TabId) -> Vec<(Action, MenuItem)> {
    super::super::tab_menu::entries(shell, tab, Surface::Switcher)
}

fn labels(items: &[(Action, MenuItem)]) -> Vec<String> {
    items
        .iter()
        .map(|(_, item)| item.label.to_string())
        .collect()
}

#[gpui::test]
fn one_header_row_as_wide_as_the_window_with_the_switcher_in_the_touch_frame(
    cx: &mut TestAppContext,
) {
    let (_probe, cx) = compact(cx);
    let header = cx.debug_bounds("compact-workspace-header").unwrap();
    let frame = cx.debug_bounds("compact-frame").unwrap();
    // The kit's compact frame: 8 over a 56 row, at the top, as wide as the
    // pane (inside the pane's 1 rule on its right).
    assert_eq!(header.size, gpui::size(px(389.0), px(64.0)));
    assert_eq!(header.origin, frame.origin);
    assert!(cx.debug_bounds("layout-main-pane-controls").is_none());
    // "…" in the toolbar's touch frame, at the row's middle.
    let more = cx.debug_bounds("workspace-actions-button").unwrap();
    assert_eq!(more.size, gpui::size(px(32.0), px(32.0)));
    // 8 above the row, then half of the 56 row.
    let middle = f32::from(header.top()).mul_add(1.0, 36.0);
    assert_eq!(more.center().y, px(middle));
}

#[gpui::test]
fn the_switcher_lists_every_tab_and_a_press_shows_that_tab(cx: &mut TestAppContext) {
    let (probe, cx) = compact(cx);
    probe.update(cx, |probe, cx| {
        probe.0.update(cx, |shell, cx| {
            shell.areas.layout.compact.switcher_open = true;
            cx.notify();
        });
    });
    cx.run_until_parked();
    let ids = tab_ids(&probe, cx);
    assert_eq!(ids.len(), 2, "the agent tab and the New tab");
    let [agent, new] = [ids[0], ids[1]];
    assert!(
        cx.debug_bounds(selector(format!("compact-tab-row-{}", agent.get())))
            .is_some()
    );
    let row = cx
        .debug_bounds(selector(format!("compact-tab-row-{}", new.get())))
        .unwrap();
    assert!(row.size.height >= px(36.0), "Paseo's least row height");
    cx.simulate_click(row.center(), Modifiers::none());
    cx.run_until_parked();
    probe.read_with(cx, |probe, cx| {
        let shell = probe.0.read(cx);
        assert!(
            !shell.areas.layout.compact.switcher_open,
            "a press closes it"
        );
        let shown = with_layout(shell, |layout| Ok(current(layout))).unwrap();
        assert_eq!(shown, Some(new));
    });
}

#[gpui::test]
fn new_agent_in_the_footer_opens_the_new_agent_page(cx: &mut TestAppContext) {
    let (probe, cx) = compact(cx);
    probe.update(cx, |probe, cx| {
        probe.0.update(cx, |shell, cx| {
            shell.areas.layout.compact.switcher_open = true;
            cx.notify();
        });
    });
    cx.run_until_parked();
    let button = cx.debug_bounds("compact-tab-switcher-footer").unwrap();
    // New agent is the footer's first half.
    let left = gpui::point(
        px(f32::from(button.left()).mul_add(1.0, 8.0)),
        button.center().y,
    );
    cx.simulate_click(left, Modifiers::none());
    cx.run_until_parked();
    probe.read_with(cx, |probe, cx| {
        let shell = probe.0.read(cx);
        assert_eq!(shell.route, Route::Draft);
        assert!(!shell.areas.layout.compact.switcher_open);
    });
}

#[gpui::test]
fn the_agent_rows_menu_has_paseos_items_then_the_sessions(cx: &mut TestAppContext) {
    let (probe, cx) = compact(cx);
    let ids = tab_ids(&probe, cx);
    probe.read_with(cx, |probe, cx| {
        let shell = probe.0.read(cx);
        let words = shell.words.layout();
        // Before the node described the session: no Rename, no Close, and
        // Kill while it may live.
        let items = row_actions(shell, ids[0]);
        assert_eq!(
            labels(&items),
            [
                words.copy_resume_command(),
                words.copy_agent_id(),
                words.close_above(),
                words.close_below(),
                words.close_others(),
                shell.words.sessions().copy_link(),
                shell.t.lang.conversation().kill(),
            ]
        );
        // Paseo's 16 glyph on every row; Copy agent id's hint is the id's
        // first seven characters.
        assert!(items.iter().all(|(_, item)| item.leading.is_some()));
        let hint = items.get(1).and_then(|(_, item)| item.value.clone());
        assert_eq!(hint.map(|hint| hint.len()), Some(7));
        let groups: Vec<bool> = items.iter().map(|(_, item)| item.starts_group).collect();
        assert_eq!(groups, [false, false, true, false, false, true, false]);
        // The tab's own copy, the same press as on the wide strip.
        assert!(matches!(
            items.get(1),
            Some((Action::Copy(Copied::AgentId), _))
        ));
        // The New tab: Paseo's closes only, Close itself last.
        let passive = row_actions(shell, ids[1]);
        assert_eq!(
            labels(&passive),
            [
                words.close_above(),
                words.close_below(),
                words.close_others(),
                words.close(),
            ]
        );
        assert!(!passive.iter().any(|(_, item)| item.starts_group));
    });
    probe.update(cx, |probe, cx| {
        probe.0.update(cx, |shell, _cx| {
            let session = shell.areas.session.open.as_ref().unwrap().session;
            shell.areas.conversation.info = Some(Box::new(info(session)));
        });
    });
    probe.read_with(cx, |probe, cx| {
        let shell = probe.0.read(cx);
        let words = shell.words.layout();
        let labels = labels(&row_actions(shell, ids[0]));
        let close = labels.iter().position(|label| label == words.close());
        let copy_link = labels
            .iter()
            .position(|label| label == shell.words.sessions().copy_link());
        assert!(close.is_some(), "Close once the node described it");
        assert!(close < copy_link, "Paseo's Close before Vessel's own");
    });
}

#[gpui::test]
fn a_rows_dots_open_its_menu_over_the_sheet(cx: &mut TestAppContext) {
    let (probe, cx) = compact(cx);
    probe.update(cx, |probe, cx| {
        probe.0.update(cx, |shell, cx| {
            shell.areas.layout.compact.switcher_open = true;
            cx.notify();
        });
    });
    cx.run_until_parked();
    // Paseo's combobox sheet: its title, and no close button (TRACKER 798).
    assert!(cx.debug_bounds("dialog-title").is_some());
    assert!(cx.debug_bounds("dialog-close").is_none());
    let ids = tab_ids(&probe, cx);
    let dots = cx
        .debug_bounds(selector(format!("compact-tab-menu-{}", ids[0].get())))
        .unwrap();
    assert_eq!(dots.size, gpui::size(px(28.0), px(28.0)));
    cx.simulate_click(dots.center(), Modifiers::none());
    cx.run_until_parked();
    probe.read_with(cx, |probe, cx| {
        let layout = &probe.0.read(cx).areas.layout;
        assert!(layout.menu.is_open());
        assert_eq!(layout.menu_surface, Surface::Switcher);
        assert_eq!(layout.menu_tab.as_ref().map(|(_, tab)| *tab), Some(ids[0]));
        assert!(layout.compact.switcher_open, "the sheet stays under it");
    });
    // A sheet titled with the tab's label, as wide as the window.
    assert_sheet(cx);
}

/// The open menu is a phone's sheet: its first row spans the window, less
/// the sheet's insets (a popover is 220 wide).
fn assert_sheet(cx: &mut gpui::VisualTestContext) {
    // The frame after the sheet's first measures it.
    cx.update(|window, cx| {
        let _ran = window.simulate_next_frame(cx);
    });
    cx.run_until_parked();
    let row = cx.debug_bounds("menu-row-0").unwrap();
    assert!(row.left() < px(16.0), "{row:?}");
    assert!(row.right() > px(374.0), "{row:?}");
}

#[gpui::test]
fn workspace_actions_copy_the_sessions_folder_once_it_is_known(cx: &mut TestAppContext) {
    let (probe, cx) = compact(cx);
    probe.read_with(cx, |probe, cx| {
        assert_eq!(workspace_menu::workspace_path(probe.0.read(cx)), None);
    });
    probe.update(cx, |probe, cx| {
        probe.0.update(cx, |shell, _cx| {
            let session = shell.areas.session.open.as_ref().unwrap().session;
            shell.areas.conversation.info = Some(Box::new(info(session)));
        });
    });
    let more = cx.debug_bounds("workspace-actions-button").unwrap();
    cx.simulate_click(more.center(), Modifiers::none());
    cx.run_until_parked();
    // Paseo's sheet titled "Workspace actions" on a phone (TRACKER 798).
    assert_sheet(cx);
    probe.read_with(cx, |probe, cx| {
        let shell = probe.0.read(cx);
        assert!(shell.areas.layout.workspace_menu.menu.is_open());
        assert_eq!(
            workspace_menu::workspace_path(shell).as_deref(),
            Some("/work/shop")
        );
    });
    // Its copy says so as Paseo's (TRACKER 804): "Copied Workspace path".
    probe.update(cx, |probe, cx| {
        probe.0.update(cx, Shell::copy_workspace_path);
    });
    probe.read_with(cx, |probe, cx| {
        let said = probe.0.read(cx).areas.toasts.shown().cloned();
        let said = said.map(|toast| (toast.text, toast.tone));
        // Fluent isolates the label (U+2068 .. U+2069).
        let expected = "Copied \u{2068}Workspace path\u{2069}".to_owned();
        assert_eq!(said, Some((expected, ToastTone::Copied)));
    });
    let copied = cx.read_from_clipboard().and_then(|item| item.text());
    assert_eq!(copied.as_deref(), Some("/work/shop"));
}

#[gpui::test]
fn import_session_opens_the_sheet_for_this_host_and_folder(cx: &mut TestAppContext) {
    let (probe, cx) = compact(cx);
    probe.update_in(cx, |probe, window, cx| {
        probe.0.update(cx, |shell, cx| {
            let session = shell.areas.session.open.as_ref().unwrap().session;
            shell.areas.conversation.info = Some(Box::new(info(session)));
            shell.open_import_for_workspace(window, cx);
        });
    });
    cx.run_until_parked();
    probe.read_with(cx, |probe, cx| {
        assert!(probe.0.read(cx).areas.import.is_open());
    });
}

#[gpui::test]
fn with_two_panes_the_switcher_is_the_focused_panes_and_closes_only_there(cx: &mut TestAppContext) {
    let (probe, cx) = compact(cx);
    let before = tab_ids(&probe, cx);
    // A second pane (made wide, kept at compact) with two New tabs, focused.
    let (side, first, second) = probe.update(cx, |probe, cx| {
        probe.0.update(cx, |shell, _cx| {
            with_layout(shell, |layout| {
                let side = layout.split(PaneId::MAIN, Position::Right, LayoutMode::SplitPanes)?;
                let mut open = || {
                    layout.open(
                        Target::NewTab,
                        OpenIntent::New,
                        Placement::Pane(side),
                        AgentRole::Unknown,
                        LayoutMode::SplitPanes,
                    )
                };
                let first = open()?;
                let second = open()?;
                layout.select(second)?;
                Ok((side, first, second))
            })
            .unwrap()
        })
    });
    cx.run_until_parked();
    // Paseo's tabs = focusedPaneTabState.tabs: the other pane's are not
    // listed, and the count badge counts the same list.
    assert_eq!(tab_ids(&probe, cx), [first, second]);
    // Close tabs above, chosen on the second row, closes the first only.
    probe.update_in(cx, |probe, window, cx| {
        probe.0.update(cx, |shell, cx| {
            let items = row_actions(shell, second);
            let above = items
                .iter()
                .find(|(action, _)| matches!(action, Action::Close(CloseScope::Before)))
                .map(|(action, item)| (*action, item.disabled));
            assert!(matches!(above, Some((_, false))), "enabled: one tab above");
            if let Some((action, _)) = above {
                super::super::menu::chose(shell, second, action, window, cx);
            }
        });
    });
    cx.run_until_parked();
    assert_eq!(tab_ids(&probe, cx), [second]);
    probe.read_with(cx, |probe, cx| {
        let main = with_layout(probe.0.read(cx), |layout| {
            Ok(layout
                .root()
                .panes()
                .into_iter()
                .find(|pane| pane.id() == PaneId::MAIN)
                .map(|pane| pane.tabs().to_vec()))
        })
        .unwrap();
        assert_eq!(main, Some(before), "the other pane is left alone");
        let side_open = with_layout(probe.0.read(cx), |layout| {
            Ok(layout
                .root()
                .panes()
                .into_iter()
                .any(|pane| pane.id() == side))
        })
        .unwrap();
        assert!(side_open);
    });
}

/// At 700 the window is a desktop's: the bar and the strip are drawn, and
/// Paseo's menus there are popovers (its one compact width is 600; the
/// app reviewer on TRACKER 798, the coordinator's ruling).
fn wide(cx: &mut TestAppContext) -> (Entity<Probe>, &mut gpui::VisualTestContext) {
    language_tests::init(cx);
    let (probe, cx) = cx.add_window_view(fixture);
    cx.simulate_resize(gpui::size(px(700.0), px(844.0)));
    cx.run_until_parked();
    cx.update(|window, _| {
        assert_eq!(super::super::mode(window), LayoutMode::SplitPanes);
        assert!(!vessel_app_kit::breakpoint::phone(window));
    });
    (probe, cx)
}

/// The open menu is a popover: its first row is the menu's 220 less its
/// insets, not the window's width.
fn assert_popover(cx: &mut gpui::VisualTestContext) {
    cx.update(|window, cx| {
        let _ran = window.simulate_next_frame(cx);
    });
    cx.run_until_parked();
    let row = cx.debug_bounds("menu-row-0").unwrap();
    assert!(row.size.width <= px(220.0), "{row:?}");
}

#[gpui::test]
fn at_700_workspace_actions_is_a_popover(cx: &mut TestAppContext) {
    let (probe, cx) = wide(cx);
    probe.update_in(cx, |probe, window, cx| {
        probe.0.update(cx, |shell, cx| {
            let at = vessel_app_kit::ui::point(px(600.0), px(40.0));
            shell.areas.layout.workspace_menu.menu.open(at, window, cx);
            cx.notify();
        });
    });
    cx.run_until_parked();
    assert_popover(cx);
}

#[gpui::test]
fn at_700_a_strip_tabs_menu_is_a_popover(cx: &mut TestAppContext) {
    let (probe, cx) = wide(cx);
    let ids = tab_ids(&probe, cx);
    probe.update_in(cx, |probe, window, cx| {
        probe.0.update(cx, |shell, cx| {
            // As a right press on the strip tab opens it (`draw::tab_view`).
            let workspace = key(shell).unwrap();
            let area = &mut shell.areas.layout;
            area.menu_surface = Surface::Strip;
            area.menu_tab = Some((workspace, ids[0]));
            let at = vessel_app_kit::ui::point(px(40.0), px(40.0));
            area.menu.open(at, window, cx);
            cx.notify();
        });
    });
    cx.run_until_parked();
    probe.read_with(cx, |probe, cx| {
        assert!(probe.0.read(cx).areas.layout.menu.is_open());
    });
    assert_popover(cx);
}
