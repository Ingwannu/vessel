//! Exercise real retained input and IME-aware hooks without a node or credentials.

use super::*;
use crate::shell::language_tests;
use gpui::{TestAppContext, size};

#[gpui::test]
fn fullscreen_escape_keeps_selection_text_and_button_only_enter(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    cx.simulate_resize(size(px(390.0), px(844.0)));
    cx.run_until_parked();
    shell.update_in(cx, |shell, window, cx| {
        window.activate_window();
        shell.areas.draft.composer.focus(window, cx);
        shell
            .areas
            .draft
            .composer
            .replace_all("one\ntwo\n한글 three", window, cx);
        cx.notify();
    });
    cx.run_until_parked();
    let focus = shell.read_with(cx, |shell, cx| shell.areas.draft.composer.focus_handle(cx));
    shell.update_in(cx, |shell, window, cx| {
        shell.composer_expand(DraftKey::New, window, cx);
        assert_eq!(
            shell.areas.composer_shell.surface(DraftKey::New).extent(),
            Extent::Fullscreen
        );
    });
    cx.run_until_parked();
    cx.simulate_keystrokes("left shift-left");
    cx.run_until_parked();
    let before = shell.read_with(cx, |shell, cx| {
        (
            shell.areas.draft.composer.text(cx),
            shell.areas.draft.composer.selection(cx),
        )
    });
    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    shell.read_with(cx, |shell, cx| {
        assert_eq!(
            shell.areas.composer_shell.surface(DraftKey::New).extent(),
            Extent::Inline
        );
        assert_eq!(shell.areas.draft.composer.text(cx), before.0);
        assert_eq!(shell.areas.draft.composer.selection(cx), before.1);
        assert_eq!(shell.areas.draft.composer.focus_handle(cx), focus);
    });
    cx.simulate_keystrokes("right enter");
    cx.run_until_parked();
    shell.read_with(cx, |shell, cx| {
        assert!(shell.areas.draft.composer.text(cx).contains('\n'));
        assert!(shell.areas.draft.creation.flow.may_submit());
        assert_eq!(
            shell.areas.draft.composer.text(cx).len(),
            before.0.len().saturating_add(1)
        );
    });
}

#[gpui::test]
fn empty_send_cannot_close_fullscreen_and_readonly_cannot_leave_it_open(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    cx.simulate_resize(size(px(390.0), px(844.0)));
    cx.run_until_parked();
    shell.update_in(cx, |shell, window, cx| {
        shell.areas.draft.composer.replace_all("\n\n\n", window, cx);
        cx.notify();
    });
    cx.run_until_parked();
    shell.update_in(cx, |shell, window, cx| {
        shell.composer_expand(DraftKey::New, window, cx);
        assert_eq!(
            shell.areas.composer_shell.surface(DraftKey::New).extent(),
            Extent::Fullscreen
        );
        shell.composer_submit_fullscreen(DraftKey::New, window, cx);
        assert_eq!(
            shell.areas.composer_shell.surface(DraftKey::New).extent(),
            Extent::Fullscreen
        );
        shell
            .areas
            .draft
            .composer
            .set_editability(vessel_app_kit::Editability::ReadOnly, cx);
        let element = render(shell, &Tokens::of(vessel_app_kit::Scheme::Dark), window, cx);
        assert!(element.is_none());
        assert_eq!(
            shell.areas.composer_shell.surface(DraftKey::New).extent(),
            Extent::Inline
        );
        assert_eq!(shell.areas.draft.composer.text(cx), "\n\n\n");
    });
}

#[gpui::test]
fn file_sheet_hides_fullscreen_without_clearing_or_submitting_the_underlying_composer(
    cx: &mut TestAppContext,
) {
    use crate::{areas::session::Open, routes::Route};
    use vessel_app_view::{
        composer_shell::surface::{Layout, RenderedLines},
        layout::{HostAlias, LayoutKey, OpenIntent, Placement, Target},
    };
    use vessel_proto::{ProjectId, SessionId, workspace::WorkspaceRoot};

    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    cx.simulate_resize(size(px(390.0), px(844.0)));
    cx.run_until_parked();
    shell.update_in(cx, |shell, window, cx| {
        let session = SessionId::random();
        let target = DraftKey::Session { session };
        let key = LayoutKey::Session {
            machine: HostAlias::parse("fixture").unwrap(),
            session,
        };
        shell.route = Route::Session;
        shell.areas.session.open = Some(Open {
            machine: "fixture".to_owned(),
            session,
            title: String::new(),
            agent: "fake".to_owned(),
            structured: true,
            live: true,
            archived_at: None,
            binding: None,
        });
        let field = shell.areas.session.composer.clone();
        field.replace_all("one\ntwo\nthree", window, cx);
        let retained = (field.text(cx), field.selection(cx), field.focus_handle(cx));
        assert_eq!(
            shell.composer_fullscreen_target(),
            None,
            "no panel fallback"
        );
        let _agent = shell
            .layout_open_target(
                &key,
                Target::Agent(session),
                OpenIntent::Reveal,
                Placement::Focused,
                window,
                cx,
            )
            .unwrap();
        assert_eq!(shell.composer_fullscreen_target(), Some(target));
        shell
            .areas
            .composer_shell
            .surface_mut(target)
            .expand(Layout::Compact, RenderedLines::new(3));
        let tokens = Tokens::of(vessel_app_kit::Scheme::Dark);
        assert!(render(shell, &tokens, window, cx).is_some());
        let sheet = shell
            .layout_open_target(
                &key,
                Target::File {
                    root: WorkspaceRoot::Project {
                        project: ProjectId::random(),
                    },
                    path: "src/main.rs".parse().unwrap(),
                },
                OpenIntent::New,
                Placement::Focused,
                window,
                cx,
            )
            .unwrap();
        assert_eq!(shell.composer_fullscreen_target(), None);
        assert!(render(shell, &tokens, window, cx).is_none());
        assert!(!shell.composer_may_submit(target, cx));
        shell.composer_submit_fullscreen(target, window, cx);
        shell.composer_collapse(target, window, cx);
        assert_eq!(
            shell.areas.composer_shell.surface(target).extent(),
            Extent::Fullscreen
        );
        assert_eq!(
            (field.text(cx), field.selection(cx), field.focus_handle(cx)),
            retained
        );
        shell.areas.composer_shell.collapse(target);
        shell.composer_expand(target, window, cx);
        assert_eq!(
            shell.areas.composer_shell.surface(target).extent(),
            Extent::Inline
        );
        // A File sheet is not dismissed through a generic layout change until
        // its File owner joins (docs/plans/wp-layout-file-close.md): the sheet
        // stays, so fullscreen stays hidden and the composer keeps its text.
        // When the owner's close path lands, dismiss through it here and
        // expect `Some(target)` again.
        assert_eq!(
            shell.update_layout(&key, |layout| layout.dismiss_sheet(sheet), cx),
            Err(vessel_app_view::layout::LayoutError::UnsupportedHost)
        );
        assert_eq!(shell.composer_fullscreen_target(), None);
        assert_eq!(
            (field.text(cx), field.selection(cx), field.focus_handle(cx)),
            retained
        );
    });
}
