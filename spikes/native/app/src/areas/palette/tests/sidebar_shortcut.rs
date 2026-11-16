//! Query-only root action goes through the real dispatcher and palette frame.
use super::*;

#[gpui::test]
fn query_sidebar_action_uses_the_existing_keyboard_owner(cx: &mut TestAppContext) {
    let (view, cx) = harness(cx);
    let (before, label) = update(&view, cx, |shell, window, cx| {
        crate::areas::shortcuts::listen(shell, window, cx);
        let entries = root::build(shell, &shell.palette_all());
        let entry = entries
            .iter()
            .find(|entry| entry.id.as_str() == "toggle-left-sidebar")
            .unwrap();
        assert_eq!(
            entry.visibility,
            vessel_app_view::palette::registration::Visibility::Query
        );
        let before = (shell.sidebar_open(), shell.sidebar_shown());
        let label = vessel_app_view::shortcuts::action_label(
            vessel_app_view::shortcuts::Action::ToggleSidebar,
            shell.words,
        )
        .to_owned();
        (before, label)
    });
    ready(cx);
    cx.simulate_input(&label);
    ready(cx);
    cx.simulate_keystrokes("enter");
    ready(cx);
    update(&view, cx, |shell, _, _| {
        assert!(!shell.areas.palette.is_open());
        assert_ne!((shell.sidebar_open(), shell.sidebar_shown()), before);
    });
}

#[gpui::test]
fn root_help_tracks_actual_viewport_without_losing_pointer_palette(cx: &mut TestAppContext) {
    let (view, cx) = harness(cx);
    for (width, offered) in [(600.0, true), (599.0, false), (390.0, false), (600.0, true)] {
        cx.simulate_resize(gpui::size(px(width), px(844.0)));
        ready(cx);
        update(&view, cx, |shell, _, _| {
            assert!(
                shell.areas.palette.is_open(),
                "pointer center remains usable"
            );
            let shown = snapshot::project(shell)
                .sections
                .iter()
                .flat_map(|section| &section.rows)
                .any(|row| row.shortcut == Some(vessel_app_view::shortcuts::Action::Help));
            assert_eq!(shown, offered, "help at actual viewport width {width}");
        });
    }
}

#[gpui::test]
fn root_aliases_find_their_real_owner_actions(cx: &mut TestAppContext) {
    let (view, cx) = harness(cx);
    for (query, expected) in [
        ("configuration", "settings"),
        ("hotkeys", "keyboard-shortcuts"),
        ("panel", "toggle-left-sidebar"),
        ("workspaces", "toggle-left-sidebar"),
    ] {
        update(&view, cx, |shell, _, cx| {
            shell.palette_searched(query.to_owned(), cx)
        });
        ready(cx);
        update(&view, cx, |shell, _, _| {
            assert!(
                snapshot::project(shell)
                    .sections
                    .iter()
                    .flat_map(|section| &section.rows)
                    .any(|row| matches!(&row.id, snapshot::Identity::Registered(id)
                    if id.entry.as_str() == expected)),
                "missing {expected} for {query}"
            );
        });
    }
}
