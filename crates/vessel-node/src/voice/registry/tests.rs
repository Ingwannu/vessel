use super::*;
use vessel_proto::voice::VoiceFailure;

fn start(session: SessionId) -> StartVoice {
    StartVoice {
        id: VoiceId::random(),
        session,
    }
}
fn unknown_stop(id: VoiceId) -> VoiceStopTarget {
    VoiceStopTarget::Starting { id }
}

#[test]
fn connection_and_session_exclusion_precede_any_provider_or_reload_work() {
    let mut registry = Registry::default();
    let connection = ConnectionLife::default();
    let sibling = connection.clone();
    let foreign = ConnectionLife::default();
    let request = start(SessionId::random());
    let attempt = registry.reserve(&connection, request).unwrap();
    assert!(matches!(
        registry.reserve(&sibling, start(SessionId::random())),
        Err(Refusal::Busy)
    ));
    assert!(matches!(
        registry.reserve(&foreign, start(request.session)),
        Err(Refusal::Busy)
    ));
    assert!(matches!(
        registry.authorize(&connection.owner(), attempt.ready.lease),
        Err(Refusal::Stopped)
    ));
    assert!(registry.active_for_session(request.session).is_none());
    registry.activate(attempt.ready.lease).unwrap();
    assert_eq!(
        registry
            .authorize(&sibling.owner(), attempt.ready.lease)
            .unwrap()
            .ready,
        attempt.ready
    );
    assert!(matches!(
        registry.authorize(&foreign.owner(), attempt.ready.lease),
        Err(Refusal::Owner)
    ));
    assert_eq!(
        registry.active_for_session(request.session).unwrap().ready,
        attempt.ready
    );
    assert!(registry.active_for_session(SessionId::random()).is_none());
}

#[test]
fn stop_can_arrive_before_the_start_handler_reserves_its_nonce() {
    let mut registry = Registry::default();
    let connection = ConnectionLife::default();
    let request = start(SessionId::random());
    assert!(matches!(
        registry.stop(&connection, unknown_stop(request.id)),
        Ok(Stop::Complete)
    ));
    assert!(matches!(
        registry.reserve(&connection, request),
        Err(Refusal::Stopped)
    ));
    assert!(matches!(
        registry.stop(&connection, unknown_stop(request.id)),
        Ok(Stop::Complete)
    ));
    assert_eq!(registry.cancelled_starts.len(), 1);
    // The marker has no authority over another connection's unrelated attempt.
    let other = ConnectionLife::default();
    assert!(registry.reserve(&other, request).is_ok());
    assert!(!other.closed().is_cancelled());
}

#[tokio::test]
async fn a_held_start_reply_cannot_publish_ready_after_stop_or_hide_uncertain_restore() {
    use std::sync::{Arc, Mutex};
    use tokio::sync::oneshot;
    let registry = Arc::new(Mutex::new(Registry::default()));
    let connection = ConnectionLife::default();
    let request = start(SessionId::random());
    let attempt = registry
        .lock()
        .unwrap()
        .reserve(&connection, request)
        .unwrap();
    let lease = attempt.ready.lease;
    let worker_registry = Arc::clone(&registry);
    let (release, held) = oneshot::channel();
    let worker = tokio::spawn(async move {
        held.await.unwrap();
        worker_registry.lock().unwrap().activate(lease)
    });
    let stopped = registry
        .lock()
        .unwrap()
        .stop(&connection, unknown_stop(request.id))
        .unwrap();
    let Stop::Cleanup(cleanup) = stopped else {
        panic!("reserved generation needs cleanup")
    };
    assert_eq!(cleanup.ready.lease, lease);
    assert!(cleanup.cancel.is_cancelled());
    assert!(attempt.cancel.is_cancelled());
    release.send(()).unwrap();
    assert_eq!(worker.await.unwrap(), Err(Refusal::Stopped));
    let mut registry = registry.lock().unwrap();
    registry
        .finish(
            lease,
            StopVoiceResult::Uncertain {
                reason: VoiceFailure::TimedOut,
            },
        )
        .unwrap();
    assert_eq!(
        registry.entries[&request.id].phase,
        Phase::Uncertain(VoiceFailure::TimedOut)
    );
    assert!(matches!(
        registry.reserve(&connection, start(request.session)),
        Err(Refusal::Busy)
    ));
    assert!(registry.active_for_session(request.session).is_none());
    registry.finish(lease, StopVoiceResult::Stopped).unwrap();
    assert!(matches!(
        registry.reserve(&connection, request),
        Err(Refusal::Reused)
    ));
    let next = registry
        .reserve(&connection, start(request.session))
        .unwrap();
    assert_ne!(next.ready.lease.epoch, lease.epoch);
    // An obsolete completion cannot bind its failure to the new generation.
    registry
        .finish(
            lease,
            StopVoiceResult::Uncertain {
                reason: VoiceFailure::Provider,
            },
        )
        .unwrap();
    assert_eq!(registry.entries[&request.id].phase, Phase::Retired);
    assert!(registry.activate(next.ready.lease).is_ok());
}

