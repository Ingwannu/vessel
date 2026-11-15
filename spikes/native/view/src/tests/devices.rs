//! The page's `data/devices.test.ts`, and the pairing link's showing.

mod phrases;
mod review;
mod rows;

use chrono::{DateTime, Duration, Utc};
use vessel_proto::DeviceInfo;

use crate::Lang;
use crate::devices::{LinkShown, device_line, invite_label, paired_devices};

fn at(text: &str) -> DateTime<Utc> {
    text.parse().unwrap()
}

fn phone() -> DeviceInfo {
    serde_json::from_value(serde_json::json!({
        "id": "00000000-0000-4000-8000-000000000001",
        "label": "phone",
        "created_at": "2026-11-11T05:00:00Z",
        "last_seen": "2026-11-11T06:57:00Z",
        "scopes": ["read", "operate"],
    }))
    .unwrap()
}

fn selected(device: DeviceInfo, tickets: &mut crate::devices::Tickets) -> crate::devices::Revoking {
    use crate::devices::{ConnectionEpochs, Listing, MachineKey, Page, Target};
    let mut epochs = ConnectionEpochs::default();
    let context = epochs
        .admit(
            MachineKey::from_book("fixture"),
            "00000000-0000-4000-8000-000000000099".parse().unwrap(),
        )
        .unwrap();
    let mut page = Page::default();
    page.tickets = std::mem::take(tickets);
    page.target(Target::Online(context.machine().clone()));
    let _change = page.bind(Some(context.clone()));
    let id = device.id;
    page.listing = Listing::Ready(vec![device]);
    page.choose_bound(&context, id).unwrap();
    *tickets = std::mem::take(&mut page.tickets);
    page.revoking
}

#[test]
fn says_what_a_device_may_do_and_when_it_was_seen_and_paired() {
    let now = at("2026-11-11T07:00:00Z");
    assert_eq!(
        device_line(Lang::En, &phone(), now),
        ["Can read, run sessions", "seen 3m ago", "paired 2h ago"]
    );
    let unseen = DeviceInfo {
        last_seen: None,
        ..phone()
    };
    assert_eq!(device_line(Lang::En, &unseen, now)[1], "never seen");
}

#[test]
fn lists_only_the_devices_that_still_have_access() {
    let revoked = DeviceInfo {
        id: "00000000-0000-4000-8000-000000000002".parse().unwrap(),
        revoked_at: Some(at("2026-11-11T06:00:00Z")),
        ..phone()
    };
    let devices = [phone(), revoked];
    assert_eq!(paired_devices(&devices), [&devices[0]]);
}

#[test]
fn a_new_devices_label_is_trimmed_and_none_when_empty() {
    assert_eq!(invite_label("  tablet "), Some("tablet".to_owned()));
    assert_eq!(invite_label("   "), None);
}

/// The link is a credential: shown when asked, hidden again after 30 s.
#[test]
fn a_shown_link_hides_itself_after_thirty_seconds() {
    let shown = at("2026-11-11T07:00:00Z");
    assert!(!LinkShown::Hidden.visible(shown));
    assert_eq!(LinkShown::Hidden.hides_at(), None);
    let since = LinkShown::Since(shown);
    assert!(since.visible(shown + Duration::seconds(29)));
    assert!(!since.visible(shown + Duration::seconds(30)));
    assert_eq!(since.hides_at(), Some(shown + Duration::seconds(30)));
}

fn reveal_key() -> crate::devices::Ticket {
    crate::devices::Tickets::default().next_ticket().unwrap()
}

