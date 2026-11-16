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

fn ready_definition(
    shell: &mut Shell,
    key: &FileKey,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> (
    vessel_app_link::workspace::IntelligenceAsk,
    vessel_proto::lsp::Location,
) {
    use vessel_proto::lsp::{
        Context as LspContext, Freshness, Indexing, Location, Offset, Page, Position, Range,
    };
    shell.language_query_for(key, IntelligenceAction::Definition, window, cx);
    let State::Loading(ask) = &shell
        .areas
        .workspace
        .code
        .viewer_at(key)
        .unwrap()
        .intelligence
    else {
        panic!("the saved fixture must start a language request");
    };
    let ask = ask.clone();
    let point = Position {
        line: std::num::NonZeroU32::MIN,
        column: std::num::NonZeroU32::MIN,
    };
    let location = Location {
        file: std::path::PathBuf::from("other.rs").try_into().unwrap(),
        range: Range {
            start: point,
            end: point,
        },
    };
    shell.language_answered(
        key,
        ask.clone(),
        vessel_app_link::workspace::IntelligenceResult::Definitions(Ok(Page {
            items: vec![location.clone()],
            next: Some(Offset(1)),
            truncated: false,
            context: LspContext {
                files: vec![ask.at.file.clone()],
                observed_at: None,
                indexing: Indexing::Unknown,
                freshness: Freshness::Saved,
                workspace_complete: false,
            },
        })),
    );
    (ask, location)
}

#[gpui::test]
fn old_language_controls_cannot_borrow_a_replacement_results_ticket(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (probe, cx) = cx.add_window_view(Probe::new);
    probe.update_in(cx, |probe, window, cx| {
        probe.shell.update(cx, |shell, cx| {
            let key = shell.areas.workspace.code.file_key().unwrap();
            let (old, location) = ready_definition(shell, &key, window, cx);
            let (replacement, _) = ready_definition(shell, &key, window, cx);
            assert_ne!(old.ticket, replacement.ticket);
            shell.language_next_for(&key, old.ticket, vessel_proto::lsp::Offset(1), cx);
            shell.language_location_for(&key, old.ticket, &location, cx);
            assert_eq!(shell.areas.workspace.code.file_key().as_ref(), Some(&key));
            let State::Ready { ask, .. } = &shell
                .areas
                .workspace
                .code
                .viewer_at(&key)
                .unwrap()
                .intelligence
            else {
                panic!("stale controls must preserve the replacement results");
            };
            assert_eq!(ask.ticket, replacement.ticket);
        });
    });
}

#[gpui::test]
fn language_page_and_location_activation_mirror_silent_input_before_navigation(
    cx: &mut TestAppContext,
) {
    crate::shell::language_tests::init(cx);
    for paginate in [true, false] {
        let (probe, cx) = cx.add_window_view(Probe::new);
        probe.update_in(cx, |probe, window, cx| {
            probe.shell.update(cx, |shell, cx| {
                let key = shell.areas.workspace.code.file_key().unwrap();
                let (ask, location) = ready_definition(shell, &key, window, cx);
                let editor = editing(shell).editor.as_ref().unwrap().clone();
                editor.open_find(cx);
                editor.find_query("alpha", FindCase::Sensitive, cx);
                assert_eq!(
                    editor.find_replace("unsaved", ReplaceScope::All, window, cx),
                    MatchCount(2)
                );
                assert!(!editing(shell).model.modified());
                if paginate {
                    shell.language_next_for(&key, ask.ticket, vessel_proto::lsp::Offset(1), cx);
                } else {
                    shell.language_location_for(&key, ask.ticket, &location, cx);
                }
                assert_eq!(shell.areas.workspace.code.file_key().as_ref(), Some(&key));
                assert_eq!(editing(shell).model.content(), "unsaved beta unsaved");
                assert!(matches!(
                    &shell
                        .areas
                        .workspace
                        .code
                        .viewer_at(&key)
                        .unwrap()
                        .intelligence,
                    State::Idle
                ));
            });
        });
    }
}
