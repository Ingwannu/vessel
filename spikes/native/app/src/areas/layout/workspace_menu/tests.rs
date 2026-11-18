//! The wide bar's workspace "…" (TRACKER 795): right after the title group
//! in Paseo's large square chrome, opening the shared Workspace actions.
use super::*;
use crate::areas::session::Open;
use crate::routes::Route;
use crate::shell::language_tests;
use gpui::{AppContext as _, Modifiers, Render, TestAppContext};
use vessel_app_kit::ui::{Entity, InteractiveElement as _, IntoElement};
use vessel_proto::SessionId;

struct Probe(Entity<Shell>, String);

impl Render for Probe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let title = self.1.clone();
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
                            title.clone(),
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
    titled("Fix the cart", window, cx)
}

fn titled(title: &str, window: &mut Window, cx: &mut Context<'_, Probe>) -> Probe {
    let shown = title.to_owned();
    Probe(
        cx.new(|cx| {
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
        }),
        shown,
    )
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

/// The test text system lays every character out one em wide (14 × 0.6).
const CHARACTER: f32 = 8.4;

#[gpui::test]
fn a_prompt_cut_title_has_room_and_shows_whole(cx: &mut TestAppContext) {
    language_tests::init(cx);
    // The node's prompt title: its first line cut at 80 characters, with no
    // ellipsis, as Paseo's provisional title (60, create-agent-title.ts).
    // The bar has room for it: shown whole, not clipped.
    let prompt = "x".repeat(80);
    let (_probe, cx) = cx.add_window_view(|window, cx| titled(&prompt, window, cx));
    cx.simulate_resize(gpui::size(px(1280.0), px(800.0)));
    cx.run_until_parked();
    let title = cx.debug_bounds("session-title-text").unwrap();
    let whole = CHARACTER.mul_add(80.0, -1.0);
    assert!(title.size.width >= px(whole), "{title:?}");
}

#[gpui::test]
fn a_title_longer_than_the_bar_shrinks_and_keeps_the_dots_inside(cx: &mut TestAppContext) {
    language_tests::init(cx);
    // Its box shrinks (its text ends in "…"), and the "…" stays in the bar.
    let long = "x".repeat(200);
    let (_probe, cx) = cx.add_window_view(|window, cx| titled(&long, window, cx));
    cx.simulate_resize(gpui::size(px(1280.0), px(800.0)));
    cx.run_until_parked();
    let bar = cx.debug_bounds("session-bar").unwrap();
    let title = cx.debug_bounds("session-title-text").unwrap();
    let dots = cx.debug_bounds("workspace-actions-button").unwrap();
    assert!(
        title.size.width < px(CHARACTER.mul_add(200.0, 0.0)),
        "{title:?}"
    );
    assert!(title.right() <= dots.left(), "{title:?} {dots:?}");
    assert!(dots.right() <= bar.right(), "{dots:?} {bar:?}");
}
