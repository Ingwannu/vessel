//! Selection is tied to the current machine and active records, not names.
use super::*;
use crate::devices::{MachineKey, Problem};
use chrono::{DateTime, Utc};
use vessel_proto::Scope;

fn at(text: &str) -> DateTime<Utc> {
    text.parse().unwrap()
}
fn device() -> DeviceInfo {
    DeviceInfo {
        id: "00000000-0000-4000-8000-000000000001".parse().unwrap(),
        label: "A long 한글 device name".to_owned(),
        created_at: at("2026-11-11T05:00:00.123Z"),
        last_seen: Some(at("2026-11-11T06:57:00.999Z")),
        revoked_at: None,
        scopes: Scope::ALL.to_vec(),
    }
}
fn page() -> Page {
    let mut page = Page::default();
    page.target(Target::Online(MachineKey::from_book("fixture")));
    let mut epochs = crate::devices::ConnectionEpochs::default();
    let context = epochs
        .admit(
            MachineKey::from_book("fixture"),
            "00000000-0000-4000-8000-000000000099".parse().unwrap(),
        )
        .unwrap();
    let _change = page.bind(Some(context));
    page.listing = Listing::Ready(vec![device()]);
    page
}

#[test]
fn selection_refuses_unavailable_targets_lists_and_revoked_or_missing_devices() {
    let mut page = page();
    let id = device().id;
    let missing = "00000000-0000-4000-8000-000000000002".parse().unwrap();
    assert_eq!(
        page.inspect_device(missing),
        Err(SelectionError::DeviceUnavailable)
    );
    assert_eq!(
        page.choose_revoke(missing),
        Err(SelectionError::DeviceUnavailable)
    );
    let revoked = DeviceInfo {
        revoked_at: Some(at("2026-11-12T00:00:00Z")),
        ..device()
    };
    page.listing = Listing::Ready(vec![revoked]);
    assert_eq!(
        page.inspect_device(id),
        Err(SelectionError::DeviceUnavailable)
    );
    assert_eq!(
        page.choose_revoke(id),
        Err(SelectionError::DeviceUnavailable)
    );
    for listing in [
        Listing::Empty,
        Listing::Failed(Problem::Offline),
        Listing::Loading(page.tickets.next_ticket().unwrap()),
    ] {
        page.listing = listing;
        assert_eq!(
            page.inspect_device(id),
            Err(SelectionError::ListUnavailable)
        );
        assert_eq!(page.choose_revoke(id), Err(SelectionError::ListUnavailable));
    }
    for target in [
        Target::Hidden,
        Target::Loading,
        Target::NoMachines,
        Target::Offline(MachineKey::from_book("fixture")),
    ] {
        page.target(target);
        assert_eq!(
            page.inspect_device(id),
            Err(SelectionError::TargetUnavailable)
        );
        assert_eq!(
            page.choose_revoke(id),
            Err(SelectionError::TargetUnavailable)
        );
    }
    assert!(page.inspected_device().is_none());
    assert!(matches!(page.revoking, Revoking::Empty));
}

#[test]
fn details_are_read_only_and_a_confirmation_is_required_before_one_revoke() {
    let mut page = page();
    let id = device().id;
    assert_eq!(page.inspect_device(id), Ok(()));
    assert_eq!(page.inspected_device(), Some(&device()));
    assert!(matches!(page.revoking, Revoking::Empty));
    page.close_inspection();
    assert!(page.inspected_device().is_none());
    assert_eq!(page.inspect_device(id), Ok(()));
    assert_eq!(page.choose_revoke(id), Ok(()));
    assert!(page.inspected_device().is_none());
    assert_eq!(page.revoking.confirmation(), Some(&device()));
    assert_eq!(
        page.inspect_device(id),
        Err(SelectionError::ConfirmationOpen)
    );
    assert_eq!(
        page.choose_revoke(id),
        Err(SelectionError::ConfirmationOpen)
    );
    let ticket = page.tickets.next_ticket().unwrap();
    let context = page.context().unwrap().clone();
    let intent = page.revoke_intent().unwrap();
    assert_eq!(page.revoking.confirm(&context, intent, ticket), Some(id));
    assert_eq!(
        page.inspect_device(id),
        Err(SelectionError::RevocationPending)
    );
    assert_eq!(
        page.choose_revoke(id),
        Err(SelectionError::RevocationPending)
    );
    assert_eq!(page.revoking.confirm(&context, intent, ticket), None);
    assert_eq!(page.revoking.confirmation(), Some(&device()));
}

#[test]
fn successful_refresh_updates_details_or_closes_a_removed_record_and_navigation_clears_it() {
    let mut page = page();
    let id = device().id;
    page.inspect_device(id).unwrap();
    page.target(page.target.clone());
    assert_eq!(page.inspected_device(), Some(&device()));
    let updated = DeviceInfo {
        scopes: vec![Scope::Read],
        ..device()
    };
    page.listing = Listing::Ready(vec![updated.clone()]);
    page.reconcile_inspection();
    assert_eq!(page.inspected_device(), Some(&updated));
    page.listing = Listing::Failed(Problem::Offline);
    page.reconcile_inspection();
    assert_eq!(page.inspected_device(), Some(&updated));
    page.listing = Listing::Ready(vec![DeviceInfo {
        revoked_at: Some(at("2026-11-12T00:00:00Z")),
        ..device()
    }]);
    page.reconcile_inspection();
    assert!(page.inspected_device().is_none());
    page.listing = Listing::Ready(vec![device()]);
    page.inspect_device(id).unwrap();
    page.listing = Listing::Ready(vec![]);
    page.reconcile_inspection();
    assert!(page.inspected_device().is_none());
    page.listing = Listing::Ready(vec![device()]);
    page.inspect_device(id).unwrap();
    page.target(Target::Online(MachineKey::from_book("other")));
    assert!(page.inspected_device().is_none());
}

#[test]
fn touch_metadata_keeps_every_grant_and_exact_local_instants_in_both_languages() {
    let offset = FixedOffset::east_opt(9 * 3600).unwrap();
    assert_eq!(DeviceMetadata::of(Lang::En, &device(), offset), DeviceMetadata {
        paired_at: "2026-11-11 14:00:00 (UTC+9)".into(),
        last_seen: "2026-11-11 15:57:00 (UTC+9)".into(),
        grants: "Read information, Run sessions, Manage machine, Receive shared settings, Access other machines".into(),
    });
    assert_eq!(
        DeviceMetadata::of(Lang::Ko, &device(), offset),
        DeviceMetadata {
            paired_at: "2026-11-11 14:00:00 (UTC+9)".into(),
            last_seen: "2026-11-11 15:57:00 (UTC+9)".into(),
            grants: "정보 읽기, 세션 실행, 머신 관리, 공유 설정 받기, 다른 머신 접근".into(),
        }
    );
    let unseen = DeviceInfo {
        last_seen: None,
        scopes: vec![],
        ..device()
    };
    for (lang, seen, grants) in [
        (Lang::En, "No connection recorded", "None"),
        (Lang::Ko, "접속 기록 없음", "없음"),
    ] {
        assert_eq!(
            DeviceMetadata::of(lang, &unseen, offset),
            DeviceMetadata {
                paired_at: "2026-11-11 14:00:00 (UTC+9)".into(),
                last_seen: seen.into(),
                grants: grants.into(),
            }
        );
    }
}
