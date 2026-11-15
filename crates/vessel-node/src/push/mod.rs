//! Device-scoped push authority and presence (ADR 0041). The worker owns
//! delivery, the store owns leases, and the protocol owns capability shapes.
//! Neither a relay nor a service receives request or conversation content.
mod crypto;
mod delivery;
mod policy;
mod vapid;
mod worker;
pub(crate) use worker::start;
#[cfg(test)]
mod tests;
use chrono::{TimeDelta, Utc};
use crypto::{Crypto, CryptoError};
use delivery::{Delivered, DeliveryError};
use policy::{Plan, Presence};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};
use tokio::sync::OnceCell;
use vessel_proto::push::{PushKey, PushLease, PushSubscription, ReportPushPresence};
use vessel_proto::{DeviceId, ErrorCode, SessionId, WireError};
use vessel_store::{DataDir, Store, StoredPush};

/// Identity of one transport connection, shared by its delegated call contexts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct PresenceId(uuid::Uuid);
impl PresenceId {
    pub(crate) fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum PushError {
    #[error(transparent)]
    Crypto(#[from] CryptoError),
    #[error(transparent)]
    Delivery(#[from] DeliveryError),
    #[error("push subscriptions could not be saved; reconnect and try again")]
    Store(#[from] vessel_store::StoreError),
    #[error("push setup could not finish; reconnect and try again")]
    Task(#[source] tokio::task::JoinError),
    #[error("the node's clock cannot represent a push lease; correct its clock and try again")]
    Clock,
}
impl PushError {
    pub(crate) fn wire(&self) -> WireError {
        let code = match self {
            Self::Crypto(
                CryptoError::PublicKey(_) | CryptoError::KeyShape(_) | CryptoError::Endpoint(_),
            )
            | Self::Delivery(DeliveryError::Endpoint) => ErrorCode::InvalidRequest,
            Self::Store(error) => error.code(),
            Self::Crypto(
                CryptoError::KeyFile(_)
                | CryptoError::PrivateKey(_)
                | CryptoError::Tls(_)
                | CryptoError::Encrypt(_)
                | CryptoError::Vapid(_),
            )
            | Self::Task(_)
            | Self::Clock => ErrorCode::Internal,
            Self::Delivery(
                DeliveryError::Resolve(_) | DeliveryError::Timeout | DeliveryError::Http(_),
            ) => ErrorCode::Unavailable,
        };
        WireError::new(code, self.to_string())
    }
    /// Safe diagnostic class, never a URL-bearing source chain.
    pub(crate) const fn label(&self) -> &'static str {
        match self {
            Self::Crypto(_) => "crypto",
            Self::Delivery(_) => "delivery",
            Self::Store(_) => "store",
            Self::Task(_) => "task",
            Self::Clock => "clock",
        }
    }
}

#[derive(Debug)]
pub(crate) struct Push {
    store: Store,
    dir: DataDir,
    crypto: OnceCell<Arc<Crypto>>,
    #[cfg(test)]
    preparation: Option<Arc<Preparation>>,
    presence: Mutex<HashMap<PresenceId, Presence>>,
    #[cfg(test)]
    test_origin: Option<url::Url>,
}
impl Push {
    /// The node's push service from its configuration, started: the node
    /// assembly's one step for push (protocol 22), so `node.rs` does not
    /// carry push's test hooks.
    pub(crate) async fn started(
        config: &crate::Config,
        store: Store,
        tasks: &tokio_util::task::TaskTracker,
        stop: tokio_util::sync::CancellationToken,
    ) -> Result<Arc<Self>, vessel_store::StoreError> {
        let push = Self::new(store, config.data_dir.clone());
        #[cfg(test)]
        let push = if let Some(origin) = &config.push_test_origin {
            push.test_at(origin.clone())
        } else {
            push
        };
        #[cfg(test)]
        let push = push.pausing_at(config.push_preparation.clone());
        let push = Arc::new(push);
        start(Arc::clone(&push), tasks, stop).await?;
        Ok(push)
    }

    pub(crate) fn new(store: Store, dir: DataDir) -> Self {
        Self {
            store,
            dir,
            crypto: OnceCell::new(),
            #[cfg(test)]
            preparation: None,
            presence: Mutex::default(),
            #[cfg(test)]
            test_origin: None,
        }
    }
    #[cfg(test)]
    pub(crate) fn test_at(mut self, origin: url::Url) -> Self {
        self.test_origin = Some(origin);
        self
    }
    #[cfg(test)]
    pub(crate) fn pausing_at(mut self, preparation: Option<Arc<Preparation>>) -> Self {
        self.preparation = preparation;
        self
    }
    #[cfg(test)]
    async fn prepared(&self, stage: PreparedStage) {
        if let Some(preparation) = self
            .preparation
            .as_ref()
            .filter(|hook| hook.stages.contains(&stage))
        {
            let (release, released) = tokio::sync::oneshot::channel();
            if preparation.ready.send(release).await.is_ok() && released.await.is_err() {
                tracing::debug!("push preparation test released its barrier");
            }
        }
    }
    async fn crypto(&self) -> Result<Arc<Crypto>, PushError> {
        self.crypto
            .get_or_try_init(|| async {
                let dir = self.dir.clone();
                #[cfg(test)]
                let origin = self.test_origin.clone();
                let crypto = tokio::task::spawn_blocking(move || {
                    let crypto = Crypto::load(&dir)?;
                    #[cfg(test)]
                    let crypto = if let Some(origin) = origin {
                        crypto.test_at(origin)
                    } else {
                        crypto
                    };
                    Ok::<_, CryptoError>(crypto)
                })
                .await
                .map_err(PushError::Task)??;
                Ok(Arc::new(crypto))
            })
            .await
            .map(Arc::clone)
    }
    pub(crate) async fn key(&self) -> Result<PushKey, PushError> {
        Ok(self.crypto().await?.public())
    }
    pub(crate) async fn subscribe(
        &self,
        device: DeviceId,
        subscription: PushSubscription,
    ) -> Result<PushLease, PushError> {
        Crypto::validate(&subscription)?;
        let crypto = self.crypto().await?;
        crypto.delivery.validate(&subscription.endpoint).await?;
        let now = Utc::now();
        let expires_at = now
            .checked_add_signed(TimeDelta::hours(48))
            .ok_or(PushError::Clock)?;
        self.store
            .subscribe_push(
                StoredPush {
                    device,
                    subscription,
                    expires_at,
                },
                now,
            )
            .await?;
        Ok(PushLease { expires_at })
    }
    pub(crate) async fn unsubscribe(&self, device: DeviceId) -> Result<(), PushError> {
        Ok(self.store.remove_push(device).await?)
    }
    /// Returns whether a close watcher is needed. One per connection, at
    /// most 128: an idle-but-open connection consumes a slot, not activity.
    pub(crate) fn present(
        &self,
        id: PresenceId,
        report: ReportPushPresence,
    ) -> Result<bool, WireError> {
        let mut states = self.presence.lock().unwrap_or_else(PoisonError::into_inner);
        let fresh = !states.contains_key(&id);
        if fresh && states.len() >= 128 {
            return Err(WireError::new(
                ErrorCode::Unavailable,
                "too many windows report presence; close an unused window and try again",
            ));
        }
        states
            .entry(id)
            .or_insert(Presence {
                focus: vessel_proto::push::PushFocus::Background,
                active_at: None,
            })
            .report(report, Utc::now());
        Ok(fresh)
    }
    pub(crate) fn forget(&self, id: PresenceId) {
        let _previous = self
            .presence
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&id);
    }
    fn should_push(&self, session: SessionId) -> bool {
        let states: Vec<_> = self
            .presence
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .values()
            .copied()
            .collect();
        matches!(policy::plan(&states, session, Utc::now()), Plan::Push)
    }
}

/// Deterministic unit-test barriers; absent from installed builds and the wire.
#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PreparedStage {
    Observed,
    Crypto,
    Transport,
}
#[cfg(test)]
#[derive(Debug)]
pub(crate) struct Preparation {
    pub(crate) stages: Vec<PreparedStage>,
    pub(crate) ready: tokio::sync::mpsc::Sender<tokio::sync::oneshot::Sender<()>>,
    pub(crate) finished: tokio::sync::mpsc::Sender<Option<vessel_store::PushAttention>>,
}
