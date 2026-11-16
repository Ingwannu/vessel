//! Startup and correlated refusals cannot become authoritative empty ownership.
//! Successful cross-App guard installation requires the owner's real actor fixture.
use super::*;
use vessel_app_view::connection::{ConnectionEpochs, MachineKey};

#[test]
fn registry_cache_startup_and_an_unguarded_refusal_never_prove_legacy_ownership() {
    let node = NodeId::random();
    let session = SessionId::random();
    let context = ConnectionEpochs::default()
        .admit(MachineKey::from_book("fixture"), node)
        .unwrap();
    let mut cache = RegistryCache::default();
    assert_eq!(
        cache.session_workspace(node, session),
        Err(LookupProblem::NotHydrated)
    );
    cache.reconcile(std::slice::from_ref(&context));
    let request = cache.next_read().unwrap();
    assert!(cache.next_read().is_none());
    cache.receive(request.refused());
    assert_eq!(cache.phase(&context), Some(RegistryPhase::Refused));
    assert_eq!(
        cache.session_workspace(node, session),
        Err(LookupProblem::Unavailable)
    );
    assert!(
        cache.next_read().is_none(),
        "no immediate refusal retry loop"
    );
}

#[test]
fn registry_cache_forgetting_a_host_drops_its_queued_ticket_and_releases_the_name() {
    let node = NodeId::random();
    let mut epochs = ConnectionEpochs::default();
    let context = epochs
        .admit(MachineKey::from_book("fixture"), node)
        .unwrap();
    let mut cache = RegistryCache::default();
    cache.reconcile(std::slice::from_ref(&context));
    let old = cache.next_read().unwrap();
    cache.forget(&Name::parse("fixture").unwrap());
    cache.receive(old.refused());
    assert_eq!(cache.phase(&context), None);
    assert!(cache.next_read().is_none());
    let replacement = epochs
        .admit(MachineKey::from_book("fixture"), node)
        .unwrap();
    cache.reconcile(std::slice::from_ref(&replacement));
    let fresh = cache.next_read().unwrap();
    assert_eq!(fresh.key.context(), &replacement);
    assert_ne!(fresh.key.context(), &context);
}

#[cfg(not(target_arch = "wasm32"))]
mod actor_cases;
