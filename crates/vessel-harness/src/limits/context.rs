//! Per-node admission and vendor backoff. Tokens never leave their format
//! boundary: this context keeps only equality keys, deadlines and a
//! credential-free client. It is neither global nor persisted.
//!
//! [Decision Log] (2026-11-16, ADR 0104 third consumer)
//! - Purpose: a cancelled or timed-out provider read must not leave an
//!   untracked native `getaddrinfo` running after the node says it stopped.
//! - Constraint: reqwest's default resolver starts OS work that dropping the
//!   HTTP future cannot stop; Harness keeps proto/proc only and must not
//!   depend on the relay crate that owns the tracked primitive.
//! - Alternatives: a Harness DNS resolver or cache (a second implementation,
//!   rejected by ADR 0104); a relay edge from Harness (rejected by the
//!   coordinator's graph ruling); keeping default Gai for the legacy read
//!   (leaves the same untracked work on the active method).
//! - Choice: the node injects an `Arc` of reqwest's own `Resolve` trait, the
//!   library's real extension point, and the one persistent client takes it
//!   through `ClientBuilder::dns_resolver`. The node retains the scope behind
//!   that resolver and drains it after both source owners have ended.
//! - Fail closed: a context built without a resolver never builds a client;
//!   every live read answers Unsupported instead of falling back to Gai.
//! - Cost: physical lookups now share a finite quota; a full or closed scope
//!   refuses a lookup, which a read reports as Unreachable.
use chrono::{DateTime, Utc};
use std::{
    collections::BTreeMap,
    fmt,
    sync::{Arc, Mutex, PoisonError},
};
use tokio::sync::{OnceCell, OwnedSemaphorePermit, Semaphore};
use vessel_proto::limits::LimitFailure;

const JOBS: usize = 2;
const COOLDOWNS: usize = 256;
/// Shared lifetime/admission of one node's saved-sign-in probes.
#[derive(Clone)]
pub struct LimitContext(Arc<Shared>);
struct Shared {
    slots: Arc<Semaphore>,
    client: OnceCell<super::http::Http>,
    /// The node's tracked name resolution for this client's whole lifetime.
    resolver: Option<Arc<dyn reqwest::dns::Resolve>>,
    cooldowns: Arc<Mutex<BTreeMap<TokenKey, Entry>>>,
}
impl fmt::Debug for LimitContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("LimitContext([local probe state])")
    }
}
impl Default for LimitContext {
    fn default() -> Self {
        Self::new()
    }
}
impl LimitContext {
    /// The existing read bound. The node sizes its client-scoped DNS quota from
    /// it (ADR 0104); the two counters stay separate, and this admission is not
    /// evidence that any lookup finished.
    pub const READ_SLOTS: usize = JOBS;

    /// Empty context without name resolution: no file, certificate or vendor
    /// I/O, and every live read fails closed (recorded sources need none).
    #[must_use]
    pub fn new() -> Self {
        Self::with(None)
    }

    /// Empty context whose one persistent client resolves names only through
    /// `resolver`. The caller owns the lifetime behind it and must drain it
    /// after every read using this context has ended.
    #[must_use]
    pub fn resolving(resolver: Arc<dyn reqwest::dns::Resolve>) -> Self {
        Self::with(Some(resolver))
    }

