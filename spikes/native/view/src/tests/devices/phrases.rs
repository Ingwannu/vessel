//! Paseo time.test.ts cases plus the whole sentences shown on device
//! rows. Security redaction assertions live separately from UI wording.

use super::{at, phone};
use crate::{
    Lang,
    badge::Variant,
    devices::{DeviceState, Problem, details, exact, time::Age},
};
use chrono::Duration;
use vessel_proto::ErrorCode;

#[test]
fn paseo_time_ago_inputs_produce_the_same_english_labels() {
    let now = at("2026-07-16T12:00:00Z");
    for (date, expected) in [
        ("2026-07-16T11:59:55Z", "just now"),
        ("2026-07-16T11:59:30Z", "just now"),
        ("2026-07-16T11:59:00Z", "1m ago"),
        ("2026-07-16T11:55:00Z", "5m ago"),
        ("2026-07-16T10:00:00Z", "2h ago"),
        ("2026-07-13T12:00:00Z", "3d ago"),
        ("2026-01-15T12:00:00Z", "Jan 15"),
    ] {
        assert_eq!(
            Lang::En.devices().compact_age(Age::of(at(date), now)),
            expected
        );
    }
    for seconds in 0..60 {
        assert_eq!(
            Lang::En
                .devices()
                .compact_age(Age::of(now - Duration::seconds(seconds), now)),
            "just now"
        );
    }
    assert_eq!(Age::of(now + Duration::seconds(1), now), Age::Now);
    assert!(matches!(
        Age::of(now - Duration::days(7) + Duration::milliseconds(1), now),
        Age::Days(6)
    ));
    assert_eq!(
        Age::of(now - Duration::days(7), now),
        Age::Date(at("2026-07-09T12:00:00Z").date_naive())
    );
    assert_eq!(
        Lang::En
            .devices()
            .compact_age(Age::of(now - Duration::days(7), now)),
        "Jul 9"
    );
    assert_eq!(
        Lang::Ko
            .devices()
            .compact_age(Age::of(now - Duration::days(7), now)),
        "7월 9일"
    );
}

#[test]
fn device_badges_describe_access_and_never_invent_a_live_connection() {
    let mut device = phone();
    for (seen, revoked, state, variant, en, ko) in [
        (
            Some(at("2026-11-11T06:57:00Z")),
            None,
            DeviceState::Paired,
            Variant::Success,
            "Paired",
            "페어링됨",
        ),
        (
            None,
            None,
            DeviceState::Unseen,
            Variant::Muted,
            "Not seen",
            "접속 전",
        ),
        (
            None,
            Some(at("2026-11-11T06:00:00Z")),
            DeviceState::Revoked,
            Variant::Error,
            "Revoked",
            "해제됨",
        ),
    ] {
        device.last_seen = seen;
        device.revoked_at = revoked;
        assert_eq!(DeviceState::of(&device), state);
        assert_eq!(state.variant(), variant);
        assert_eq!(Lang::En.devices().state(state), en);
        assert_eq!(Lang::Ko.devices().state(state), ko);
    }
}

#[test]
fn row_sentences_and_exact_hover_instants_are_complete_in_both_languages() {
    let now = at("2026-11-11T07:00:00Z");
    assert_eq!(
        details(Lang::En, &phone(), now),
        "Can read, run sessions\nLast seen 3 minutes ago\nPaired 2 hours ago"
    );
    assert_eq!(
        details(Lang::Ko, &phone(), now),
        "권한: 읽기, 세션 실행\n마지막 접속: 3분 전\n페어링: 2시간 전"
    );
    assert_eq!(
        exact(Lang::En, &phone()),
        "Paired: 2026-11-11 05:00:00 UTC\nLast seen: 2026-11-11 06:57:00 UTC"
    );
    assert_eq!(
        exact(Lang::Ko, &phone()),
        "페어링: 2026-11-11 05:00:00 UTC\n마지막 접속: 2026-11-11 06:57:00 UTC"
    );
    let unseen = vessel_proto::DeviceInfo {
        last_seen: None,
        ..phone()
    };
    assert_eq!(
        details(Lang::En, &unseen, now),
        "Can read, run sessions\nNo connection recorded\nPaired 2 hours ago"
    );
    assert_eq!(
        details(Lang::Ko, &unseen, now),
        "권한: 읽기, 세션 실행\n기록된 접속이 없습니다\n페어링: 2시간 전"
    );
    assert_eq!(
        exact(Lang::En, &unseen),
        "Paired: 2026-11-11 05:00:00 UTC\nLast seen: Never"
    );
    assert_eq!(
        exact(Lang::Ko, &unseen),
        "페어링: 2026-11-11 05:00:00 UTC\n마지막 접속: 접속한 적 없음"
    );
}

