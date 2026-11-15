//! The integrated device.self contract, independent of session Read or names.
use super::*;
use crate::devices::{ConnectionEpochs, MachineKey, Target};
use vessel_proto::{ErrorCode, NodeId};

fn admitted(epochs: &mut ConnectionEpochs, node: NodeId) -> ConnectionContext {
    epochs
        .admit(MachineKey::from_book("fixture"), node)
        .unwrap()
}
fn selected(context: &ConnectionContext) -> Page {
    let mut page = Page::default();
    page.target(Target::Online(context.machine().clone()));
    let _change = page.bind(Some(context.clone()));
    page
}
fn report(scopes: Vec<Scope>, device: Option<DeviceId>) -> DeviceSelf {
    DeviceSelf { device, scopes }
}
fn answer(page: &mut Page, context: &ConnectionContext, facts: DeviceSelf) {
    let ticket = page.tickets.next_ticket().unwrap();
    assert!(page.checking_authority(context, ticket));
    assert!(page.authority_answer(context, ticket, Ok(facts)));
}

#[test]
fn admin_without_session_read_can_manage_but_other_independent_scopes_cannot() {
    let mut epochs = ConnectionEpochs::default();
    let context = admitted(&mut epochs, NodeId::random());
    let mut page = selected(&context);
    answer(&mut page, &context, report(vec![Scope::Admin], None));
    assert!(page.authority.may_manage());
    assert_eq!(page.authority.device(), None);
    for scopes in [
        vec![],
        vec![Scope::Read],
        vec![Scope::Operate],
        vec![Scope::Sync],
        vec![Scope::Relay],
        vec![Scope::Read, Scope::Operate, Scope::Sync, Scope::Relay],
    ] {
        answer(&mut page, &context, report(scopes, None));
        assert!(!page.authority.may_manage());
        assert!(matches!(page.authority, Authority::Known(_)));
    }
}

#[test]
fn unknown_failed_and_empty_grants_remain_distinct_without_default_authority() {
    let mut epochs = ConnectionEpochs::default();
    let context = admitted(&mut epochs, NodeId::random());
    let mut page = selected(&context);
    assert!(matches!(page.authority, Authority::Unchecked));
    assert!(!page.authority.may_manage());
    for code in [
        ErrorCode::Unsupported,
        ErrorCode::Unavailable,
        ErrorCode::Forbidden,
    ] {
        let ticket = page.tickets.next_ticket().unwrap();
        assert!(page.checking_authority(&context, ticket));
        assert!(!page.authority.may_manage());
        assert!(page.authority_answer(&context, ticket, Err(Problem::Refused(code))));
        assert!(matches!(page.authority, Authority::Failed(Problem::Refused(c)) if c == code));
        assert!(!page.authority.may_manage());
    }
    answer(&mut page, &context, report(vec![], None));
    assert!(matches!(page.authority, Authority::Known(_)));
}

#[test]
fn a_refresh_hides_the_reported_caller_and_rejects_the_previous_read_ticket() {
    let mut epochs = ConnectionEpochs::default();
    let context = admitted(&mut epochs, NodeId::random());
    let mut page = selected(&context);
    let device = DeviceId::random();
    answer(
        &mut page,
        &context,
        report(vec![Scope::Admin], Some(device)),
    );
    assert_eq!(page.authority.device(), Some(device));
    let old = page.tickets.next_ticket().unwrap();
    assert!(page.checking_authority(&context, old));
    let current = page.tickets.next_ticket().unwrap();
    assert!(page.checking_authority(&context, current));
    assert_eq!(page.authority.device(), None);
    assert!(!page.authority.may_manage());
    assert!(!page.authority_answer(&context, old, Ok(report(vec![Scope::Admin], Some(device)))));
    assert!(matches!(page.authority, Authority::Loading(t) if t == current));
    assert!(page.authority_answer(&context, current, Ok(report(vec![], None))));
    assert_eq!(page.authority.device(), None);
}

#[test]
fn a_same_node_reconnect_cannot_inherit_grants_or_accept_a_previous_reply() {
    let mut epochs = ConnectionEpochs::default();
    let node = NodeId::random();
    let old = admitted(&mut epochs, node);
    let current = admitted(&mut epochs, node);
    let mut page = selected(&old);
    let ticket = page.tickets.next_ticket().unwrap();
    assert!(page.checking_authority(&old, ticket));
    let _change = page.bind(Some(current.clone()));
    assert!(matches!(page.authority, Authority::Unchecked));
    assert!(!page.authority_answer(&old, ticket, Ok(report(vec![Scope::Admin], None))));
    let current_ticket = page.tickets.next_ticket().unwrap();
    assert!(page.checking_authority(&current, current_ticket));
    assert!(!page.checking_authority(&old, ticket));
    assert!(matches!(page.authority, Authority::Loading(t) if t == current_ticket));
}

#[test]
fn duplicate_or_overlong_facts_are_refused_instead_of_truncated_to_admin() {
    let mut epochs = ConnectionEpochs::default();
    let context = admitted(&mut epochs, NodeId::random());
    let mut page = selected(&context);
    for scopes in [
        vec![Scope::Admin, Scope::Admin],
        vec![Scope::Admin; Scope::ALL.len() + 1],
    ] {
        answer(
            &mut page,
            &context,
            report(scopes, Some(DeviceId::random())),
        );
        assert!(matches!(
            page.authority,
            Authority::Failed(Problem::InvalidReply)
        ));
        assert!(!page.authority.may_manage());
        assert_eq!(page.authority.device(), None);
    }
    answer(&mut page, &context, report(Scope::ALL.to_vec(), None));
    assert!(page.authority.may_manage());
}
