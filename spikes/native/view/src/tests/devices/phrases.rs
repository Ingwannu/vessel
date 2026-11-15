//! Paseo time.test.ts cases plus the whole sentences shown on device
//! rows. Security redaction assertions live separately from UI wording.

use super::{at, phone};
use crate::{
    Lang,
    badge::Variant,
    devices::{DeviceState, Problem, details, exact, time::Age},
};
use chrono::{Duration, FixedOffset};

fn offset() -> FixedOffset {
    FixedOffset::east_opt(0).unwrap()
}
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
            Lang::En
                .devices()
                .compact_age(Age::of(at(date), now, offset())),
            expected
        );
    }
    for seconds in 0..60 {
        assert_eq!(
            Lang::En.devices().compact_age(Age::of(
                now - Duration::seconds(seconds),
                now,
                offset()
            )),
            "just now"
        );
    }
    assert_eq!(Age::of(now + Duration::seconds(1), now, offset()), Age::Now);
    assert!(matches!(
        Age::of(
            now - Duration::days(7) + Duration::milliseconds(1),
            now,
            offset()
        ),
        Age::Days(6)
    ));
    assert_eq!(
        Age::of(now - Duration::days(7), now, offset()),
        Age::Date(at("2026-07-09T12:00:00Z").date_naive())
    );
    assert_eq!(
        Lang::En
            .devices()
            .compact_age(Age::of(now - Duration::days(7), now, offset())),
        "Jul 9"
    );
    assert_eq!(
        Lang::Ko
            .devices()
            .compact_age(Age::of(now - Duration::days(7), now, offset())),
        "7월 9일"
    );
}

#[test]
fn device_badges_describe_access_and_never_invent_a_live_connection() {
    let mut device = phone();
    for (seen, revoked, state, variant, label) in [
        (
            Some(at("2026-11-11T06:57:00Z")),
            None,
            DeviceState::Paired,
            Variant::Success,
            None,
        ),
        (
            None,
            None,
            DeviceState::Unseen,
            Variant::Muted,
            Some(("Not seen", "접속 전")),
        ),
        (
            None,
            Some(at("2026-11-11T06:00:00Z")),
            DeviceState::Revoked,
            Variant::Error,
            Some(("Revoked", "해제됨")),
        ),
    ] {
        device.last_seen = seen;
        device.revoked_at = revoked;
        assert_eq!(DeviceState::of(&device), state);
        assert_eq!(state.variant(), variant);
        let badge = crate::devices::DeviceBadge::of(&device, None);
        assert_eq!(
            badge.map(|badge| (
                Lang::En.devices().state(badge),
                Lang::Ko.devices().state(badge)
            )),
            label
        );
    }
}