fn pairing(expires: DateTime<Utc>) -> crate::devices::Pairing {
    let offer = vessel_proto::PairingOffer {
        v: vessel_proto::OfferVersion,
        node: "00000000-0000-4000-8000-000000000099".parse().unwrap(),
        key: vessel_proto::NodePublicKey::from_bytes([4; 32]),
        direct: Vec::new(),
        relays: Vec::new(),
        invite: vessel_proto::InvitationSecret::from_bytes([7; 16]),
        expires,
    };
    crate::devices::Pairing::new(
        vessel_proto::Invitation {
            link: vessel_proto::PairingLink::encode(&offer).unwrap(),
            expires,
        },
        "public fingerprint".to_owned(),
    )
}

#[test]
fn a_credential_is_initially_masked_and_cannot_be_revealed_after_expiry() {
    let now = at("2026-11-11T07:00:00Z");
    let mut pairing = pairing(now + Duration::seconds(10));
    assert!(pairing.visible_link(now).is_none());
    assert_eq!(pairing.hides_at(), None);
    assert_eq!(pairing.fingerprint(), "public fingerprint");
    pairing.show(now, reveal_key());
    assert!(pairing.visible_link(now).is_some());
    assert_eq!(pairing.hides_at(), Some(now + Duration::seconds(10)));
    assert!(pairing.visible_link(now + Duration::seconds(10)).is_none());
    pairing.hide();
    pairing.show(now + Duration::seconds(10), reveal_key());
    assert!(pairing.visible_link(now + Duration::seconds(10)).is_none());
}

#[test]
fn copy_focus_loss_and_navigation_hide_a_link_until_explicitly_revealed() {
    let now = at("2026-11-11T07:00:00Z");
    let mut pairing = pairing(now + Duration::minutes(10));
    pairing.show(now, reveal_key());
    assert!(pairing.visible_link(now + Duration::seconds(29)).is_some());
    assert!(pairing.visible_link(now + Duration::seconds(30)).is_none());
    pairing.show(now, reveal_key());
    pairing.hide();
    assert!(pairing.visible_link(now).is_none());
    pairing.show(now + Duration::seconds(31), reveal_key());
    assert!(pairing.visible_link(now + Duration::seconds(31)).is_some());
    let debug = format!("{pairing:?}");
    // Check without ever putting the credential in assertion output.
    let raw = pairing
        .visible_link(now + Duration::seconds(31))
        .unwrap()
        .expose()
        .to_owned();
    assert!(!debug.contains(&raw));
}

#[test]
fn late_reads_and_invitations_cannot_restore_an_old_machine_or_credential() {
    use crate::devices::{Inviting, Listing, Problem, Tickets};
    let mut tickets = Tickets::default();
    let old = tickets.next_ticket().unwrap();
    let new = tickets.next_ticket().unwrap();
    let mut list = Listing::Loading(new);
    list.answer(old, Ok(vec![phone()]));
    assert!(matches!(list, Listing::Loading(_)));
    list.answer(new, Ok(Vec::new()));
    assert!(matches!(list, Listing::Ready(devices) if devices.is_empty()));
    let mut inviting = Inviting::Loading(new);
    assert!(!inviting.answer(old, Ok(pairing(at("2026-11-11T08:00:00Z")))));
    assert!(matches!(inviting, Inviting::Loading(_)));
    assert!(inviting.answer(new, Err(Problem::Offline)));
    assert!(matches!(inviting, Inviting::Failed(Problem::Offline)));
    // Leaving the machine clears pending state, so its late response is dropped.
    let mut inviting = Inviting::Empty;
    assert!(!inviting.answer(new, Ok(pairing(at("2026-11-11T08:00:00Z")))));
    assert!(matches!(inviting, Inviting::Empty));
}

