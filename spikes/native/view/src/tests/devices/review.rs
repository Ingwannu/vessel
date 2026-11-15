//! Screen-review regressions: reader-local dates and invitation lifetime.

use super::{at, phone};
use crate::{
    Lang,
    devices::{
        details, exact,
        time::{Age, Remaining, exact_time},
    },
};
use chrono::{Duration, FixedOffset};

#[test]
fn exact_hover_uses_the_viewers_offset_to_seconds_without_storage_fractions() {
    let offset = FixedOffset::east_opt(9 * 3600).unwrap();
    let device = vessel_proto::DeviceInfo {
        created_at: at("2026-11-11T12:28:27.380091515Z"),
        last_seen: Some(at("2026-11-12T00:00:00.999999999Z")),
        ..phone()
    };
    assert_eq!(
        exact(Lang::En, &device, offset),
        "Paired: 2026-11-11 21:28:27 (UTC+9)\nLast seen: 2026-11-12 09:00:00 (UTC+9)\nAccess: Read information, Run sessions"
    );
    assert_eq!(
        exact(Lang::Ko, &device, offset),
        "페어링: 2026-11-11 21:28:27 (UTC+9)\n마지막 접속: 2026-11-12 09:00:00 (UTC+9)\n권한: 정보 읽기, 세션 실행"
    );
    let west = FixedOffset::west_opt(5 * 3600 + 30 * 60).unwrap();
    assert_eq!(
        exact_time(at("2026-11-12T03:00:00.999Z"), west),
        "2026-11-11 21:30:00 (UTC-5:30)"
    );
    let east = FixedOffset::east_opt(5 * 3600 + 45 * 60).unwrap();
    assert_eq!(
        exact_time(at("2026-11-11T23:00:00Z"), east),
        "2026-11-12 04:45:00 (UTC+5:45)"
    );
}

#[test]
fn older_calendar_dates_are_the_viewers_date_in_both_languages() {
    let offset = FixedOffset::east_opt(9 * 3600).unwrap();
    let paired = at("2026-11-01T20:00:00Z");
    let now = at("2026-11-12T00:00:00Z");
    let device = vessel_proto::DeviceInfo {
        created_at: paired,
        last_seen: Some(paired),
        ..phone()
    };
    assert_eq!(
        Age::of(paired, now, offset),
        Age::Date(at("2026-11-02T00:00:00Z").date_naive())
    );
    assert_eq!(
        details(Lang::En, &device, now, offset),
        "Run sessions · seen on Nov 2"
    );
    assert_eq!(
        details(Lang::Ko, &device, now, offset),
        "세션 실행 · 11월 2일 접속"
    );
}

#[test]
fn an_invitation_counts_down_whole_minutes_then_says_it_is_expired() {
    let now = at("2026-11-13T14:11:22Z");
    let expiry = now + Duration::minutes(10);
    for (elapsed, en, ko) in [
        (0, "Expires in 10 minutes", "10분 뒤 만료"),
        (1, "Expires in 9 minutes", "9분 뒤 만료"),
        (60_000, "Expires in 9 minutes", "9분 뒤 만료"),
        (60_001, "Expires in 8 minutes", "8분 뒤 만료"),
        (540_000, "Expires in 1 minute", "1분 뒤 만료"),
        (540_001, "Expires in less than a minute", "1분 안에 만료"),
        (599_999, "Expires in less than a minute", "1분 안에 만료"),
        (
            600_000,
            "This invitation expired. Make a new link.",
            "초대가 만료되었습니다. 새 링크를 만드세요.",
        ),
        (
            660_000,
            "This invitation expired. Make a new link.",
            "초대가 만료되었습니다. 새 링크를 만드세요.",
        ),
    ] {
        let remaining = Remaining::of(expiry, now + Duration::milliseconds(elapsed));
        assert_eq!(Lang::En.devices().expires(remaining), en);
        assert_eq!(Lang::Ko.devices().expires(remaining), ko);
    }
}

#[test]
fn countdown_wakes_at_each_word_change_including_the_exact_expiry() {
    let now = at("2026-11-13T14:11:22Z");
    for millis in [600_000, 599_999, 90_000, 60_000, 59_999, 1] {
        let expiry = now + Duration::milliseconds(millis);
        let wait = Remaining::next_change(expiry, now).unwrap();
        let change = now + Duration::from_std(wait).unwrap();
        assert_ne!(Remaining::of(expiry, now), Remaining::of(expiry, change));
        assert_eq!(
            Remaining::of(expiry, now),
            Remaining::of(expiry, change - Duration::milliseconds(1))
        );
    }
    assert_eq!(Remaining::next_change(now, now), None);
    assert_eq!(
        Remaining::next_change(now - Duration::seconds(1), now),
        None
    );
}
