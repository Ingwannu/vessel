//! Paseo host-runtime.test.ts:1630 and 4220: an old switch or confirmation
//! cannot settle the next one. The observed editable-book ABA is additional.
use super::*;
use crate::devices::{ConnectionEpochs, Inviting, MachineKey, Revoking, SelectionError, Target};
use chrono::{DateTime, Duration};
use vessel_proto::{DeviceId, Invitation, NodeId, PairingLink, PairingOffer, Scope};

fn node(last: u8) -> NodeId {
    format!("00000000-0000-4000-8000-{last:012}")
        .parse()
        .unwrap()
}
fn device() -> DeviceInfo {
    DeviceInfo {
        id: "00000000-0000-4000-8000-000000000001"
            .parse::<DeviceId>()
            .unwrap(),
        label: "The same device id on both machines".into(),
        created_at: DateTime::UNIX_EPOCH,
        last_seen: None,
        revoked_at: None,
        scopes: vec![Scope::Admin],
    }
}
fn admit(epochs: &mut ConnectionEpochs, node: NodeId) -> ConnectionContext {
    epochs
        .admit(MachineKey::from_book("fixture"), node)
        .unwrap()
}
fn page(context: &ConnectionContext) -> Page {
    let mut page = Page::default();
    page.target(Target::Online(context.machine().clone()));
    let _change = page.bind(Some(context.clone()));
    grant(&mut page, context);
    page.listing = Listing::Ready(vec![device()]);
    page
}
fn grant(page: &mut Page, context: &ConnectionContext) {
    let authority = page.tickets.next_ticket().unwrap();
    assert!(page.checking_authority(context, authority));
    assert!(page.authority_answer(
        context,
        authority,
        Ok(vessel_proto::DeviceSelf {
            device: None,
            scopes: vec![Scope::Admin],
        })
    ));
}
fn invitation() -> Pairing {
    let expires = DateTime::UNIX_EPOCH
        .checked_add_signed(Duration::minutes(10))
        .unwrap();
    let offer = PairingOffer {
        v: vessel_proto::OfferVersion,
        node: node(1),
        key: vessel_proto::NodePublicKey::from_bytes([4; 32]),
        direct: Vec::new(),
        relays: Vec::new(),
        invite: vessel_proto::InvitationSecret::from_bytes([7; 16]),
        expires,
    };
    Pairing::new(
        Invitation {
            link: PairingLink::encode(&offer).unwrap(),
            expires,
        },
        "public test fingerprint".into(),
    )
}

#[test]
fn replacement_clears_every_owned_surface_before_a_frame_without_resetting_tickets() {
    let mut epochs = ConnectionEpochs::default();
    let first = admit(&mut epochs, node(1));
    let second = admit(&mut epochs, node(2));
    let mut page = page(&first);
    page.inspect_bound(&first, device().id).unwrap();
    let old = page.tickets.next_ticket().unwrap();
    let mut credential = invitation();
    credential.show(DateTime::UNIX_EPOCH, old);
    assert!(credential.visible_link(DateTime::UNIX_EPOCH).is_some());
    page.inviting = Inviting::Ready(credential);
    assert_eq!(
        page.bind(Some(second.clone())),
        crate::devices::IdentityChange::Replaced
    );
    assert!(matches!(page.listing, Listing::Empty));
    assert!(matches!(page.inviting, Inviting::Empty));
    assert!(matches!(page.revoking, Revoking::Empty));
    assert!(page.inspected_device().is_none());
    assert!(page.revoke_intent().is_none());
    assert_ne!(page.tickets.next_ticket().unwrap(), old);
    grant(&mut page, &second);
    page.listing = Listing::Ready(vec![device()]);
    assert_eq!(
        page.inspect_bound(&first, device().id),
        Err(SelectionError::ConnectionChanged)
    );
    assert_eq!(
        page.choose_bound(&first, device().id),
        Err(SelectionError::ConnectionChanged)
    );
    assert_eq!(page.choose_bound(&second, device().id), Ok(()));
}

#[test]
fn an_old_confirmation_cannot_consume_or_cancel_a_new_intent_on_the_same_connection() {
    let mut epochs = ConnectionEpochs::default();
    let context = admit(&mut epochs, node(1));
    let mut page = page(&context);
    page.choose_bound(&context, device().id).unwrap();
    let first = page.revoke_intent().unwrap();
    page.cancel_revoke(&context, first);
    page.choose_bound(&context, device().id).unwrap();
    let second = page.revoke_intent().unwrap();
    assert_ne!(first, second);
    let request = page.tickets.next_ticket().unwrap();
    assert_eq!(page.confirm_revoke(&context, first, request), None);
    page.cancel_revoke(&context, first);
    assert_eq!(page.revoke_intent(), Some(second));
    assert_eq!(
        page.confirm_revoke(&context, second, request),
        Some(device().id)
    );
    assert_eq!(page.confirm_revoke(&context, second, request), None);
    page.cancel_revoke(&context, second);
    assert!(matches!(page.revoking, Revoking::Sending { .. }));
}

