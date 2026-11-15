//! One independent native-root initialization per node's dictation service.
//!
//! Cancellation of a start must not restart a non-abortable blocking root loader.
//! `OnceLock` installs a watch channel synchronously; its one producer runs on its
//! own task, and callers only wait on cloned receivers. The cached result contains
//! TLS configuration, never an HTTP pool, destination, credential or audio.

use std::sync::{Arc, OnceLock};
use tokio::sync::watch;

/// Root failures retain their causes while callers share the same initialization.
#[derive(Debug, thiserror::Error)]
pub(super) enum RootsError {
    /// Preserve the existing catalogue helper's rustls cause.
    #[error("the dictation TLS configuration was refused")]
    Tls(#[source] rustls::Error),
    /// A blocking worker may fail independently of the caller's cancellation.
    #[error("the dictation root-loading worker did not finish")]
    Worker(#[source] tokio::task::JoinError),
    /// A producer that disappears must not leave every future start waiting.
    #[error("the dictation root-loading result was lost")]
    Closed(#[source] watch::error::RecvError),
}

/// Clone the root configuration for a stream-owned HTTP client.
#[derive(Clone)]
pub(super) struct Configuration(Arc<rustls::ClientConfig>);

impl Configuration {
    /// Clients share roots but keep their own host pools and credentials.
    #[must_use]
    pub(super) fn for_client(&self) -> rustls::ClientConfig {
        let mut config = self.0.as_ref().clone();
        // rustls clones share their resumption store. Keep session names/tickets
        // with this stream-owned client, never with the shared root cache.
        config.resumption = rustls::client::Resumption::default();
        config
    }
}

type Loaded = Result<Configuration, Arc<RootsError>>;

/// Construct once with the node's dictation service, never once per stream.
#[derive(Default)]
pub(super) struct Factory {
    result: OnceLock<watch::Receiver<Option<Loaded>>>,
    #[cfg(test)]
    gate: Option<Arc<Gate>>,
    #[cfg(test)]
    refusal_probe: Option<Arc<std::sync::atomic::AtomicUsize>>,
}

impl Factory {
    /// Only the synthetic loader edge can force a native-root refusal.
    #[cfg(test)]
    pub(super) fn refusing_once(probe: Arc<std::sync::atomic::AtomicUsize>) -> Self {
        Self {
            refusal_probe: Some(probe),
            ..Self::default()
        }
    }

    fn initialize(
        #[cfg(test)] gate: Option<Arc<Gate>>,
        #[cfg(test)] refusal_probe: Option<Arc<std::sync::atomic::AtomicUsize>>,
    ) -> watch::Receiver<Option<Loaded>> {
        let (sender, receiver) = watch::channel(None);
        drop(tokio::spawn(async move {
            #[cfg(test)]
            if let Some(gate) = gate {
                let _previous = gate
                    .starts
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                gate.entered.notify_one();
                gate.release.notified().await;
            }
            #[cfg(test)]
            let load = move || {
                if let Some(probe) = refusal_probe
                    && probe.fetch_add(1, std::sync::atomic::Ordering::SeqCst) == 0
                {
                    return Err(rustls::Error::General("synthetic root refusal".to_owned()));
                }
                crate::http_tls::config()
            };
            #[cfg(not(test))]
            let load = crate::http_tls::config;
            let result = tokio::task::spawn_blocking(load)
                .await
                .map_err(RootsError::Worker)
                .and_then(|result| result.map_err(RootsError::Tls))
                .map(|config| Configuration(Arc::new(config)))
                .map_err(Arc::new);
            // The cache has no earlier result. Once the service and all
            // waiters are gone, the last sender drops this configuration.
            drop(sender.send_replace(Some(result)));
        }));
        receiver
    }

    /// Waiting is cancel-safe: the independent producer remains unique.
    pub(super) async fn get(&self) -> Result<Configuration, Arc<RootsError>> {
        #[cfg(test)]
        let initialize = || Self::initialize(self.gate.clone(), self.refusal_probe.clone());
        #[cfg(not(test))]
        let initialize = Self::initialize;
        let mut receiver = self.result.get_or_init(initialize).clone();
        loop {
            let loaded = receiver.borrow().clone();
            if let Some(result) = loaded {
                return result;
            }
            receiver
                .changed()
                .await
                .map_err(|source| Arc::new(RootsError::Closed(source)))?;
        }
    }
}

#[cfg(test)]
#[derive(Default)]
struct Gate {
    starts: std::sync::atomic::AtomicUsize,
    entered: tokio::sync::Notify,
    release: tokio::sync::Notify,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::Ordering;

    #[tokio::test]
    async fn a_cancelled_wait_does_not_start_another_loader() {
        vessel_testkit::within(std::time::Duration::from_secs(60), async {
            let gate = Arc::new(Gate::default());
            let factory = Factory {
                gate: Some(Arc::clone(&gate)),
                ..Factory::default()
            };
            let mut first = Box::pin(factory.get());
            assert!(futures_util::poll!(first.as_mut()).is_pending());
            gate.entered.notified().await;
            drop(first);
            let mut second = Box::pin(factory.get());
            assert!(futures_util::poll!(second.as_mut()).is_pending());
            assert_eq!(gate.starts.load(Ordering::SeqCst), 1);
            gate.release.notify_one();
            let loaded = second.await.unwrap();
            let cached = factory.get().await.unwrap();
            assert!(Arc::ptr_eq(&loaded.0, &cached.0));
            assert_eq!(gate.starts.load(Ordering::SeqCst), 1);
        })
        .await
        .unwrap();
    }

    /// Observe the external session-store interface without network I/O.
    #[derive(Debug, Default)]
    struct SessionProbe(std::sync::atomic::AtomicUsize);

    impl rustls::client::ClientSessionStore for SessionProbe {
        fn set_kx_hint(
            &self,
            _name: rustls::pki_types::ServerName<'static>,
            _group: rustls::NamedGroup,
        ) {
        }
        fn kx_hint(&self, _name: &rustls::pki_types::ServerName<'_>) -> Option<rustls::NamedGroup> {
            let _previous = self.0.fetch_add(1, Ordering::SeqCst);
            None
        }
        fn set_tls12_session(
            &self,
            _name: rustls::pki_types::ServerName<'static>,
            _value: rustls::client::Tls12ClientSessionValue,
        ) {
        }
        fn tls12_session(
            &self,
            _name: &rustls::pki_types::ServerName<'_>,
        ) -> Option<rustls::client::Tls12ClientSessionValue> {
            let _previous = self.0.fetch_add(1, Ordering::SeqCst);
            None
        }
        fn remove_tls12_session(&self, _name: &rustls::pki_types::ServerName<'static>) {}
        fn insert_tls13_ticket(
            &self,
            _name: rustls::pki_types::ServerName<'static>,
            _value: rustls::client::Tls13ClientSessionValue,
        ) {
        }
        fn take_tls13_ticket(
            &self,
            _name: &rustls::pki_types::ServerName<'static>,
        ) -> Option<rustls::client::Tls13ClientSessionValue> {
            let _previous = self.0.fetch_add(1, Ordering::SeqCst);
            None
        }
    }

    #[test]
    fn stream_clients_cannot_populate_or_read_the_factory_session_store() {
        let probe = Arc::new(SessionProbe::default());
        let mut config = crate::http_tls::config().unwrap();
        let shared = Arc::clone(&probe);
        config.resumption = rustls::client::Resumption::store(shared);
        let config = Configuration(Arc::new(config));
        let name = rustls::pki_types::ServerName::try_from("dictation.invalid").unwrap();
        // Establish that a normal clone really would reuse this mutable store.
        drop(rustls::ClientConnection::new(Arc::clone(&config.0), name.clone()).unwrap());
        assert!(probe.0.load(Ordering::SeqCst) > 0);
        probe.0.store(0, Ordering::SeqCst);
        for _ in 0..2 {
            drop(
                rustls::ClientConnection::new(Arc::new(config.for_client()), name.clone()).unwrap(),
            );
        }
        assert_eq!(probe.0.load(Ordering::SeqCst), 0);
    }
}
