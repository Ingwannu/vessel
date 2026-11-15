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
enum RootsError {
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
struct Configuration(Arc<rustls::ClientConfig>);

type Loaded = Result<Configuration, Arc<RootsError>>;

/// Construct once with the node's dictation service, never once per stream.
#[derive(Default)]
struct Factory {
    result: OnceLock<watch::Receiver<Option<Loaded>>>,
    #[cfg(test)]
    gate: Option<Arc<Gate>>,
}

impl Factory {
    /// Waiting is cancel-safe: the independent producer remains unique.
    async fn get(&self) -> Result<Configuration, Arc<RootsError>> {
        let mut receiver = self
            .result
            .get_or_init(|| {
                let (sender, receiver) = watch::channel(None);
                #[cfg(test)]
                let gate = self.gate.clone();
                drop(tokio::spawn(async move {
                    #[cfg(test)]
                    if let Some(gate) = gate {
                        let _previous = gate
                            .starts
                            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        gate.entered.notify_one();
                        gate.release.notified().await;
                    }
                    let result = tokio::task::spawn_blocking(crate::http_tls::config)
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
            })
            .clone();
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
}
