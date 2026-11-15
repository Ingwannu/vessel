//! One hint per device; detailed metadata is retained outside the list row.
use super::{at, phone};
use crate::{
    Lang,
    badge::Variant,
    devices::{Access, DeviceBadge, details, exact},
};
use chrono::FixedOffset;
use vessel_proto::{DeviceInfo, Scope};

#[test]
fn ordinary_paired_devices_have_no_badge_and_only_proven_identity_says_this_device() {
    let device = phone();
    assert_eq!(DeviceBadge::of(&device, None), None);
    assert_eq!(
        DeviceBadge::of(
            &device,
            Some("00000000-0000-4000-8000-000000000002".parse().unwrap())
        ),
        None
    );
    assert_eq!(
        DeviceBadge::of(&device, Some(device.id)),
        Some(DeviceBadge::ThisDevice)
    );
    let unseen = DeviceInfo {
        last_seen: None,
        ..device.clone()
    };
    assert_eq!(DeviceBadge::of(&unseen, None), Some(DeviceBadge::Unseen));
    assert_eq!(
        DeviceBadge::of(&unseen, Some(device.id)),
        Some(DeviceBadge::ThisDevice)
    );
    let revoked = DeviceInfo {
        revoked_at: Some(at("2026-11-11T07:00:00Z")),
        ..device.clone()
    };
    assert_eq!(DeviceBadge::of(&revoked, None), Some(DeviceBadge::Revoked));
    assert_eq!(
        DeviceBadge::of(&revoked, Some(device.id)),
        Some(DeviceBadge::Revoked)
    );
    for (badge, variant, en, ko) in [
        (
            DeviceBadge::ThisDevice,
            Variant::Success,
            "This device",
            "이 기기",
        ),
        (DeviceBadge::Unseen, Variant::Muted, "Not seen", "접속 전"),
        (DeviceBadge::Revoked, Variant::Error, "Revoked", "해제됨"),
    ] {
        assert_eq!(badge.variant(), variant);
        assert_eq!(Lang::En.devices().state(badge), en);
        assert_eq!(Lang::Ko.devices().state(badge), ko);
    }
}
#[test]
fn the_primary_access_label_never_calls_independent_sync_or_relay_grants_read_only() {
    let now = at("2026-11-11T07:00:00Z");
    let offset = FixedOffset::east_opt(0).unwrap();
    for (scopes, access, en, ko) in [
        (
            vec![],
            Access::None,
            "No access · seen 3 minutes ago",
            "권한 없음 · 3분 전 접속",
        ),
        (
            vec![Scope::Read],
            Access::ReadOnly,
            "Read only · seen 3 minutes ago",
            "읽기 전용 · 3분 전 접속",
        ),
        (
            vec![Scope::Operate],
            Access::Sessions,
            "Run sessions · seen 3 minutes ago",
            "세션 실행 · 3분 전 접속",
        ),
        (
            vec![Scope::Admin],
            Access::Manage,
            "Admin · seen 3 minutes ago",
            "관리 · 3분 전 접속",
        ),
        (
            vec![Scope::Sync],
            Access::Sharing,
            "Settings sync · seen 3 minutes ago",
            "설정 동기화 · 3분 전 접속",
        ),
        (
            vec![Scope::Relay],
            Access::Forwarding,
            "Other machine access · seen 3 minutes ago",
            "다른 머신 접근 · 3분 전 접속",
        ),
        (
            vec![Scope::Read, Scope::Sync, Scope::Relay],
            Access::Sharing,
            "Settings sync · seen 3 minutes ago",
            "설정 동기화 · 3분 전 접속",
        ),
        (
            Scope::ALL.into_iter().rev().collect(),
            Access::Manage,
            "Admin · seen 3 minutes ago",
            "관리 · 3분 전 접속",
        ),
    ] {
        let device = DeviceInfo { scopes, ..phone() };
        assert_eq!(Access::of(&device.scopes), access);
        assert_eq!(details(Lang::En, &device, now, offset), en);
        assert_eq!(details(Lang::Ko, &device, now, offset), ko);
    }
}
#[test]
fn every_grant_and_the_exact_pairing_date_remain_available_in_the_tooltip() {
    let device = DeviceInfo {
        scopes: Scope::ALL.to_vec(),
        ..phone()
    };
    let offset = FixedOffset::east_opt(9 * 3600).unwrap();
    assert_eq!(
        exact(Lang::En, &device, offset),
        "Paired: 2026-11-11 14:00:00 (UTC+9)\nLast seen: 2026-11-11 15:57:00 (UTC+9)\nAccess: Read information, Run sessions, Manage machine, Receive shared settings, Access other machines"
    );
    assert_eq!(
        exact(Lang::Ko, &device, offset),
        "페어링: 2026-11-11 14:00:00 (UTC+9)\n마지막 접속: 2026-11-11 15:57:00 (UTC+9)\n권한: 정보 읽기, 세션 실행, 머신 관리, 공유 설정 받기, 다른 머신 접근"
    );
    assert_eq!(Lang::En.devices().granted_scopes(&[]), "None");
    assert_eq!(Lang::Ko.devices().granted_scopes(&[]), "없음");
}
