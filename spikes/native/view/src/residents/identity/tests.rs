//! Lossless render identity, without another counter or admission mechanism.
use super::*;

fn token() -> ResidentToken {
    ResidentToken {
        registry: RegistryId("11111111-1111-4111-8111-111111111111".parse().unwrap()),
        address: SessionAddress {
            node: "22222222-2222-4222-8222-222222222222".parse().unwrap(),
            session: "33333333-3333-4333-8333-333333333333".parse().unwrap(),
        },
        epoch: ResidentEpoch(4),
    }
}

#[test]
fn render_identity_contains_all_canonical_components_without_debug_or_hash() {
    let token = token();
    assert_eq!(
        token.to_string(),
        "resident/11111111-1111-4111-8111-111111111111/22222222-2222-4222-8222-222222222222/33333333-3333-4333-8333-333333333333/4"
    );
}

#[test]
fn registry_node_session_and_full_width_epoch_each_separate_a_render_key() {
    let original = token();
    let mut registry = original;
    registry.registry = RegistryId("44444444-4444-4444-8444-444444444444".parse().unwrap());
    let mut node = original;
    node.address.node = "55555555-5555-4555-8555-555555555555".parse().unwrap();
    let mut session = original;
    session.address.session = "66666666-6666-4666-8666-666666666666".parse().unwrap();
    let mut epoch = original;
    epoch.epoch = ResidentEpoch(u64::MAX);
    let keys: BTreeSet<_> = [original, registry, node, session, epoch]
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(keys.len(), 5);
    assert!(epoch.to_string().ends_with("/18446744073709551615"));
}