const FAILURES: [(Problem, &str, &str); 15] = [
    (
        Problem::NoRoute,
        "This machine has no address for pairing. Set a public address or relay in its Vessel setup. Then make a new link.",
        "이 머신에 페어링용 주소가 없습니다. 머신의 Vessel 설정에서 공인 주소나 릴레이를 지정하세요. 그런 다음 새 링크를 만드세요.",
    ),
    (
        Problem::Offline,
        "The machine is not connected. Reconnect the machine. Then try again.",
        "머신이 연결되어 있지 않습니다. 머신을 연결하세요. 그런 다음 다시 시도하세요.",
    ),
    (
        Problem::InvalidReply,
        "The machine returned an invalid response. Try again.",
        "머신의 응답이 올바르지 않습니다. 다시 시도하세요.",
    ),
    (
        Problem::Exhausted,
        "Reopen the app to continue safely.",
        "계속하려면 앱을 다시 여세요.",
    ),
    (
        Problem::Refused(ErrorCode::Unauthorized),
        "This device cannot manage access. Use an administrator device.",
        "이 기기에는 접근 관리 권한이 없습니다. 관리자 기기를 사용하세요.",
    ),
    (
        Problem::Refused(ErrorCode::Forbidden),
        "This device cannot manage access. Use an administrator device.",
        "이 기기에는 접근 관리 권한이 없습니다. 관리자 기기를 사용하세요.",
    ),
    (
        Problem::Refused(ErrorCode::Unsupported),
        "Update the machine to use this feature.",
        "이 기능을 사용하려면 머신을 업데이트하세요.",
    ),
    (
        Problem::Refused(ErrorCode::ProtocolMismatch),
        "Update the machine to use this feature.",
        "이 기능을 사용하려면 머신을 업데이트하세요.",
    ),
    (
        Problem::Refused(ErrorCode::NotFound),
        "That device no longer exists. Refresh the list.",
        "기기가 더 이상 존재하지 않습니다. 목록을 새로고침하세요.",
    ),
    (
        Problem::Refused(ErrorCode::InvalidRequest),
        "The machine refused the request. Check the request. Then try again.",
        "머신이 요청을 거부했습니다. 요청을 확인하세요. 그런 다음 다시 시도하세요.",
    ),
    (
        Problem::Refused(ErrorCode::Conflict),
        "The machine could not complete the request. Try again.",
        "머신이 요청을 완료하지 못했습니다. 다시 시도하세요.",
    ),
    (
        Problem::Refused(ErrorCode::Unavailable),
        "The machine could not complete the request. Try again.",
        "머신이 요청을 완료하지 못했습니다. 다시 시도하세요.",
    ),
    (
        Problem::Refused(ErrorCode::Internal),
        "The machine could not complete the request. Try again.",
        "머신이 요청을 완료하지 못했습니다. 다시 시도하세요.",
    ),
    (
        Problem::Refused(ErrorCode::HarnessNotInstalled),
        "The machine could not complete the request. Try again.",
        "머신이 요청을 완료하지 못했습니다. 다시 시도하세요.",
    ),
    (
        Problem::Refused(ErrorCode::SpawnFailed),
        "The machine could not complete the request. Try again.",
        "머신이 요청을 완료하지 못했습니다. 다시 시도하세요.",
    ),
];

#[test]
fn every_failure_and_action_has_a_whole_localized_explanation_and_next_step() {
    for (why, en, ko) in FAILURES {
        for (lang, reason, list, invite, revoke) in [
            (
                Lang::En,
                en,
                "Could not list devices.",
                "Could not make a pairing link.",
                "Could not confirm access was revoked.",
            ),
            (
                Lang::Ko,
                ko,
                "기기를 불러오지 못했습니다.",
                "페어링 링크를 만들지 못했습니다.",
                "접근 해제 결과를 확인하지 못했습니다.",
            ),
        ] {
            let words = lang.devices();
            assert_eq!(words.list_problem(why), format!("{list} {reason}"));
            assert_eq!(words.invite_problem(why), format!("{invite} {reason}"));
            assert_eq!(words.revoke_problem(why), format!("{revoke} {reason}"));
        }
    }
}

