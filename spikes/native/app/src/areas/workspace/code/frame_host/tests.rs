//! The real book supplies the host; stale legacy tab/open state cannot select
//! its body or invent a root. This is construction, not painted click evidence.
use super::*;
use gpui::TestAppContext;
use std::cell::Cell;
use vessel_app_kit::{Scheme, ui::div};
use vessel_app_view::layout::{ExplorerVisibility, LayoutKey, OpenIntent, Placement};
use vessel_proto::SessionId;

#[gpui::test]
fn typed_explorer_mount_uses_the_actual_book_while_legacy_flags_disagree(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let session = SessionId::random();
    let (shell, cx) = cx.add_window_view(move |window, cx| {
        let mut shell = Shell::without_io(window, cx);
        shell.route = crate::routes::Route::Session;
        shell.areas.session.open = Some(crate::areas::session::Open {
            machine: "isolated".to_owned(),
            session,
            title: "Explorer fixture".to_owned(),
            agent: "fake".to_owned(),
            structured: true,
            live: true,
        });
        shell
    });
    shell.update_in(cx, |shell, window, cx| {
        use crate::areas::layout::registry::PanelFocus;
        use crate::areas::workspace::{ExplorerHost, code_beside_at};
        let key = LayoutKey::Session {
            machine: "isolated".parse().unwrap(),
            session,
        };
        let tab = shell
            .layout_open_target(
                &key,
                Target::Files,
                OpenIntent::Reveal,
                Placement::Explorer,
                window,
                cx,
            )
            .unwrap();
        shell.areas.workspace.code.open = false;
        shell.areas.workspace.code.tab = Tab::Changes;
        assert!(shell.areas.workspace.code.target.is_none());
        let calls = Cell::new(0);
        let rendered = shell.layout_with_explorer(PanelFocus::Background, |panel| {
            let host = ExplorerHost {
                panel,
                select: Rc::new(|_, _, _, _| panic!("construction must not select")),
                hide: Rc::new(|_, _, _| panic!("construction must not hide")),
            };
            assert_eq!(host.selected(), Some(ExplorerKind::Files));
            assert_eq!(panel.workspace, &key);
            assert_eq!(panel.tab, tab);
            let _element = code_beside_at(
                shell,
                div(),
                &host,
                &Tokens::of(Scheme::Dark),
                window,
                cx,
                |_, received, _, _, _| {
                    assert!(std::ptr::eq(received, panel));
                    calls.set(calls.get() + 1);
                    div().into_any_element()
                },
            );
            true
        });
        assert_eq!(rendered, Some(true));
        assert_eq!(calls.get(), 1);
        assert!(shell.areas.workspace.code.target.is_none());
        assert!(!shell.areas.workspace.code.open);
        assert_eq!(shell.areas.workspace.code.tab, Tab::Changes);
        shell
            .update_layout(
                &key,
                |layout| {
                    layout.show_explorer(ExplorerVisibility::Hidden);
                    Ok(())
                },
                cx,
            )
            .unwrap();
        assert_eq!(
            shell.layout_with_explorer(PanelFocus::Background, |_| {
                calls.set(calls.get() + 1);
                true
            }),
            None
        );
        assert_eq!(calls.get(), 1);
    });
}
