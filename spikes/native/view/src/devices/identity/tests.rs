//! Stable aliases and repeated node ids cannot revive old user intent.
use super::*;
use crate::connection::{ConnectionEpochs, MachineKey};
use vessel_proto::NodeId;

fn node(last: u8) -> NodeId {
    let text = format!("00000000-0000-4000-8000-{last:012}");
    text.parse().unwrap()
}

fn admit(epochs: &mut ConnectionEpochs, name: &str, node: NodeId) -> ConnectionContext {
    epochs.admit(MachineKey::from_book(name), node).unwrap()
}

#[test]
fn the_same_alias_does_not_accept_a_different_authenticated_node() {
    let mut epochs = ConnectionEpochs::default();
    let first = admit(&mut epochs, "fixture", node(1));
    let replacement = admit(&mut epochs, "fixture", node(2));
    let mut selected = DeviceIdentity::default();
    assert_eq!(
        selected.observe(Some(first.clone())),
        IdentityChange::Replaced
    );
    assert_eq!(selected.check(&first), Ok(()));
    assert_eq!(
        selected.observe(Some(replacement.clone())),
        IdentityChange::Replaced
    );
    assert_eq!(first.machine(), replacement.machine());
    assert_ne!(first.node(), replacement.node());
    assert_eq!(selected.check(&first), Err(StaleContext));
    assert_eq!(selected.check(&replacement), Ok(()));
}

#[test]
fn reconnecting_to_the_same_node_retires_the_previous_admission() {
    // Paseo host-runtime.test.ts:751 replaces the client after re-pairing,
    // even when the host's stable identifier and connection name stay equal.
    let mut epochs = ConnectionEpochs::default();
    let first = admit(&mut epochs, "fixture", node(1));
    let reconnected = admit(&mut epochs, "fixture", node(1));
    let mut selected = DeviceIdentity::default();
    let _first = selected.observe(Some(first.clone()));
    assert_eq!(
        selected.observe(Some(reconnected.clone())),
        IdentityChange::Replaced
    );
    assert_eq!(first.node(), reconnected.node());
    assert_ne!(first.epoch(), reconnected.epoch());
    assert_eq!(selected.check(&first), Err(StaleContext));
    assert_eq!(selected.check(&reconnected), Ok(()));
}

#[test]
fn observed_a_to_b_to_a_never_restores_an_older_callback_or_answer() {
    // Paseo host-runtime.test.ts:1630 ignores the earlier switch's error
    // after the replacement is online. The additional ABA case is Vessel's
    // editable-book boundary, with the same physical id returning later.
    let mut epochs = ConnectionEpochs::default();
    let first_a = admit(&mut epochs, "fixture", node(1));
    let b = admit(&mut epochs, "fixture", node(2));
    let next_a = admit(&mut epochs, "fixture", node(1));
    let mut selected = DeviceIdentity::default();
    for current in [&first_a, &b, &next_a] {
        assert_eq!(
            selected.observe(Some(current.clone())),
            IdentityChange::Replaced
        );
    }
    assert_eq!(selected.check(&first_a), Err(StaleContext));
    assert_eq!(selected.check(&b), Err(StaleContext));
    assert_eq!(selected.check(&next_a), Ok(()));
}

#[test]
fn unchanged_facts_keep_identity_and_retirement_cannot_accept_a_late_answer() {
    let mut epochs = ConnectionEpochs::default();
    let current = admit(&mut epochs, "fixture", node(1));
    let mut selected = DeviceIdentity::default();
    assert_eq!(selected.check(&current), Err(StaleContext));
    let _initial = selected.observe(Some(current.clone()));
    assert_eq!(
        selected.observe(Some(current.clone())),
        IdentityChange::Retained
    );
    assert_eq!(selected.current(), Some(&current));
    assert_eq!(selected.check(&current), Ok(()));
    assert_eq!(selected.observe(None), IdentityChange::Replaced);
    assert_eq!(selected.current(), None);
    assert_eq!(selected.check(&current), Err(StaleContext));
    assert_eq!(selected.observe(None), IdentityChange::Retained);
}

#[test]
fn a_renamed_book_entry_does_not_reuse_the_old_control_identity() {
    let mut epochs = ConnectionEpochs::default();
    let original = admit(&mut epochs, "original", node(1));
    let renamed = admit(&mut epochs, "renamed", node(1));
    let mut selected = DeviceIdentity::default();
    let _initial = selected.observe(Some(original.clone()));
    assert_eq!(
        selected.observe(Some(renamed.clone())),
        IdentityChange::Replaced
    );
    assert_eq!(original.node(), renamed.node());
    assert_ne!(original.machine(), renamed.machine());
    assert_eq!(selected.check(&original), Err(StaleContext));
    assert_eq!(selected.check(&renamed), Ok(()));
}
