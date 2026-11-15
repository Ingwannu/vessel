//! Private state barriers do not stand in for actual relay transport fixtures.
use super::*;
use std::time::Duration;
use tokio::{sync::oneshot, task::JoinHandle};
const OUTER: Duration = Duration::from_secs(30);
const PHASE: Duration = Duration::from_secs(5);
const CANARY: &str = "synthetic-private-relay-token";

fn configured() -> Configured {
    let mut configured = Configured::default();
    configured
        .configure(
            &["ws://127.0.0.1:1/private-route".parse().unwrap()],
            &RelayToken::new(CANARY).unwrap(),
            &TlsConfig::with_roots(rustls::RootCertStore::empty()).unwrap(),
        )
        .unwrap();
    configured
}

#[test]
fn saved_consent_wins_before_tasks_and_unknown_never_advertises() {
    let mut configured = configured();
    let advertisement = configured.advertisement();
    assert!(advertisement.includes_configured());
    configured.initial(Some(RelayChange::Disabled));
    assert_eq!(configured.snapshot(), ConfiguredRelays::Disabled);
    assert!(!advertisement.includes_configured());
    configured.initial(Some(RelayChange::Unknown));
    assert_eq!(configured.snapshot(), ConfiguredRelays::Unknown);
    assert!(!advertisement.includes_configured());
    configured.initial(Some(RelayChange::Enabled));
    assert!(advertisement.includes_configured());
    assert_eq!(
        configured.snapshot(),
        ConfiguredRelays::Enabled {
            health: RelayHealth::Connecting
        }
    );
    drop(configured);
    assert!(
        !advertisement.includes_configured(),
        "sender loss never retains an advertised route"
    );
}

#[test]
fn unconfigured_and_over_cap_never_start_or_truncate() {
    let mut configured = Configured::default();
    assert_eq!(
        configured.validate(RelayChange::Enabled),
        Err(RelaySettingsError::NotConfigured)
    );
    assert_eq!(
        configured.validate(RelayChange::Disabled),
        Err(RelaySettingsError::NotConfigured)
    );
    assert_eq!(
        configured.validate(RelayChange::Unknown),
        Err(RelaySettingsError::Unknown)
    );
    let route: RouteUrl = "ws://127.0.0.1:1".parse().unwrap();
    let tls = TlsConfig::with_roots(rustls::RootCertStore::empty()).unwrap();
    let token = RelayToken::new(CANARY).unwrap();
    let count = MAX_CONFIGURED_RELAYS.checked_add(1).unwrap();
    assert_eq!(
        configured.configure(&vec![route.clone(); count], &token, &tls),
        Err(RelaySettingsError::NotConfigured)
    );
    assert!(configured.recipe.is_none());
    configured
        .configure(&vec![route; MAX_CONFIGURED_RELAYS], &token, &tls)
        .unwrap();
    assert_eq!(
        configured.recipe.as_ref().unwrap().routes.len(),
        MAX_CONFIGURED_RELAYS
    );
    let debug = format!("{configured:?}");
    assert!(!debug.contains(CANARY));
    assert!(!debug.contains("127.0.0.1"));
}

#[test]
fn health_never_calls_a_dropped_registered_sender_online() {
    let route: RouteUrl = "ws://127.0.0.1:1".parse().unwrap();
    let (send, receive) = watch::channel(LinkState::Registered);
    assert_eq!(
        health(&[(route.clone(), receive.clone())]),
        RelayHealth::Online
    );
    drop(send);
    assert_eq!(health(&[(route, receive)]), RelayHealth::Unknown);
}

struct Task(JoinHandle<()>);
impl Drop for Task {
    fn drop(&mut self) {
        self.0.abort();
    }
}

#[tokio::test]
async fn a_cancelled_waiter_does_not_release_a_stopping_generation() {
    tokio::time::timeout(OUTER, async {
        let mut configured = configured();
        let (release, released) = oneshot::channel();
        let tracker = TaskTracker::new();
        let task = Task(tracker.spawn(async move {
            let _released = released.await;
        }));
        let _first_close = tracker.close();
        let stop = CancellationToken::new();
        configured.generation = Some(Generation {
            phase: Phase::Active,
            stop: stop.clone(),
            tasks: tracker,
            links: Vec::new(),
        });
        let waiter = configured.disable().unwrap();
        assert!(stop.is_cancelled());
        assert_eq!(configured.snapshot(), ConfiguredRelays::Disabling);
        assert_eq!(
            configured.validate(RelayChange::Enabled),
            Err(RelaySettingsError::Busy)
        );
        drop(waiter);
        assert_eq!(configured.snapshot(), ConfiguredRelays::Disabling);
        assert_eq!(
            configured.validate(RelayChange::Enabled),
            Err(RelaySettingsError::Busy)
        );
        let repeated = configured.disable().unwrap();
        release.send(()).unwrap();
        tokio::time::timeout(PHASE, repeated.wait())
            .await
            .expect("registration completion");
        assert_eq!(configured.snapshot(), ConfiguredRelays::Disabled);
        assert!(configured.generation.is_none());
        assert_eq!(configured.validate(RelayChange::Enabled), Ok(()));
        drop(task);
    })
    .await
    .expect("configured generation outer safety");
}
