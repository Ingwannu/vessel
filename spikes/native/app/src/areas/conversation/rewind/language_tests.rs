//! Actual refusal projection and live language change, with a real retained
//! composer value/caret/selection and no node or agent process.
use crate::shell::language_tests;
use vessel_app_link::{ActAnswer, ActFailed, ActKind};
use vessel_proto::ErrorCode;

#[gpui::test]
fn a_refusal_changes_language_without_showing_the_diagnostic_or_losing_input(
    cx: &mut gpui::TestAppContext,
) {
    language_tests::init(cx);
    language_tests::preserves(
        cx,
        |shell, _window, _cx| {
            shell.rewind_answered(&ActAnswer {
                terminal: None,
                machine: "isolated".to_owned(),
                session: "00000000-0000-0000-0000-000000000001".parse().unwrap(),
                kind: ActKind::Rewind,
                result: Err(ActFailed {
                    code: Some(ErrorCode::Unsupported),
                    message: "raw English diagnostic".to_owned(),
                }),
            });
            assert_eq!(
                shell.notice.as_deref(),
                Some("This agent cannot rewind conversations.")
            );
        },
        |shell| &shell.areas.session.composer,
        |shell| {
            assert_eq!(
                shell.notice.as_deref(),
                Some("이 에이전트는 대화를 되감을 수 없습니다.")
            );
            assert!(shell.areas.conversation.rewinding.pending.is_none());
        },
    );
}
