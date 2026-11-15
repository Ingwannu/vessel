//! An actual origin-read refusal, retained across a live language change.
use super::Origin;
use crate::areas::conversation::area::Shown;
use crate::shell::language_tests;
use vessel_app_link::{ActAnswer, ActFailed, ActKind};
use vessel_app_view::timeline::Until;
use vessel_proto::{ErrorCode, EventRecord, EventSeq, SessionEvent, SessionId};

fn source() -> SessionId {
    "00000000-0000-0000-0000-000000000002".parse().unwrap()
}

#[gpui::test]
fn an_origin_read_refusal_rewords_without_retrying_or_losing_input(cx: &mut gpui::TestAppContext) {
    language_tests::init(cx);
    language_tests::preserves(
        cx,
        |shell, _window, cx| {
            let area = &mut shell.areas.conversation;
            area.shown = Some(Shown {
                machine: "isolated".to_owned(),
                session: "00000000-0000-0000-0000-000000000001".parse().unwrap(),
            });
            let _change = area.timeline.apply(&[EventRecord {
                seq: EventSeq::new(0),
                at: "2026-11-07T09:00:00Z".parse().unwrap(),
                event: SessionEvent::Forked {
                    from: source(),
                    after: Some(EventSeq::new(7)),
                },
            }]);
            area.rows_again();
            area.list.reset(area.rows.len());
            area.origin = Origin::Reading {
                from: source(),
                until: Until::Record(EventSeq::new(7)),
                asked: std::time::Instant::now(),
            };
            shell.origin_answered(
                &ActAnswer {
                    terminal: None,
                    machine: "isolated".to_owned(),
                    session: source(),
                    kind: ActKind::Log,
                    result: Err(ActFailed {
                        code: Some(ErrorCode::Forbidden),
                        message: "raw English diagnostic".to_owned(),
                    }),
                },
                cx,
            );
            let Origin::Unread { notice, .. } = &shell.areas.conversation.origin else {
                panic!("the read result was not retained");
            };
            assert_eq!(
                &*notice.words(),
                "This device is not allowed to read the conversation it was forked from."
            );
        },
        |shell| &shell.areas.session.composer,
        |shell| {
            let Origin::Unread {
                removed,
                notice,
                from,
                until,
            } = &shell.areas.conversation.origin
            else {
                panic!("language change retried or dropped the read");
            };
            assert!(!removed);
            assert_eq!(*from, source());
            assert!(matches!(until, Until::Record(seq) if *seq == EventSeq::new(7)));
            assert_eq!(
                &*notice.words(),
                "이 기기에는 분기한 원래 대화를 읽을 권한이 없습니다."
            );
        },
    );
}