#[test]
fn observed_aba_and_same_node_reconnect_refuse_old_replies_even_with_current_tickets() {
    let mut epochs = ConnectionEpochs::default();
    let first = admit(&mut epochs, node(1));
    let b = admit(&mut epochs, node(2));
    let next = admit(&mut epochs, node(1));
    let reconnect = admit(&mut epochs, node(1));
    let mut page = page(&first);
    page.choose_bound(&first, device().id).unwrap();
    let first_intent = page.revoke_intent().unwrap();
    for context in [&b, &next, &reconnect] {
        let _change = page.bind(Some(context.clone()));
    }
    grant(&mut page, &reconnect);
    let read = page.tickets.next_ticket().unwrap();
    let invite = page.tickets.next_ticket().unwrap();
    let revoke = page.tickets.next_ticket().unwrap();
    page.listing = Listing::Ready(vec![device()]);
    page.choose_bound(&reconnect, device().id).unwrap();
    let intent = page.revoke_intent().unwrap();
    assert_eq!(
        page.confirm_revoke(&reconnect, intent, revoke),
        Some(device().id)
    );
    page.listing = Listing::Loading(read);
    page.inviting = Inviting::Loading(invite);
    for old in [&first, &b, &next] {
        assert!(!page.listed(old, read, Ok(vec![device()])));
        assert!(!page.invited(old, invite, Ok(invitation())));
        assert!(!page.revoked(old, revoke, Ok(())));
        assert_eq!(page.confirm_revoke(old, first_intent, revoke), None);
    }
    assert!(matches!(page.listing, Listing::Loading(_)));
    assert!(matches!(page.inviting, Inviting::Loading(_)));
    assert!(matches!(page.revoking, Revoking::Sending { .. }));
    assert!(page.listed(&reconnect, read, Ok(vec![device()])));
    assert!(page.invited(&reconnect, invite, Ok(invitation())));
    assert!(page.revoked(&reconnect, revoke, Ok(())));
}

#[test]
fn navigation_and_retirement_refuse_callbacks_without_any_new_frame_or_reply() {
    let mut epochs = ConnectionEpochs::default();
    let context = admit(&mut epochs, node(1));
    let mut page = page(&context);
    page.choose_bound(&context, device().id).unwrap();
    let intent = page.revoke_intent().unwrap();
    let request = page.tickets.next_ticket().unwrap();
    let _change = page.bind(None);
    assert_eq!(page.confirm_revoke(&context, intent, request), None);
    assert!(matches!(page.revoking, Revoking::Empty));
    assert!(!page.revoked(&context, request, Ok(())));
    page.target(Target::Hidden);
    let _change = page.bind(Some(context.clone()));
    assert!(page.context().is_none());
}

#[test]
fn even_a_retained_old_confirmation_cannot_be_attached_to_another_admission() {
    let mut epochs = ConnectionEpochs::default();
    let first = admit(&mut epochs, node(1));
    let replacement = admit(&mut epochs, node(2));
    let mut page = page(&first);
    page.choose_bound(&first, device().id).unwrap();
    let intent = page.revoke_intent().unwrap();
    let old = std::mem::take(&mut page.revoking);
    let _change = page.bind(Some(replacement.clone()));
    page.revoking = old;
    let request = page.tickets.next_ticket().unwrap();
    assert_eq!(page.confirm_revoke(&replacement, intent, request), None);
    assert_eq!(page.revoking.context(), Some(&first));
}

#[test]
fn refreshing_authority_drops_a_revealed_link_rows_details_and_confirmation() {
    let mut epochs = ConnectionEpochs::default();
    let context = admit(&mut epochs, node(1));
    let mut page = page(&context);
    page.inspect_bound(&context, device().id).unwrap();
    page.choose_bound(&context, device().id).unwrap();
    let reveal = page.tickets.next_ticket().unwrap();
    let mut credential = invitation();
    credential.show(DateTime::UNIX_EPOCH, reveal);
    assert!(credential.visible_link(DateTime::UNIX_EPOCH).is_some());
    page.inviting = Inviting::Ready(credential);
    let query = page.tickets.next_ticket().unwrap();
    assert!(page.checking_authority(&context, query));
    assert!(matches!(page.authority, crate::devices::Authority::Loading(t) if t == query));
    assert!(matches!(page.inviting, Inviting::Empty));
    assert!(matches!(page.listing, Listing::Empty));
    assert!(matches!(page.revoking, Revoking::Empty));
    assert!(page.inspected_device().is_none());
    assert!(page.revoke_intent().is_none());
    assert_ne!(page.tickets.next_ticket().unwrap(), query);
}
