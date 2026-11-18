//! The wide bar's workspace "…" (TRACKER 795): right after the title group
//! in Paseo's large square chrome, opening the shared Workspace actions.
use super::*;
use crate::areas::session::Open;
use crate::routes::Route;
use crate::shell::language_tests;
use gpui::{AppContext as _, Modifiers, Render, TestAppContext};
use vessel_app_kit::ui::{Entity, InteractiveElement as _, IntoElement};
use vessel_proto::SessionId;

struct Probe(Entity<Shell>);

impl Render for Probe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        self.0.update(cx, |shell, cx| {
            let tokens = vessel_app_kit::tokens(cx);
            div()
                .size_full()
                .child(
                    div()
                        .w_full()
                        .debug_selector(|| "session-bar".to_owned())
                        .child(crate::areas::conversation::bar(
                            shell,
                            "Fix the cart".to_owned(),
                            "laptop · Claude Code".to_owned(),
                            &tokens,
                            window,
                            cx,
                        )),
                )
                .children(overlay(shell, &tokens, window, cx))
        })
    }
}

fn fixture(window: &mut Window, cx: &mut Context<'_, Probe>) -> Probe {
    Probe(cx.new(|cx| {
        let mut shell = Shell::without_io(window, cx);
        shell.route = Route::Session;
        shell.areas.session.open = Some(Open {
            machine: "laptop".to_owned(),
            session: SessionId::random(),
            title: "Fix the cart".to_owned(),
            agent: "Claude Code".to_owned(),
            structured: true,
            live: true,
            archived_at: None,
            binding: None,
        });
        shell
    }))
}

#[gpui::test]
fn the_wide_bar_has_the_workspace_dots_after_its_title_in_the_large_chrome(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    let (probe, cx) = cx.add_window_view(fixture);
    cx.simulate_resize(gpui::size(px(1280.0), px(800.0)));
    cx.run_until_parked();
    let bar = cx.debug_bounds("session-bar").unwrap();
    let dots = cx.debug_bounds("workspace-actions-button").unwrap();
    // Paseo's large chrome on a wide window: 26 square.
    assert_eq!(dots.size, gpui::size(px(26.0), px(26.0)));
    // Right after the title group, not at the bar's end.
    assert!(dots.right() < bar.center().x, "{dots:?} in {bar:?}");
    cx.simulate_click(dots.center(), Modifiers::none());
    cx.run_until_parked();
    probe.read_with(cx, |probe, cx| {
        let shell = probe.0.read(cx);
        assert!(shell.areas.layout.workspace_menu.menu.is_open());
        // The shared builder: the compact "…" lists the same rows.
        let words = shell.words.layout();
        let labels: Vec<String> = items(shell)
            .iter()
            .map(|item| item.label.to_string())
            .collect();
        assert_eq!(
            labels,
            [words.copy_workspace_path(), shell.words.history().title()]
        );
        assert!(items(shell).iter().all(|item| item.leading.is_some()));
    });
}
