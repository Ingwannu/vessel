//! The actual fork refusal handler and live language change, without node I/O.
use crate::shell::language_tests;
use vessel_app_link::{ActAnswer, ActFailed, ActKind};
use vessel_proto::ErrorCode;

#[gpui::test]
fn a_fork_refusal_rewords_without_diagnostics_or_losing_composer_input(
    cx: &mut gpui::TestAppContext,
) {
    language_tests::init(cx);
    language_tests::preserves(
        cx,
        |shell, _window, _cx| {
            shell.fork_answered(&ActAnswer {
                machine: "isolated".to_owned(),
                session: "00000000-0000-0000-0000-000000000001".parse().unwrap(),
                kind: ActKind::Fork,
                result: Err(ActFailed {
                    code: Some(ErrorCode::Unsupported),
                    message: "raw English diagnostic".to_owned(),
                }),
            });
            assert_eq!(
                shell.notice.as_deref(),
                Some("This agent cannot fork conversations.")
            );
        },
        |shell| &shell.areas.session.composer,
        |shell| {
            assert_eq!(
                shell.notice.as_deref(),
                Some("이 에이전트는 대화를 분기할 수 없습니다.")
            );
            assert!(shell.areas.conversation.forking.pending.is_none());
        },
    );
}

#[gpui::test]
fn translating_an_older_fork_refusal_preserves_another_areas_later_notice(
    cx: &mut gpui::TestAppContext,
) {
    language_tests::init(cx);
    language_tests::preserves(
        cx,
        |shell, _window, _cx| {
            shell.fork_answered(&ActAnswer {
                machine: "isolated".to_owned(),
                session: "00000000-0000-0000-0000-000000000001".parse().unwrap(),
                kind: ActKind::Fork,
                result: Err(ActFailed {
                    code: Some(ErrorCode::Internal),
                    message: String::new(),
                }),
            });
            shell.notice = Some("another area's later notice".to_owned());
        },
        |shell| &shell.areas.session.composer,
        |shell| assert_eq!(shell.notice.as_deref(), Some("another area's later notice")),
    );
}
