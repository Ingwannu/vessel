//! A row's "…" as assistive technology meets it (app review R1, as TRACKER
//! 662's project kebab): a named Button that takes a Click while the page
//! is idle and none while a change is out, and a click that opens the menu
//! once rather than toggling it shut.

use gpui::{AppContext as _, Entity, IntoElement, Render, TestAppContext, point};
use vessel_app_kit::ui::px;
use vessel_app_kit::{Scheme, Tokens};
use vessel_app_link::Link;
use vessel_app_view::devices::MachineKey;
use vessel_app_view::plugins::{Action, Page, Reply, Request};
use vessel_proto::plugins::{PluginId, PluginListItem, PluginSourceKind, PluginStatus};

use super::super::Open;
use super::*;

struct Probe {
    shell: Entity<Shell>,
}

impl Render for Probe {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        self.shell.update(cx, |shell, cx| {
            let drawn =
                shell.areas.settings.plugins.page().map(|page| {
                    trigger(shell, page, &night(), &Tokens::of(Scheme::Dark), false, cx)
                });
            div().size_full().children(drawn)
        })
    }
}

fn night() -> PluginId {
    "night".parse().unwrap()
}

fn listed() -> PluginListItem {
    PluginListItem {
        id: night(),
        name: None,
        description: None,
        icon: None,
        path: None,
        source: PluginSourceKind::Directory,
        enabled: true,
        status: PluginStatus::Running,
        error: None,
    }
}

/// The page for "laptop" with "night" listed.
fn opened(shell: &mut Shell) {
    let machine = MachineKey::from_book("laptop");
    let state = &mut shell.areas.settings.plugins;
    let (mut page, calls) = Page::open(machine.clone(), &mut state.tickets);
    let list = calls
        .iter()
        .find(|call| call.request == Request::List)
        .unwrap()
        .ticket;
    let _answered = page.answer(
        &mut state.tickets,
        &machine,
        list,
        Ok(Reply::List(vec![listed()])),
    );
    state.open = Some(Open { page, epoch: None });
}

/// The trigger's accessibility node, as the window's tree says it after a
/// fresh frame.
fn trigger_node(cx: &mut gpui::VisualTestContext) -> String {
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
    let json = cx.update(|window, _| window.debug_a11y_tree_json().unwrap());
    let nodes: Vec<String> = json
        .split("\"aria\":")
        .skip(1)
        .filter_map(|rest| rest.split_once('}').map(|(aria, _)| aria.to_owned()))
        // Fluent isolates the id in the sentence (U+2068 … U+2069).
        .filter(|aria| aria.contains("\"label\": \"Actions for \u{2068}night\u{2069}\""))
        .collect();
    assert_eq!(nodes.len(), 1, "one trigger: {json}");
    nodes.into_iter().next().unwrap()
}

#[gpui::test]
fn a_rows_trigger_is_a_button_that_takes_a_click_only_while_idle(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (link, _commands) = Link::detached(8).unwrap();
    let (probe, cx) = cx.add_window_view(move |window, cx| Probe {
        shell: cx.new(|cx| Shell::with_test_link(window, cx, link)),
    });
    let shell = probe.update(cx, |probe, _cx| probe.shell.clone());
    let handle = cx.update(|window, _| window.window_handle());
    assert!(cx.simulate_accessibility_activation(handle));
    cx.run_until_parked();
    shell.update(cx, |shell, cx| {
        opened(shell);
        cx.notify();
    });
    cx.run_until_parked();
    let idle = trigger_node(cx);
    assert!(idle.contains(r#""Button""#), "{idle}");
    assert!(idle.contains(r#""Click""#), "{idle}");
    assert!(idle.contains(r#""expanded": false"#), "{idle}");
    // A change out: the row waits and offers nothing to press.
    shell.update(cx, |shell, cx| {
        let state = &mut shell.areas.settings.plugins;
        let open = state.open.as_mut().unwrap();
        assert!(
            open.page
                .act(&mut state.tickets, Action::Reload, &night())
                .is_some()
        );
        cx.notify();
    });
    cx.run_until_parked();
    let busy = trigger_node(cx);
    assert!(!busy.contains(r#""Click""#), "{busy}");
}

#[gpui::test]
fn a_click_alone_opens_the_rows_menu_once(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (link, _commands) = Link::detached(8).unwrap();
    let (probe, cx) = cx.add_window_view(move |window, cx| Probe {
        shell: cx.new(|cx| Shell::with_test_link(window, cx, link)),
    });
    let shell = probe.update(cx, |probe, _cx| probe.shell.clone());
    let handle = cx.update(|window, _| window.window_handle());
    assert!(cx.simulate_accessibility_activation(handle));
    shell.update(cx, |shell, cx| {
        opened(shell);
        cx.notify();
    });
    cx.run_until_parked();
    cx.update(|window, cx| {
        shell.update(cx, |shell, cx| {
            let at = point(px(200.0), px(120.0));
            // No press went down first: assistive technology's Click.
            shell.plugins_menu_activate(night(), at, window, cx);
            let state = &shell.areas.settings.plugins;
            assert!(state.menu.is_open());
            assert_eq!(state.menu_for, Some(night()));
            // The click that ends a pointer's press leaves it open.
            shell.plugins_menu_activate(night(), at, window, cx);
            let state = &shell.areas.settings.plugins;
            assert!(state.menu.is_open());
            assert_eq!(state.menu_for, Some(night()));
        });
    });
    let open = trigger_node(cx);
    assert!(open.contains(r#""expanded": true"#), "{open}");
}
