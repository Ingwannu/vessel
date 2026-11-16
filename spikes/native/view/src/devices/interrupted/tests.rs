//! Interrupted writes remain unknown across reconnect, never across identity.
use super::*;
use crate::devices::{ConnectionEpochs, Listing, MachineKey, Target};
use vessel_proto::{DeviceId, DeviceInfo, DeviceSelf, NodeId, Scope};
fn setup() -> (Page, ConnectionContext, crate::Machine) {
    let mut epochs = ConnectionEpochs::default();
    let context = epochs
        .admit(MachineKey::from_book("fixture"), NodeId::random())
        .unwrap();
    let mut page = Page::default();
    page.target(Target::Online(context.machine().clone()));
    let _change = page.bind(Some(context.clone()));
    let ticket = page.tickets.next_ticket().unwrap();
    assert!(page.checking_authority(&context, ticket));
    assert!(page.authority_answer(
        &context,
        ticket,
        Ok(DeviceSelf {
            device: None,
            scopes: vec![Scope::Admin]
        })
    ));
    let mut machine = crate::Machine::unreachable("fixture".into(), "session read refused");
    machine.node = Some(context.node());
    machine.connection_epoch = Some(context.epoch());
    (page, context, machine)
}
fn confirm(page: &mut Page, context: &ConnectionContext) {
    let device = DeviceInfo {
        id: DeviceId::random(),
        label: "Public device".into(),
        created_at: chrono::DateTime::UNIX_EPOCH,
        last_seen: None,
        scopes: vec![Scope::Admin],
        revoked_at: None,
    };
    let id = device.id;
    page.listing = Listing::Ready(vec![device]);
    page.choose_bound(context, id).unwrap();
}
#[test]
fn confirmation_without_dispatch_creates_no_uncertain_operation() {
    let (mut page, context, machine) = setup();
    confirm(&mut page, &context);
    let _change = page.bind(None);
    assert_eq!(page.unconfirmed(&machine), None);
}
#[test]
fn invitation_revocation_and_concurrent_dispatch_have_distinct_bounded_notices() {
    for expected in [
        Unconfirmed::Invitation,
        Unconfirmed::Revocation,
        Unconfirmed::Both,
    ] {
        let (mut page, context, mut machine) = setup();
        if expected != Unconfirmed::Invitation {
            confirm(&mut page, &context);
            let intent = page.revoke_intent().unwrap();
            let ticket = page.tickets.next_ticket().unwrap();
            assert!(page.confirm_revoke(&context, intent, ticket).is_some());
        }
        if expected != Unconfirmed::Revocation {
            page.inviting = Inviting::Loading(page.tickets.next_ticket().unwrap());
        }
        let _change = page.bind(None);
        assert_eq!(page.unconfirmed(&machine), Some(expected));
        assert!(matches!(page.inviting, Inviting::Empty));
        assert!(matches!(page.revoking, Revoking::Empty));
        assert!(!page.authority().may_manage());
        machine.connection_epoch = None;
        assert_eq!(page.unconfirmed(&machine), Some(expected));
        machine.node = Some(NodeId::random());
        assert_eq!(page.unconfirmed(&machine), None);
        machine.node = Some(context.node());
        machine.name = "another-selected-entry".into();
        assert_eq!(page.unconfirmed(&machine), None);
    }
}
#[test]
fn a_read_does_not_become_uncertain_on_navigation() {
    let (mut page, context, machine) = setup();
    page.listing = Listing::Loading(page.tickets.next_ticket().unwrap());
    page.target(Target::Hidden);
    assert_eq!(page.unconfirmed(&machine), None);
    let _change = page.bind(Some(context));
    page.interrupted();
    assert_eq!(page.unconfirmed(&machine), None);
}

#[test]
fn permission_refresh_cannot_silently_forget_an_outstanding_write() {
    for invite in [false, true] {
        let (mut page, context, machine) = setup();
        let ticket = page.tickets.next_ticket().unwrap();
        if invite {
            page.inviting = Inviting::Loading(ticket);
        } else {
            confirm(&mut page, &context);
            let intent = page.revoke_intent().unwrap();
            assert!(page.confirm_revoke(&context, intent, ticket).is_some());
        }
        assert!(page.mutation_pending());
        let query = page.tickets.next_ticket().unwrap();
        assert!(!page.checking_authority(&context, query));
        assert!(!page.cannot_check_authority(&context, crate::devices::Problem::Exhausted));
        assert!(page.mutation_pending());
        assert!(page.authority().may_manage());
        let _change = page.bind(None);
        assert_eq!(
            page.unconfirmed(&machine),
            Some(if invite {
                Unconfirmed::Invitation
            } else {
                Unconfirmed::Revocation
            })
        );
    }
}

#[test]
fn a_local_write_deadline_settles_only_its_current_ticket_without_losing_live_authority() {
    let (mut page, context, machine) = setup();
    let ticket = page.tickets.next_ticket().unwrap();
    page.inviting = Inviting::Loading(ticket);
    let wrong = page.tickets.next_ticket().unwrap();
    assert!(!page.cannot_confirm(&context, wrong));
    assert!(matches!(page.inviting, Inviting::Loading(current) if current == ticket));
    assert!(page.cannot_confirm(&context, ticket));
    assert!(matches!(page.inviting, Inviting::Empty));
    assert!(page.authority().may_manage());
    assert_eq!(page.context(), Some(&context));
    assert_eq!(page.unconfirmed(&machine), Some(Unconfirmed::Invitation));
    assert!(!page.cannot_confirm(&context, ticket));
    confirm(&mut page, &context);
    let intent = page.revoke_intent().unwrap();
    let ticket = page.tickets.next_ticket().unwrap();
    assert!(page.confirm_revoke(&context, intent, ticket).is_some());
    assert!(page.cannot_confirm(&context, ticket));
    assert_eq!(page.unconfirmed(&machine), Some(Unconfirmed::Both));
}
