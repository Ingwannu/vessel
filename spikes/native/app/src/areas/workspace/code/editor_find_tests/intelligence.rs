//! Saved-source queries must inspect the actual engine before its queued mirror.
use super::*;
use crate::areas::workspace::code::intelligence::{Refusal, State};
use vessel_app_kit::code_editor::FindCase;
use vessel_app_view::workspace::intelligence::IntelligenceAction;

#[gpui::test]
fn language_query_refuses_silent_accepted_input_before_its_owner_mirror(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (probe, cx) = cx.add_window_view(Probe::new);
    probe.update_in(cx, |probe, window, cx| {
        probe.shell.update(cx, |shell, cx| {
            let key = shell.areas.workspace.code.file_key().unwrap();
            let editor = editing(shell).editor.as_ref().unwrap().clone();
            shell.language_query_for(&key, IntelligenceAction::Hover, window, cx);
            assert!(
                matches!(
                    &shell
                        .areas
                        .workspace
                        .code
                        .viewer_at(&key)
                        .unwrap()
                        .intelligence,
                    State::Loading(_)
                ),
                "the saved baseline has a valid language request scope"
            );
            editor.open_find(cx);
            editor.find_query("alpha", FindCase::Sensitive, cx);
            assert_eq!(
                editor.find_replace("unsaved", ReplaceScope::All, window, cx),
                MatchCount(2)
            );
            assert!(!editing(shell).model.modified());
            assert_eq!(editor.text(cx), "unsaved beta unsaved");
            for action in [
                IntelligenceAction::Hover,
                IntelligenceAction::Definition,
                IntelligenceAction::Diagnostics,
            ] {
                shell.language_query_for(&key, action, window, cx);
                assert!(matches!(
                    &shell
                        .areas
                        .workspace
                        .code
                        .viewer_at(&key)
                        .unwrap()
                        .intelligence,
                    State::Refused(Refusal::Unsaved)
                ));
                assert_eq!(editing(shell).model.content(), "unsaved beta unsaved");
                assert!(editing(shell).model.modified());
                assert!(editing(shell).writing.is_none());
            }
        });
    });
}

#[gpui::test]
fn language_query_waits_for_a_pending_disk_projection_without_overwriting_it(
    cx: &mut TestAppContext,
) {
    crate::shell::language_tests::init(cx);
    let (probe, cx) = cx.add_window_view(Probe::new);
    probe.update_in(cx, |probe, window, cx| {
        probe.shell.update(cx, |shell, cx| {
            let key = shell.areas.workspace.code.file_key().unwrap();
            let editor = editing(shell).editor.as_ref().unwrap().clone();
            let step = shell.editing_for(&key).unwrap().model.reconcile();
            shell.send_step_for(&key, step);
            let ticket = shell.editing_for(&key).unwrap().reading.unwrap().0;
            shell.editor_read_answered_for(
                &key,
                ticket,
                &super::super::super::editing::whole_read(key.root, key.path.clone()),
                Ok(io_tests::text("new saved disk")),
                cx,
            );
            assert!(editing(shell).stale);
            assert!(!editing(shell).model.modified());
            shell.language_query_for(&key, IntelligenceAction::Hover, window, cx);
            assert!(matches!(
                &shell
                    .areas
                    .workspace
                    .code
                    .viewer_at(&key)
                    .unwrap()
                    .intelligence,
                State::Refused(Refusal::Unsaved)
            ));
            assert_eq!(editing(shell).model.content(), "new saved disk");
            assert_eq!(editor.text(cx), "alpha beta alpha");
            assert!(editing(shell).stale);
        });
    });
}