#[test]
fn revocation_requires_confirmation_and_accepts_only_its_own_answer() {
    use crate::devices::{Problem, Revoking, Tickets};
    let mut tickets = Tickets::default();
    let old = tickets.next_ticket().unwrap();
    let ticket = tickets.next_ticket().unwrap();
    let chosen = selected(phone(), &mut tickets);
    let context = chosen.context().unwrap().clone();
    let intent = chosen.intent().unwrap();
    let mut revoking = Revoking::Empty;
    assert_eq!(revoking.confirm(&context, intent, ticket), None);
    revoking = chosen;
    assert_eq!(
        revoking.confirmation().map(|device| device.label.as_str()),
        Some("phone")
    );
    assert_eq!(revoking.confirm(&context, intent, ticket), Some(phone().id));
    assert_eq!(
        revoking.confirmation().map(|device| device.label.as_str()),
        Some("phone")
    );
    assert_eq!(revoking.confirm(&context, intent, ticket), None);
    assert!(!revoking.answer(old, Ok(())));
    assert!(revoking.answer(ticket, Err(Problem::Offline)));
    assert!(matches!(revoking, Revoking::Failed(Problem::Offline)));
    revoking = selected(phone(), &mut tickets);
    let intent = revoking.intent().unwrap();
    let next = tickets.next_ticket().unwrap();
    assert_eq!(revoking.confirm(&context, intent, next), Some(phone().id));
    assert!(!revoking.answer(ticket, Ok(())));
    assert!(revoking.answer(next, Ok(())));
    assert!(matches!(revoking, Revoking::Empty));
    assert!(revoking.confirmation().is_none());
}

#[test]
fn changing_machine_or_leaving_page_drops_all_credentials_and_pending_work() {
    use crate::devices::{Inviting, Listing, MachineKey, Page, Revoking, Target};
    let mut page = Page::default();
    let a = Target::Online(MachineKey::from_book("laptop"));
    page.target(a.clone());
    let old = page.tickets.next_ticket().unwrap();
    page.listing = Listing::Loading(old);
    page.inviting = Inviting::Ready(pairing(at("2026-11-11T08:00:00Z")));
    page.revoking = selected(phone(), &mut page.tickets);
    // Same target preserves state; another connection or leaving drops it.
    page.target(a.clone());
    assert!(matches!(page.inviting, Inviting::Ready(_)));
    page.target(Target::Offline(MachineKey::from_book("laptop")));
    assert!(matches!(page.listing, Listing::Empty));
    assert!(matches!(page.inviting, Inviting::Empty));
    assert!(matches!(page.revoking, Revoking::Empty));
    page.target(a);
    let new = page.tickets.next_ticket().unwrap();
    assert_ne!(old, new);
    page.inviting = Inviting::Loading(new);
    page.target(Target::Hidden);
    assert!(
        !page
            .inviting
            .answer(new, Ok(pairing(at("2026-11-11T08:00:00Z"))))
    );
    assert!(matches!(page.inviting, Inviting::Empty));
}

#[test]
fn hidden_and_expired_credentials_have_no_qr_matrix() {
    let now = at("2026-11-11T07:00:00Z");
    let mut pairing = pairing(now + Duration::minutes(10));
    assert!(pairing.matrix(now).unwrap().is_none());
    pairing.show(now, reveal_key());
    let matrix = pairing.matrix(now).unwrap().unwrap();
    assert_eq!(format!("{matrix:?}"), "QrMatrix(<redacted>)");
    assert!(
        pairing
            .matrix(now + Duration::seconds(30))
            .unwrap()
            .is_none()
    );
    pairing.hide();
    assert!(pairing.matrix(now).unwrap().is_none());
}