    fn with(resolver: Option<Arc<dyn reqwest::dns::Resolve>>) -> Self {
        Self(Arc::new(Shared {
            slots: Arc::new(Semaphore::new(JOBS)),
            client: OnceCell::new(),
            resolver,
            cooldowns: Arc::new(Mutex::new(BTreeMap::new())),
        }))
    }
    pub(super) fn admit(&self) -> Result<Admission, LimitFailure> {
        Arc::clone(&self.0.slots)
            .try_acquire_owned()
            .map(Arc::new)
            .map_err(|_full| LimitFailure::Busy)
    }
    /// Call only after stopping and draining this node's probe jobs. An
    /// unexpected caller cancellation cannot abandon a blocking reader:
    /// its permit must return before the node releases its own lifetime.
    pub async fn drain(&self) {
        match Arc::clone(&self.0.slots).acquire_many_owned(2).await {
            Ok(permit) => drop(permit),
            Err(_closed) => {}
        }
    }
    pub(super) async fn client(
        &self,
        admission: &Admission,
    ) -> Result<&super::http::Http, super::http::HttpError> {
        // Refuse before any blocking work: no resolver means no client at all,
        // never reqwest's default resolver.
        let resolver = self
            .0
            .resolver
            .as_ref()
            .ok_or(super::http::HttpError::Unresolved)?;
        self.0
            .client
            .get_or_try_init(|| async {
                let held = Arc::clone(admission);
                let resolver = Arc::clone(resolver);
                tokio::task::spawn_blocking(move || {
                    let _held = held;
                    super::http::Http::new(resolver)
                })
                .await
                .map_err(super::http::HttpError::Task)?
            })
            .await
    }
    pub(super) fn check(&self, key: TokenKey, now: DateTime<Utc>) -> Result<Lease, Refusal> {
        let mut entries = self
            .0
            .cooldowns
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        entries.retain(|_, entry| match entry {
            Entry::Checking => true,
            Entry::Until(at) => *at > now,
        });
        if let Some(entry) = entries.get(&key) {
            return Err(match entry {
                Entry::Checking => Refusal {
                    reason: LimitFailure::Busy,
                    retry_at: None,
                },
                Entry::Until(at) => Refusal {
                    reason: LimitFailure::RateLimited,
                    retry_at: Some(*at),
                },
            });
        }
        if entries.len() >= COOLDOWNS {
            return Err(Refusal {
                reason: LimitFailure::Busy,
                retry_at: None,
            });
        }
        let _previous = entries.insert(key.clone(), Entry::Checking);
        Ok(Lease {
            entries: Arc::clone(&self.0.cooldowns),
            key,
            finished: false,
        })
    }
}
// A clone belongs to EVERY queued/running blocking closure, not merely the
// awaiting future. This is the same ownership rule as guarded store scans.
pub(super) type Admission = Arc<OwnedSemaphorePermit>;
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct TokenKey([u8; 32]);
impl TokenKey {
    pub(super) fn account(
        source: vessel_proto::usage_reports::BuiltinUsageSource,
        token: &str,
    ) -> Self {
        use vessel_proto::usage_reports::BuiltinUsageSource;
        let vendor = match source {
            BuiltinUsageSource::Claude => "anthropic",
            BuiltinUsageSource::Codex => "chatgpt",
            BuiltinUsageSource::Copilot => "copilot",
            BuiltinUsageSource::Cursor => "cursor",
            BuiltinUsageSource::Grok => "grok",
            BuiltinUsageSource::Kimi => "kimi",
            BuiltinUsageSource::MiniMax => "minimax",
            BuiltinUsageSource::OpenCodeGo => "opencode-go",
            BuiltinUsageSource::Zai => "zai",
        };
        Self::vendor(vendor, token)
    }
    fn vendor(vendor: &str, token: &str) -> Self {
        use sha2::{Digest, Sha256};
        let mut hash = Sha256::new();
        hash.update(vendor.as_bytes());
        hash.update([0]);
        hash.update(token.as_bytes());
        Self(hash.finalize().into())
    }
    pub(super) fn new(source: vessel_proto::limits::LimitLoginStore, token: &str) -> Self {
        let vendor = match source {
            vessel_proto::limits::LimitLoginStore::ClaudeCode => "anthropic",
            vessel_proto::limits::LimitLoginStore::Codex
            | vessel_proto::limits::LimitLoginStore::OpenCode => "chatgpt",
        };
        Self::vendor(vendor, token)
    }
}
enum Entry {
    Checking,
    Until(DateTime<Utc>),
}
pub(super) struct Refusal {
    pub(super) reason: LimitFailure,
    pub(super) retry_at: Option<DateTime<Utc>>,
}
pub(super) struct Lease {
    entries: Arc<Mutex<BTreeMap<TokenKey, Entry>>>,
    key: TokenKey,
    finished: bool,
}
impl Lease {
    pub(super) fn finish(mut self, retry_at: Option<DateTime<Utc>>) {
        let mut entries = self.entries.lock().unwrap_or_else(PoisonError::into_inner);
        match retry_at {
            Some(at) => {
                let _previous = entries.insert(self.key.clone(), Entry::Until(at));
            }
            None => {
                let _previous = entries.remove(&self.key);
            }
        }
        self.finished = true;
    }
}
impl Drop for Lease {
    fn drop(&mut self) {
        if self.finished {
            return;
        }
        let mut entries = self.entries.lock().unwrap_or_else(PoisonError::into_inner);
        if matches!(entries.get(&self.key), Some(Entry::Checking)) {
            let _previous = entries.remove(&self.key);
        }
    }
}