#[test]
fn row_sentences_and_exact_hover_instants_are_complete_in_both_languages() {
    let now = at("2026-11-11T07:00:00Z");
    assert_eq!(
        details(Lang::En, &phone(), now, offset()),
        "Run sessions · seen 3 minutes ago"
    );
    assert_eq!(
        details(Lang::Ko, &phone(), now, offset()),
        "세션 실행 · 3분 전 접속"
    );
    assert_eq!(
        exact(Lang::En, &phone(), offset()),
        "Paired: 2026-11-11 05:00:00 (UTC+0)\nLast seen: 2026-11-11 06:57:00 (UTC+0)\nAccess: Read information, Run sessions"
    );
    assert_eq!(
        exact(Lang::Ko, &phone(), offset()),
        "페어링: 2026-11-11 05:00:00 (UTC+0)\n마지막 접속: 2026-11-11 06:57:00 (UTC+0)\n권한: 정보 읽기, 세션 실행"
    );
    let unseen = vessel_proto::DeviceInfo {
        last_seen: None,
        ..phone()
    };
    assert_eq!(details(Lang::En, &unseen, now, offset()), "Run sessions");
    assert_eq!(details(Lang::Ko, &unseen, now, offset()), "세션 실행");
    assert_eq!(
        exact(Lang::En, &unseen, offset()),
        "Paired: 2026-11-11 05:00:00 (UTC+0)\nLast seen: Never\nAccess: Read information, Run sessions"
    );
    assert_eq!(
        exact(Lang::Ko, &unseen, offset()),
        "페어링: 2026-11-11 05:00:00 (UTC+0)\n마지막 접속: 접속한 적 없음\n권한: 정보 읽기, 세션 실행"
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
            assert_eq!(
                words.list_problem(why, translations(lang)),
                format!("{list} {reason}")
            );
            assert_eq!(
                words.invite_problem(why, translations(lang)),
                format!("{invite} {reason}")
            );
            assert_eq!(
                words.revoke_problem(why, translations(lang)),
                format!("{revoke} {reason}")
            );
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
}

#[test]
fn every_request_state_has_one_shared_badge_meaning() {
    use crate::devices::{Inviting, Listing, Revoking, Tickets};
    let mut tickets = Tickets::default();
    let ticket = tickets.next_ticket().unwrap();
    assert_eq!(Listing::Empty.variant(), Variant::Muted);
    assert_eq!(Listing::Loading(ticket).variant(), Variant::Muted);
    assert_eq!(Listing::Ready(Vec::new()).variant(), Variant::Success);
    assert_eq!(Listing::Failed(Problem::Offline).variant(), Variant::Error);
    assert_eq!(Inviting::Empty.variant(), Variant::Muted);
    assert_eq!(Inviting::Loading(ticket).variant(), Variant::Muted);
    assert_eq!(
        Inviting::Ready(super::pairing(at("2026-11-11T08:00:00Z"))).variant(),
        Variant::Success
    );
    assert_eq!(Inviting::Failed(Problem::Offline).variant(), Variant::Error);
    assert_eq!(Revoking::Empty.variant(), Variant::Muted);
    let mut revoking = super::selected(phone(), &mut tickets);
    assert_eq!(revoking.variant(), Variant::Muted);
    let context = revoking.context().unwrap().clone();
    let intent = revoking.intent().unwrap();
    assert_eq!(revoking.confirm(&context, intent, ticket), Some(phone().id));
    assert_eq!(revoking.variant(), Variant::Muted);
    assert_eq!(Revoking::Failed(Problem::Offline).variant(), Variant::Error);
}

#[test]
fn korean_machine_and_confirmation_values_need_no_chosen_particle() {
    let words = Lang::Ko.devices();
    assert_eq!(words.paired_with("연구실"), "연구실에 페어링된 기기");
    assert_eq!(words.revoke_title("휴대폰"), "휴대폰의 접근을 해제할까요?");
    assert_eq!(
        words.revoke_hint("연구실"),
        "이 기기는 지금부터 연구실에 접근할 수 없으며, 열려 있는 연결도 닫힙니다. 다시 페어링하려면 새 링크가 필요합니다."
    );
    assert_eq!(
        Lang::En.devices().paired_with("laptop"),
        "Paired with laptop"
    );
    assert_eq!(Lang::En.devices().revoke_title("phone"), "Revoke phone?");
    assert_eq!(
        Lang::En.devices().revoke_hint("laptop"),
        "This device loses access to laptop now, and its open connections close. Pairing it again needs a new link."
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
        "laptop에 기기 하나를 페어링하는 링크입니다. 10분 동안 한 번만 사용할 수 있습니다."
    );
}

fn translations(lang: Lang) -> &'static crate::localization::Translator {
    let tag = match lang {
        Lang::En => "en",
        Lang::Ko => "ko",
    };
    crate::localization::Translator::get(crate::localization::Locale::from_tag(tag).unwrap())
        .unwrap()
}

#[test]
fn replaced_connection_refusals_are_whole_sentences_per_action_in_both_languages() {
    for (lang, changed, list, invite, revoke) in [
        (
            Lang::En,
            "The machine’s connection changed. Refresh its devices and try again.",
            "Could not list devices because the machine’s connection changed. Refresh its devices and try again.",
            "Could not make a pairing link because the machine’s connection changed. Refresh its devices and make a new link.",
            "Could not confirm access was revoked because the machine’s connection changed. Refresh its devices before trying again.",
        ),
        (
            Lang::Ko,
            "머신 연결이 바뀌었습니다. 기기 목록을 새로고침한 뒤 다시 시도하세요.",
            "머신 연결이 바뀌어 기기를 불러오지 못했습니다. 기기 목록을 새로고침한 뒤 다시 시도하세요.",
            "머신 연결이 바뀌어 페어링 링크를 만들지 못했습니다. 기기 목록을 새로고침한 뒤 새 링크를 만드세요.",
            "머신 연결이 바뀌어 접근 해제 결과를 확인하지 못했습니다. 다시 시도하기 전에 기기 목록을 새로고침하세요.",
        ),
    ] {
        let words = lang.devices();
        let t = translations(lang);
        assert_eq!(words.problem(Problem::TargetChanged, t), changed);
        assert_eq!(words.list_problem(Problem::TargetChanged, t), list);
        assert_eq!(words.invite_problem(Problem::TargetChanged, t), invite);
        assert_eq!(words.revoke_problem(Problem::TargetChanged, t), revoke);
    }
}