#[test]
fn qr_modules_match_the_iso_annex_i_known_matrix() {
    // ISO/IEC 18004 Annex I, version 1, medium correction, "01234567".
    let matrix = crate::devices::qr::Matrix::encode("01234567").unwrap();
    let expected = [
        "#######..#.##.#######",
        "#.....#..####.#.....#",
        "#.###.#.#.....#.###.#",
        "#.###.#.##....#.###.#",
        "#.###.#.#.###.#.###.#",
        "#.....#.#...#.#.....#",
        "#######.#.#.#.#######",
        "........#..##........",
        "#.#####..#..#.#####..",
        "...#.#.##.#.#..#.##..",
        "..#...##.#.#.#..#####",
        "....#....#.....####..",
        "...######..#.#..#....",
        "........#.#####..##..",
        "#######..##.#.##.....",
        "#.....#.#.#####...#.#",
        "#.###.#.#...#..#.##..",
        "#.###.#.##..#..#.....",
        "#.###.#.#.##.#..#.#..",
        "#.....#........##.##.",
        "#######.####.#..#.#..",
    ];
    let dark: std::collections::BTreeSet<_> = matrix.dark().collect();
    assert_eq!(matrix.side().get(), 21);
    for (y, row) in expected.iter().enumerate() {
        for (x, value) in row.chars().enumerate() {
            assert_eq!(
                dark.contains(&(u16::try_from(x).unwrap(), u16::try_from(y).unwrap())),
                value == '#'
            );
        }
    }
}

#[test]
fn wall_clock_rollback_does_not_extend_the_real_elapsed_display_limit() {
    let now = at("2026-11-11T07:00:00Z");
    let mut pairing = pairing(now + Duration::minutes(10));
    pairing.show(now, reveal_key());
    let deadline = pairing.display_limit().unwrap();
    assert!(pairing.visible_link(now - Duration::seconds(1)).is_none());
    pairing.show(now + Duration::seconds(1), reveal_key());
    pairing.hide_limit(deadline);
    assert!(pairing.visible_link(now + Duration::seconds(1)).is_some());
    pairing.hide_limit(pairing.display_limit().unwrap());
    assert!(pairing.visible_link(now + Duration::seconds(1)).is_none());
}

#[test]
fn a_qr_encoding_failure_contains_no_source_text() {
    let too_long = "private-invitation-text".repeat(1000);
    let error = crate::devices::qr::Matrix::encode(&too_long).unwrap_err();
    assert!(!format!("{error:?}").contains("private-invitation-text"));
    assert!(!error.to_string().contains("private-invitation-text"));
}

#[test]
fn an_old_timer_cannot_hide_a_new_reveal_with_the_same_deadline() {
    let now = at("2026-11-11T07:00:00Z");
    let mut tickets = crate::devices::Tickets::default();
    let mut first = pairing(now + Duration::minutes(10));
    first.show(now, tickets.next_ticket().unwrap());
    let old = first.display_limit().unwrap();
    first.show(now, tickets.next_ticket().unwrap());
    first.hide_limit(old);
    assert!(first.visible_link(now).is_some());
    let mut replacement = pairing(now + Duration::minutes(10));
    replacement.show(now, tickets.next_ticket().unwrap());
    assert_eq!(old.deadline(), replacement.hides_at().unwrap());
    replacement.hide_limit(old);
    assert!(replacement.visible_link(now).is_some());
    replacement.hide_limit(replacement.display_limit().unwrap());
    assert!(replacement.visible_link(now).is_none());
}

#[test]
fn an_invitation_needs_a_route_before_it_can_be_shown_as_success() {
    use crate::devices::{Problem, invitation_route};
    let mut offer = vessel_proto::PairingOffer {
        v: vessel_proto::OfferVersion,
        node: "00000000-0000-4000-8000-000000000099".parse().unwrap(),
        key: vessel_proto::NodePublicKey::from_bytes([4; 32]),
        direct: Vec::new(),
        relays: Vec::new(),
        invite: vessel_proto::InvitationSecret::from_bytes([7; 16]),
        expires: at("2026-11-11T08:00:00Z"),
    };
    assert_eq!(invitation_route(&offer), Err(Problem::NoRoute));
    offer
        .direct
        .push("wss://direct.invalid/node".parse().unwrap());
    assert_eq!(invitation_route(&offer), Ok(()));
    offer.direct.clear();
    offer
        .relays
        .push("wss://relay.invalid/vessel".parse().unwrap());
    assert_eq!(invitation_route(&offer), Ok(()));
}