#[test]
fn closure_cancels_jobs_without_retiring_an_unconfirmed_remote_reservation() {
    let mut registry = Registry::default();
    let connection = ConnectionLife::default();
    let request = start(SessionId::random());
    let attempt = registry.reserve(&connection, request).unwrap();
    registry.activate(attempt.ready.lease).unwrap();
    drop(connection);
    assert!(attempt.cancel.is_cancelled());
    assert!(registry.active_for_session(request.session).is_none());
    let next = ConnectionLife::default();
    assert!(matches!(
        registry.reserve(&next, start(request.session)),
        Err(Refusal::Busy)
    ));
    registry
        .finish(
            attempt.ready.lease,
            StopVoiceResult::Uncertain {
                reason: VoiceFailure::Provider,
            },
        )
        .unwrap();
    assert!(matches!(
        registry.reserve(&next, start(request.session)),
        Err(Refusal::Busy)
    ));
    registry
        .finish(attempt.ready.lease, StopVoiceResult::Stopped)
        .unwrap();
    assert!(registry.entries.is_empty());
    assert!(registry.reserve(&next, request).is_ok());
}

#[test]
fn foreign_or_stale_stop_and_completion_do_not_cancel_the_active_generation() {
    let mut registry = Registry::default();
    let connection = ConnectionLife::default();
    let other = ConnectionLife::default();
    let attempt = registry
        .reserve(&connection, start(SessionId::random()))
        .unwrap();
    let lease = attempt.ready.lease;
    registry.activate(lease).unwrap();
    let stale = VoiceLease {
        id: lease.id,
        epoch: VoiceEpoch::random(),
    };
    assert!(matches!(
        registry.stop(&other, unknown_stop(lease.id)),
        Err(Refusal::Owner)
    ));
    assert!(matches!(
        registry.stop(&connection, VoiceStopTarget::Active { lease: stale }),
        Err(Refusal::Generation)
    ));
    assert_eq!(
        registry.finish(stale, StopVoiceResult::Stopped),
        Err(Refusal::Generation)
    );
    assert!(!attempt.cancel.is_cancelled());
    assert!(registry.authorize(&connection.owner(), lease).is_ok());
    assert!(matches!(
        registry.stop(&connection, VoiceStopTarget::Active { lease }),
        Ok(Stop::Cleanup(_))
    ));
    assert!(attempt.cancel.is_cancelled());
    assert!(!connection.closed().is_cancelled());
}

#[test]
fn active_and_retained_nonce_quotas_are_independent_and_connection_scoped() {
    let mut registry = Registry::default();
    let connections: Vec<_> = (0..=ACTIVE_PER_NODE)
        .map(|_| ConnectionLife::default())
        .collect();
    for connection in &connections[..ACTIVE_PER_NODE] {
        registry
            .reserve(connection, start(SessionId::random()))
            .unwrap();
    }
    assert!(matches!(
        registry.reserve(&connections[ACTIVE_PER_NODE], start(SessionId::random())),
        Err(Refusal::Capacity)
    ));
    let mut markers = Registry::default();
    let connection = ConnectionLife::default();
    for _ in 0..ATTEMPTS_PER_CONNECTION {
        assert!(matches!(
            markers.stop(&connection, unknown_stop(VoiceId::random())),
            Ok(Stop::Complete)
        ));
    }
    assert!(matches!(
        markers.stop(&connection, unknown_stop(VoiceId::random())),
        Err(Refusal::Capacity)
    ));
    assert!(matches!(
        markers.reserve(&connection, start(SessionId::random())),
        Err(Refusal::Capacity)
    ));
    assert!(!connection.closed().is_cancelled());
    let other = ConnectionLife::default();
    assert!(markers.reserve(&other, start(SessionId::random())).is_ok());
    drop(connection);
    markers.reap();
    assert!(markers.cancelled_starts.is_empty());
}

#[test]
fn unknown_stop_nonce_history_has_a_node_cap_and_reaps_only_closed_connections() {
    let mut registry = Registry::default();
    let connections: Vec<_> = (0..ATTEMPTS_PER_NODE / ATTEMPTS_PER_CONNECTION)
        .map(|_| ConnectionLife::default())
        .collect();
    for connection in &connections {
        for _ in 0..ATTEMPTS_PER_CONNECTION {
            assert!(matches!(
                registry.stop(connection, unknown_stop(VoiceId::random())),
                Ok(Stop::Complete)
            ));
        }
    }
    assert_eq!(registry.cancelled_starts.len(), ATTEMPTS_PER_NODE);
    let extra = ConnectionLife::default();
    assert!(matches!(
        registry.stop(&extra, unknown_stop(VoiceId::random())),
        Err(Refusal::Capacity)
    ));
    assert!(matches!(
        registry.reserve(&extra, start(SessionId::random())),
        Err(Refusal::Capacity)
    ));
    // Retaining the connections is what retains nonce authority. An ordinary
    // Stop never cancels that parent token or breaks non-voice operations.
    drop(connections);
    assert!(registry.reserve(&extra, start(SessionId::random())).is_ok());
    assert!(registry.cancelled_starts.is_empty());
}
