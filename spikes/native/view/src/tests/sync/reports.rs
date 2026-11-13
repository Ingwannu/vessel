//! Missing reports must not become a confirmed zero, including partial replies.
use super::{member, now};
use crate::Lang;
use crate::sync::{KeptEdits, MemberBadge, MemberRow, MemberState, member_line};
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
    for (reply, kept, en, ko, badge) in [
        (
            never.clone(),
            KeptEdits::Unreported,
            None,
            None,
            MemberBadge::Unreported,
        ),
        (
            FleetMember {
                applied: Some(Generation::new(3)),
                ..never.clone()
            },
            KeptEdits::Known(0),
            Some("Version 3"),
            Some("버전 3"),
            MemberBadge::Current,
        ),
        (
            FleetMember {
                reported_at: member().reported_at,
                ..never.clone()
            },
            KeptEdits::Known(0),
            Some("Reported 1 minute ago"),
            Some("1분 전 보고"),
            MemberBadge::VersionUnreported,
        ),
        (
            FleetMember {
                conflicts: vec!["skills/한글 안내.md".parse().unwrap()],
                ..never
            },
            KeptEdits::Known(1),
            None,
            None,
            MemberBadge::VersionUnreported,
        ),
    ] {
        assert_eq!(KeptEdits::of(&reply), kept);
        let state = MemberState::of(&reply, Generation::new(3));
        assert_eq!(
            MemberRow::of(Lang::En, &reply, Some(state), now(), offset),
            MemberRow {
                hint: en.map(str::to_owned),
                badge
            }
        );
        assert_eq!(
            MemberRow::of(Lang::Ko, &reply, Some(state), now(), offset),
            MemberRow {
                hint: ko.map(str::to_owned),
                badge
            }
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
