//! Real shell dispatch with canonical attachment/context storage and no transport.
use super::*;
use crate::shell::language_tests;
use gpui::TestAppContext;
use vessel_app_view::composer::context::{
    ContextAttachment, ContextId, ContextKind, ContextText, ContextTitle, Payload,
};
use vessel_app_view::composer::tray::Origin;

#[gpui::test]
fn retained_files_and_context_prevent_client_archive_and_plain_commands_use_lifecycle(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    cx.run_until_parked();
    for case in 0..3 {
        shell.update_in(cx, |shell, window, cx| {
            let session = vessel_proto::SessionId::random();
            let target = DraftKey::Session { session };
            shell.route = Route::Session;
            shell.areas.session.open = Some(crate::areas::session::Open {
                machine: "fixture".to_owned(),
                session,
                title: String::new(),
                agent: String::new(),
                structured: true,
                live: true,
                archived_at: None,
                binding: None,
            });
            shell
                .layout_open_target(
                    &vessel_app_view::layout::LayoutKey::Session {
                        machine: "fixture".parse().unwrap(),
                        session,
                    },
                    vessel_app_view::layout::Target::Agent(session),
                    vessel_app_view::layout::OpenIntent::Reveal,
                    vessel_app_view::layout::Placement::Focused,
                    window,
                    cx,
                )
                .unwrap();
            shell
                .areas
                .session
                .composer
                .replace_all("/exit", window, cx);
            shell.notice = None;
            if case == 0 {
                let item = shell.areas.composer.trays.tray(target).add(
                    "image.png",
                    1,
                    Origin::Pasted(vec![1].into()),
                );
                assert!(item.is_some());
            } else if case == 1 {
                shell
                    .areas
                    .composer
                    .contexts
                    .tray(target)
                    .add(ContextAttachment::Snapshot {
                        kind: ContextKind::ChatHistory,
                        payload: Payload {
                            id: ContextId::parse("fixture").unwrap(),
                            title: ContextTitle::parse("History").unwrap(),
                            text: ContextText::parse("retained context").unwrap(),
                            url: None,
                        },
                    })
                    .unwrap();
            }
            assert!(!shell.composer_client_command("/exit arguments", window, cx));
            let handled = shell.composer_client_command(" /exit ", window, cx);
            assert_eq!(handled, case == 2);
            assert_eq!(shell.areas.session.composer.text(cx), "/exit");
            assert_eq!(shell.areas.session.open.as_ref().unwrap().session, session);
            if case == 2 {
                // No link delivers an acknowledgement in this fixture. The real
                // lifecycle is pending, so command handling cannot navigate early.
                assert!(shell.sessions_lifecycle_busy());
                assert_eq!(shell.route, Route::Session);
            } else {
                assert!(shell.notice.is_none());
            }
        });
    }
}

#[gpui::test]
fn unavailable_context_scope_is_not_admitted_and_history_seeds_only_its_new_draft(
    cx: &mut TestAppContext,
) {
    use vessel_app_view::composer::context::{ChatHistory, ContextProblem, ContextScope};
    use vessel_proto::{ProjectId, SessionId, workspace::WorkspaceRoot};

    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let payload = Payload {
            id: ContextId::parse("fixture").unwrap(),
            title: ContextTitle::parse("History").unwrap(),
            text: ContextText::parse("retained history").unwrap(),
            url: None,
        };
        let scope = ContextScope {
            machine: "fixture".parse().unwrap(),
            root: WorkspaceRoot::Project {
                project: ProjectId::random(),
            },
        };
        let text = shell.areas.draft.composer.text(cx);
        assert_eq!(
            shell.composer_attach_context(
                DraftKey::New,
                &scope,
                ContextAttachment::Snapshot {
                    kind: ContextKind::Review,
                    payload: payload.clone(),
                },
                window,
                cx,
            ),
            Err(ContextProblem::Scope)
        );
        assert!(!shell.composer_has_context(DraftKey::New));
        let other = DraftKey::Session {
            session: SessionId::random(),
        };
        assert_eq!(
            shell.composer_seed_history(other, ChatHistory::new(payload.clone()), window, cx),
            Err(ContextProblem::Scope)
        );
        assert!(!shell.composer_has_context(other));
        shell
            .composer_seed_history(DraftKey::New, ChatHistory::new(payload), window, cx)
            .unwrap();
        assert!(shell.composer_has_context(DraftKey::New));
        assert_eq!(shell.areas.draft.composer.text(cx), text);
    });
}