#[test]
fn every_age_reads_as_a_whole_phrase_in_both_languages() {
    for (age, en, ko) in [
        (Age::Now, "just now", "방금"),
        (Age::Minutes(1), "1 minute ago", "1분 전"),
        (Age::Minutes(2), "2 minutes ago", "2분 전"),
        (Age::Hours(1), "1 hour ago", "1시간 전"),
        (Age::Hours(2), "2 hours ago", "2시간 전"),
        (Age::Days(1), "1 day ago", "1일 전"),
        (Age::Days(2), "2 days ago", "2일 전"),
        (
            Age::Date(at("2026-01-15T12:00:00Z").date_naive()),
            "Jan 15",
            "1월 15일",
        ),
    ] {
        assert_eq!(Lang::En.devices().prose_age(age), en);
        assert_eq!(Lang::Ko.devices().prose_age(age), ko);
    }
    assert_eq!(Lang::En.devices().loading_badge, "Loading");
    assert_eq!(Lang::Ko.devices().loading_badge, "불러오는 중");
    assert_eq!(Lang::En.devices().list_failed_badge, "Failed");
    assert_eq!(Lang::Ko.devices().list_failed_badge, "실패");
    assert_eq!(Lang::En.devices().revoke_failed_badge, "Failed");
    assert_eq!(Lang::Ko.devices().revoke_failed_badge, "실패");
}

#[test]
fn every_request_state_has_one_shared_badge_meaning() {
    use crate::devices::{Inviting, Listing, Revoking, Tickets};
    let ticket = Tickets::default().next_ticket().unwrap();
    assert_eq!(Listing::Empty.variant(), Variant::Warning);
    assert_eq!(Listing::Loading(ticket).variant(), Variant::Warning);
    assert_eq!(Listing::Ready(Vec::new()).variant(), Variant::Success);
    assert_eq!(Listing::Failed(Problem::Offline).variant(), Variant::Error);
    assert_eq!(Inviting::Empty.variant(), Variant::Muted);
    assert_eq!(Inviting::Loading(ticket).variant(), Variant::Warning);
    assert_eq!(
        Inviting::Ready(super::pairing(at("2026-11-11T08:00:00Z"))).variant(),
        Variant::Success
    );
    assert_eq!(Inviting::Failed(Problem::Offline).variant(), Variant::Error);
    assert_eq!(Revoking::Empty.variant(), Variant::Muted);
    assert_eq!(Revoking::Confirm(phone()).variant(), Variant::Muted);
    assert_eq!(
        Revoking::Sending {
            ticket,
            device: phone().id
        }
        .variant(),
        Variant::Warning
    );
    assert_eq!(Revoking::Failed(Problem::Offline).variant(), Variant::Error);
    assert_eq!(Lang::En.devices().invite_failed_badge, "Failed");
    assert_eq!(Lang::Ko.devices().invite_failed_badge, "실패");
}

#[test]
fn korean_machine_and_confirmation_values_need_no_chosen_particle() {
    let words = Lang::Ko.devices();
    assert_eq!(words.paired_with("연구실"), "페어링 대상: 연구실");
    assert_eq!(
        words.revoke_title("휴대폰"),
        "다음 기기의 접근을 해제할까요? 휴대폰"
    );
    assert_eq!(
        words.revoke_hint("휴대폰", "연구실"),
        "이 기기의 접근을 즉시 해제하고 열려 있는 연결을 닫습니다. 기기 이름: 휴대폰. 머신 이름: 연구실. 다시 페어링하려면 새 링크가 필요합니다."
    );
    assert_eq!(
        Lang::En.devices().paired_with("laptop"),
        "Paired with laptop"
    );
    assert_eq!(Lang::En.devices().revoke_title("phone"), "Revoke phone?");
    assert_eq!(
        Lang::En.devices().revoke_hint("phone", "laptop"),
        "phone loses access to laptop now, and its open connections close. Pairing it again needs a new link."
    );
}

#[test]
fn notices_have_their_own_five_variant_vocabulary() {
    assert_eq!(
        crate::devices::PAIRING_WARNING,
        crate::alert::Variant::Warning
    );
    for (problem, _, _) in FAILURES {
        assert_eq!(problem.notice(), crate::alert::Variant::Error);
    }
}

#[test]
fn the_pairing_destination_stays_with_the_form_after_the_heading_scrolls_away() {
    assert_eq!(
        Lang::En.devices().name_hint("laptop"),
        "A link that pairs one device with laptop; it works once, for ten minutes."
    );
    assert_eq!(
        Lang::Ko.devices().name_hint("laptop"),
        "페어링 대상: laptop. 10분 동안 한 번만 사용할 수 있는 링크입니다."
    );
}
