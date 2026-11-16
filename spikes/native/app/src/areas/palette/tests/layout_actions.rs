//! Layout rows retain the owner fence and canonical hidden search vocabulary.
use super::*;

#[gpui::test]
fn korean_layout_rows_match_reference_hidden_aliases_across_viewports(cx: &mut TestAppContext) {
    use vessel_app_view::layout::{HostAlias, LayoutKey, OpenIntent, Placement};
    use vessel_app_view::localization::{Locale, Translator};

    let (view, cx) = harness(cx);
    cx.simulate_resize(gpui::size(px(1280.0), px(900.0)));
    update(&view, cx, |shell, window, cx| {
        let korean = Translator::get(Locale::from_tag("ko").unwrap()).unwrap();
        shell.apply_language(korean, window, cx);
        let session = SessionId::random();
        shell.route = Route::Session;
        shell.areas.session.open = Some(Open {
            machine: "one".to_owned(),
            session,
            title: "A".to_owned(),
            agent: "fake".to_owned(),
            structured: false,
            live: true,
        });
        let key = LayoutKey::Session {
            machine: HostAlias::parse("one").unwrap(),
            session,
        };
        let _tab = shell
            .layout_open_target(
                &key,
                vessel_app_view::layout::Target::NewTab,
                OpenIntent::New,
                Placement::Focused,
                window,
                cx,
            )
            .unwrap();
        shell.open_palette(window, cx);
    });
    ready(cx);
    for (width, query, expected) in [
        (1280.0, "vertical", "pane:split-right"),
        (1280.0, "horizontal", "pane:split-down"),
        (1280.0, "previous switch", "tab:previous"),
        (1280.0, "next navigate", "tab:next"),
        (599.0, "zen", "workspace:toggle-focus-mode"),
        (599.0, "distraction", "workspace:toggle-focus-mode"),
    ] {
        cx.simulate_resize(gpui::size(px(width), px(900.0)));
        ready(cx);
        cx.simulate_keystrokes("ctrl-a");
        cx.simulate_input(query);
        ready(cx);
        update(&view, cx, |shell, _, _| {
            assert_eq!(shell.words.locale().tag(), "ko");
            assert_eq!(shell.areas.palette.query, query);
            assert!(
                snapshot::project(shell)
                    .sections
                    .iter()
                    .flat_map(|section| &section.rows)
                    .any(|row| matches!(&row.id, snapshot::Identity::Registered(id)
                    if id.source.as_str() == "workspace-layout" && id.entry.as_str() == expected)),
                "missing {expected} for hidden alias {query} at {width}"
            );
        });
    }
}

#[gpui::test]
fn captured_layout_command_cannot_close_a_newly_selected_workspace(cx: &mut TestAppContext) {
    use vessel_app_view::layout::{HostAlias, LayoutKey, OpenIntent, Placement};
    let (view, cx) = harness(cx);
    let first = SessionId::random();
    let target = update(&view, cx, |shell, window, cx| {
        shell.route = Route::Session;
        shell.areas.session.open = Some(Open {
            machine: "one".to_owned(),
            session: first,
            title: "A".to_owned(),
            agent: "fake".to_owned(),
            structured: false,
            live: true,
        });
        let key = LayoutKey::Session {
            machine: HostAlias::parse("one").unwrap(),
            session: first,
        };
        let _tab = shell
            .layout_open_target(
                &key,
                vessel_app_view::layout::Target::NewTab,
                OpenIntent::New,
                Placement::Focused,
                window,
                cx,
            )
            .unwrap();
        source::refresh_dynamic(shell, window);
        shell.palette_searched("close current tab".to_owned(), cx);
        snapshot::project(shell).sections.into_iter().flat_map(|section| section.rows)
            .find(|row| matches!(&row.id, snapshot::Identity::Registered(id)
                if id.source.as_str() == "workspace-layout" && id.entry.as_str() == "tab:close-current"))
            .unwrap().activate
    });
    update(&view, cx, |shell, window, cx| {
        shell.areas.session.open.as_mut().unwrap().session = SessionId::random();
        shell.activate_command(target, window, cx);
        assert!(
            shell.areas.palette.is_open(),
            "stale source is refused before closing the palette"
        );
        assert_eq!(
            shell
                .prefs
                .layouts
                .iter()
                .find(|entry| matches!(entry.key,
            LayoutKey::Session { session, .. } if session == first))
                .unwrap()
                .layout
                .tabs()
                .count(),
            1
        );
    });
}
