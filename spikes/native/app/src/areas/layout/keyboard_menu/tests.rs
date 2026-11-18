//! Shift+F10 and the Menu key open the focused tab's menu (TRACKER 800),
//! the keyboard's way to the session's actions now the bar has no "…".
use super::*;
use crate::areas::conversation::MenuAction;
use crate::areas::session::Open;
use crate::shell::language_tests;
use gpui::{Entity, TestAppContext, VisualTestContext, px, size};
use vessel_app_view::layout::{AgentRole, OpenIntent, Placement, TabId, Target};
use vessel_app_view::{Machine, Reach};
use vessel_proto::SessionId;

/// A session page with the agent's tab focused and a New tab beside it,
/// the keyboard listener on, at `width`.
fn page(cx: &mut TestAppContext, width: f32) -> (Entity<Shell>, &mut VisualTestContext, TabId) {
    language_tests::init(cx);
    cx.update(|cx| cx.set_reduce_motion(true));
    let mut agent = None;
    let (shell, cx) = cx.add_window_view(|window, cx| {
        let mut shell = Shell::without_io(window, cx);
        shell.loaded = true;
        shell.fleet.machines = vec![Machine {
            name: "laptop".to_owned(),
            node: None,
            connection_epoch: None,
            reach: Reach::Online,
            sessions: Vec::new(),
            agents: Vec::new(),
            protocol: None,
            version: None,
        }];
        let session = SessionId::random();
        shell.route = Route::Session;
        shell.areas.session.open = Some(Open {
            machine: "laptop".to_owned(),
            session,
            title: "Fix the cart".to_owned(),
            agent: "Claude Code".to_owned(),
            structured: true,
            live: true,
            archived_at: None,
            binding: None,
        });
        agent = with_layout(&shell, |layout| {
            let tab = layout.open(
                Target::Agent(session),
                OpenIntent::Reveal,
                Placement::Focused,
                AgentRole::Unknown,
                LayoutMode::SplitPanes,
            )?;
            let _new = layout.open(
                Target::NewTab,
                OpenIntent::New,
                Placement::Focused,
                AgentRole::Unknown,
                LayoutMode::SplitPanes,
            )?;
            layout.select(tab)?;
            Ok(tab)
        })
        .ok();
        crate::areas::shortcuts::listen(&mut shell, window, cx);
        shell
    });
    cx.update(|window, _| window.activate_window());
    cx.simulate_resize(size(px(width), px(800.0)));
    cx.run_until_parked();
    (shell, cx, agent.unwrap())
}

fn menu_on(shell: &Entity<Shell>, cx: &mut VisualTestContext) -> Option<TabId> {
    shell.read_with(cx, |shell, _| {
        let area = &shell.areas.layout;
        (area.menu.is_open() && area.menu_surface == Surface::Strip)
            .then(|| area.menu_tab.as_ref().map(|(_, tab)| *tab))
            .flatten()
    })
}

#[gpui::test]
fn shift_f10_and_the_menu_key_open_the_focused_tabs_menu(cx: &mut TestAppContext) {
    let (shell, cx, agent) = page(cx, 1280.0);
    assert_eq!(menu_on(&shell, cx), None);
    shell.read_with(cx, |shell, _| {
        assert_eq!(shell.route, Route::Session);
        assert!(shell.global_shortcuts_enabled());
        assert!(
            shell.areas.layout.tab_place(agent).get().is_some(),
            "the strip drew the tab"
        );
    });
    // The opening itself, then the keys that ask for it.
    let opened = shell.update_in(cx, |shell, window, cx| {
        shell.layout_tab_menu_from_keyboard(window, cx)
    });
    assert!(opened, "the focused tab's menu opens under it");
    cx.run_until_parked();
    assert_eq!(menu_on(&shell, cx), Some(agent), "and stays open a frame");
    shell.update(cx, |shell, _| shell.areas.layout.menu.close());
    cx.run_until_parked();
    cx.simulate_keystrokes("shift-f10");
    let seen = shell.read_with(cx, |shell, _| {
        let area = &shell.areas.layout;
        (
            area.menu.is_open(),
            area.menu_surface,
            area.menu_tab.as_ref().map(|(_, tab)| *tab),
        )
    });
    assert_eq!(seen, (true, Surface::Strip, Some(agent)), "after the key");
    cx.run_until_parked();
    assert_eq!(menu_on(&shell, cx), Some(agent));
    // It holds the session's own actions, as the bar's "…" did.
    shell.read_with(cx, |shell, _| {
        let rows = super::super::tab_menu::entries(shell, agent, Surface::Strip);
        assert!(rows.iter().any(|(action, _)| {
            *action == super::super::menu::Action::Session(MenuAction::Kill)
        }));
    });
    shell.update(cx, |shell, _| shell.areas.layout.menu.close());
    cx.simulate_keystrokes("menu");
    cx.run_until_parked();
    assert_eq!(menu_on(&shell, cx), Some(agent));
}

#[gpui::test]
fn other_f10_presses_and_the_compact_width_leave_the_menu_shut(cx: &mut TestAppContext) {
    let (shell, cx, _agent) = page(cx, 1280.0);
    cx.simulate_keystrokes("f10");
    cx.simulate_keystrokes("ctrl-shift-f10");
    cx.run_until_parked();
    assert_eq!(menu_on(&shell, cx), None);
    // Below the compact width the strip is not drawn: the switcher's row
    // "…" is the way there.
    cx.simulate_resize(size(px(390.0), px(844.0)));
    cx.run_until_parked();
    cx.simulate_keystrokes("shift-f10");
    cx.run_until_parked();
    assert_eq!(menu_on(&shell, cx), None);
}

#[test]
fn the_context_menu_keys_are_shift_f10_and_the_menu_key_alone() {
    assert!(is_context_menu_key("f10", true, false));
    assert!(is_context_menu_key("menu", false, false));
    assert!(!is_context_menu_key("f10", false, false));
    assert!(!is_context_menu_key("f10", true, true));
    assert!(!is_context_menu_key("menu", true, false));
    assert!(!is_context_menu_key("menu", false, true));
}
