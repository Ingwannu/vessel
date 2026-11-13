//! Missing reports must not become a confirmed zero, including partial replies.
use super::{member, now};
use crate::Lang;
use crate::sync::{KeptEdits, MemberState, member_details, member_line};
use chrono::FixedOffset;
use vessel_proto::{FleetMember, Generation};

#[test]
fn a_report_default_and_partial_report_facts_have_different_complete_sentences() {
    let offset = FixedOffset::east_opt(0).unwrap();
    let never = FleetMember {
        applied: None,
        reported_at: None,
        ..member()
    };
    for (reply, kept, en, ko) in [
        (
            never.clone(),
            KeptEdits::Unreported,
            "No applied version reported.\nNo local-change report received.\nNo report time received.",
            "적용한 버전의 보고가 없습니다.\n로컬 수정의 보고가 없습니다.\n보고받은 시각이 없습니다.",
        ),
        (
            FleetMember {
                applied: Some(Generation::new(3)),
                ..never.clone()
            },
            KeptEdits::Known(0),
            "Applied version 3.\nNo local edits kept.\nNo report time received.",
            "적용한 버전: 3.\n유지된 로컬 수정이 없습니다.\n보고받은 시각이 없습니다.",
        ),
        (
            FleetMember {
                reported_at: member().reported_at,
                ..never.clone()
            },
            KeptEdits::Known(0),
            "No applied version reported.\nNo local edits kept.\nLast reported 1 minute ago.",
            "적용한 버전의 보고가 없습니다.\n유지된 로컬 수정이 없습니다.\n마지막 보고: 1분 전.",
        ),
        (
            FleetMember {
                conflicts: vec!["skills/한글 안내.md".parse().unwrap()],
                ..never
            },
            KeptEdits::Known(1),
            "No applied version reported.\nKept a local edit in 1 file.\nNo report time received.",
            "적용한 버전의 보고가 없습니다.\n파일 1개의 로컬 수정을 유지했습니다.\n보고받은 시각이 없습니다.",
        ),
    ] {
        assert_eq!(KeptEdits::of(&reply), kept);
        let state = MemberState::of(&reply, Generation::new(3));
        assert_eq!(
            member_details(Lang::En, &reply, Some(state), now(), offset),
            en
        );
        assert_eq!(
            member_details(Lang::Ko, &reply, Some(state), now(), offset),
            ko
        );
    }
}

#[test]
fn the_legacy_row_helper_does_not_reintroduce_the_absent_report_inference() {
    let never = FleetMember {
        applied: None,
        reported_at: None,
        ..member()
    };
    assert_eq!(
        member_line(Lang::En, &never, now()),
        ["Has not reported", "No local-change report received."]
    );
    assert_eq!(
        member_line(Lang::Ko, &never, now()),
        ["보고 없음", "로컬 수정의 보고가 없습니다."]
    );
}
