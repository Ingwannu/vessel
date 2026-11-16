//! Opt-in saved-sign-in quota lookups. All provider schemas, paths and
//! fixed network destinations stay here. The node owns policy, admission
//! and cache lifetime; this module never discovers or changes a login.
mod account_sources;
mod claude;
pub use account_sources::{
    AccountSourceAccess, AccountUsageIdentity, AccountUsageObservation, AccountUsageProbe,
    RecordedUsageFormat,
};
mod codex;
mod context;
mod credential;
pub use context::LimitContext;
mod decode;
mod environment;
mod http;
mod keychain;
mod model;
mod number;
use crate::{HarnessEnv, Profile};
use chrono::{DateTime, Utc};
use decode::unique_key;
pub use environment::LimitEnvironment;
use sha2::{Digest, Sha256};
use std::{fmt, path::PathBuf, sync::Arc};
use vessel_proto::limits::{LimitFailure, LimitLoginStore, LimitSnapshot};

/// Resolved configured sign-in location. Never accepted from a client.
/// Shared immutable configuration keeps launch futures small and clones cheap;
/// this allocation contains no token and equality compares the configuration.
#[derive(Clone, PartialEq, Eq)]
pub struct LimitBinding(Arc<Bound>);
#[derive(PartialEq, Eq)]
struct Bound {
    local: environment::Binding,
    account: vessel_proto::limits::LimitAccount,
}
impl fmt::Debug for LimitBinding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("LimitBinding([local saved sign-in])")
    }
}
impl LimitBinding {
    /// Canonical account label captured with the actual launch/configuration.
    #[must_use]
    pub fn account(&self) -> &vessel_proto::limits::LimitAccount {
        &self.0.account
    }
    /// Store kind for the opt-in disclosure, without a private path.
    #[must_use]
    pub fn source(&self) -> LimitLoginStore {
        self.0.local.source
    }
}
/// Resolve profile configuration without reading any sign-in or endpoint.
/// # Errors
/// Unsupported sources, foreign model routes or ambiguous relative homes.
pub fn limits_binding(
    env: &LimitEnvironment,
    homes: &HarnessEnv,
    profile: &Profile,
) -> Result<LimitBinding, LimitFailure> {
    let store = profile
        .known()
        .and_then(|kind| crate::harnesses::descriptor(kind).limits)
        .ok_or(LimitFailure::Unsupported)?;
    env.resolve(homes, profile, store).map(|local| {
        LimitBinding(Arc::new(Bound {
            local,
            account: vessel_proto::limits::LimitAccount {
                harness: profile.harness(),
                profile: Some(profile.name().clone()),
            },
        }))
    })
}
/// Internal equality key for one observed saved login; never serialized,
/// logged, persisted or used as a public account identity.
#[derive(Clone, PartialEq, Eq)]
pub struct LimitLoginIdentity([u8; 32]);
impl fmt::Debug for LimitLoginIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("LimitLoginIdentity([redacted])")
    }
}
/// A sanitized probe outcome, including cooldown and same-login association.
#[derive(Debug)]
pub struct LimitProbeResult {
    /// Observation or safe failure category.
    pub result: Result<LimitSnapshot, LimitFailure>,
    /// None means no same-login snapshot may be retained.
    pub identity: Option<LimitLoginIdentity>,
    /// Vendor Retry-After, if available.
    pub retry_at: Option<DateTime<Utc>>,
}
/// Explicit backend. `Config::new` must use Disabled; only binaries opt into
/// Local, still guarded by the separately persisted off-by-default policy.
#[derive(Debug, Clone, Default)]
pub enum LimitProbe {
    /// No file, Keychain, certificate or vendor access.
    #[default]
    Disabled,
    /// The configured local login and fixed HTTPS origins. Disabled in
    /// builds with the fakes feature, even if accidentally selected.
    Local,
    /// Synthetic sign-ins and vendor-shaped replies inside one test root.
    Recorded {
        /// Every read is canonicalized under this temporary root.
        root: PathBuf,
        /// Bounded synthetic body; status and Retry-After are metadata.
        body: PathBuf,
        /// Recorded response status.
        status: u16,
        /// Recorded Retry-After header.
        retry_after: Option<String>,
    },
}
impl LimitProbe {
    /// Whether this explicitly selected backend can perform a saved-login probe.
    #[must_use]
    pub fn supported(&self) -> bool {
        match self {
            Self::Disabled => false,
            Self::Local => !cfg!(feature = "fakes"),
            Self::Recorded { .. } => true,
        }
    }

    /// Probe one resolved login. The context is shared by all this node's
    /// callers; both it and the actor retain admission through local work.
    pub async fn read(
        &self,
        context: &LimitContext,
        binding: &LimitBinding,
        now: DateTime<Utc>,
        stop: &tokio_util::sync::CancellationToken,
    ) -> LimitProbeResult {
        if stop.is_cancelled()
            || matches!(self, Self::Disabled)
            || cfg!(feature = "fakes") && matches!(self, Self::Local)
        {
            return failed(LimitFailure::Unsupported, None, None);
        }
        let admission = match context.admit() {
            Ok(admission) => admission,
            Err(reason) => return failed(reason, None, None),
        };
        let credential = match self.credential(binding, &admission, stop).await {
            Ok(credential) => credential,
            Err(reason) => return failed(reason, None, None),
        };
        let mut hash = Sha256::new();
        hash.update(credential.token.as_bytes());
        hash.update([0]);
        if let Some(account) = &credential.account {
            hash.update(account.as_bytes());
        }
        let identity = Some(LimitLoginIdentity(hash.finalize().into()));
        if credential
            .expires_ms
            .is_some_and(|expires| expires <= now.timestamp_millis())
        {
            return failed(LimitFailure::Expired, identity, None);
        }
        let key = context::TokenKey::new(binding.source(), credential.token.as_str());
        let lease = match context.check(key, now) {
            Ok(lease) => lease,
            Err(refusal) => return failed(refusal.reason, identity, refusal.retry_at),
        };
        let result = self
            .body(
                context,
                &admission,
                binding.source(),
                &credential,
                now,
                stop,
            )
            .await;
        let body = match result {
            Ok(body) => {
                lease.finish(None);
                body
            }
            Err((reason, retry_at)) => {
                lease.finish(if reason == LimitFailure::RateLimited {
                    retry_at
                } else {
                    None
                });
                return failed(reason, identity, retry_at);
            }
        };
        let at = if matches!(self, Self::Local) {
            Utc::now()
        } else {
            now
        };
        let result = match binding.source() {
            LimitLoginStore::ClaudeCode => claude::parse(&body, at),
            LimitLoginStore::Codex | LimitLoginStore::OpenCode => codex::parse(&body, at),
        }
        .and_then(|snapshot| {
            if safe_snapshot(&snapshot, &credential) {
                Ok(snapshot)
            } else {
                Err(LimitFailure::ChangedResponse)
            }
        });
        LimitProbeResult {
            result,
            identity,
            retry_at: None,
        }
    }
    async fn credential(
        &self,
        binding: &LimitBinding,
        admission: &context::Admission,
        stop: &tokio_util::sync::CancellationToken,
    ) -> Result<credential::Credential, LimitFailure> {
        let guard = match self {
            Self::Recorded { root, .. } => Some(root.clone()),
            Self::Disabled | Self::Local => None,
        };
        let chosen = Arc::clone(&binding.0);
        #[cfg(target_os = "macos")]
        let keychain =
            if matches!(self, Self::Local) && chosen.local.source == LimitLoginStore::ClaudeCode {
                keychain::read(
                    &chosen.local.keychain_service,
                    &chosen.local.keychain_account,
                )
                .await
            } else {
                None
            };
        #[cfg(not(target_os = "macos"))]
        let keychain = None;
        let stop = stop.clone();
        let held = Arc::clone(admission);
        tokio::task::spawn_blocking(move || {
            let _held = held;
            if stop.is_cancelled() {
                return Err(credential::CredentialError::Cancelled);
            }
            match keychain {
                Some(credential) => Ok(credential),
                None => credential::read(&chosen.local.path, guard.as_deref(), chosen.local.source),
            }
        })
        .await
        .map_err(|_join| LimitFailure::UnsafeLoginStore)?
        .map_err(|error| error.failure())
    }
    async fn body(
        &self,
        context: &LimitContext,
        admission: &context::Admission,
        source: LimitLoginStore,
        credential: &credential::Credential,
        now: DateTime<Utc>,
        stop: &tokio_util::sync::CancellationToken,
    ) -> Result<zeroize::Zeroizing<Vec<u8>>, (LimitFailure, Option<DateTime<Utc>>)> {
        match self {
            Self::Disabled => Err((LimitFailure::Unsupported, None)),
            Self::Local => {
                // Certificate loading can block. Await its owner before
                // observing cancellation; only the actual HTTP is selected.
                let client = context
                    .client(admission)
                    .await
                    .map_err(|error| http::safe(&error))?;
                tokio::select! {
                    biased;
                    ()=stop.cancelled()=>Err((LimitFailure::Unsupported,None)),
                    result=http::read(client,source,credential)=>result,
                }
            }
            Self::Recorded {
                root,
                body,
                status,
                retry_after,
            } => {
                if stop.is_cancelled() {
                    return Err((LimitFailure::Unsupported, None));
                }
                let retry = retry_after
                    .as_deref()
                    .and_then(|text| http::retry_after(text, now));
                match *status {
                    401 | 403 => return Err((LimitFailure::Rejected, None)),
                    429 => return Err((LimitFailure::RateLimited, retry)),
                    200..=299 => {}
                    _ => return Err((LimitFailure::ChangedResponse, None)),
                }
                let root = root.clone();
                let path = body.clone();
                let held = Arc::clone(admission);
                let stopped = stop.clone();
                let body = tokio::task::spawn_blocking(move || {
                    let _held = held;
                    if stopped.is_cancelled() {
                        return Err((LimitFailure::Unsupported, None));
                    }
                    let actual = path
                        .canonicalize()
                        .map_err(|_private| (LimitFailure::UnsafeLoginStore, None))?;
                    let root = root
                        .canonicalize()
                        .map_err(|_private| (LimitFailure::UnsafeLoginStore, None))?;
                    if !actual.starts_with(root) {
                        return Err((LimitFailure::UnsafeLoginStore, None));
                    }
                    credential::bounded(&actual).map_err(|error| (error.failure(), None))
                })
                .await
                .map_err(|_join| (LimitFailure::UnsafeLoginStore, None))??;
                if stop.is_cancelled() {
                    return Err((LimitFailure::Unsupported, None));
                }
                Ok(body)
            }
        }
    }
}
fn failed(
    reason: LimitFailure,
    identity: Option<LimitLoginIdentity>,
    retry_at: Option<DateTime<Utc>>,
) -> LimitProbeResult {
    LimitProbeResult {
        result: Err(reason),
        identity,
        retry_at,
    }
}

// Check decoded fields too: JSON escaping can hide a credential from a raw
// byte search. Only these explicitly bounded presentation fields leave here.
fn safe_snapshot(snapshot: &LimitSnapshot, credential: &credential::Credential) -> bool {
    use vessel_proto::limits::LimitScope;
    let safe = |text: &str| {
        !text.contains(credential.token.as_str())
            && credential
                .account
                .as_ref()
                .is_none_or(|account| !text.contains(account.as_str()))
    };
    snapshot
        .plan
        .as_ref()
        .is_none_or(|plan| safe(plan.as_str()))
        && snapshot.windows.iter().all(|window| {
            safe(window.id.as_str())
                && match &window.scope {
                    LimitScope::General | LimitScope::CodeReview => true,
                    LimitScope::Model { id, name } | LimitScope::Surface { id, name } => {
                        id.as_ref().is_none_or(|id| safe(id.as_str())) && safe(name.as_str())
                    }
                    LimitScope::Feature { id, name } => safe(id.as_str()) && safe(name.as_str()),
                }
        })
}

#[cfg(test)]
mod failures;
#[cfg(test)]
mod tests;

#[cfg(test)]
mod context_tests;
#[cfg(test)]
mod resolution_tests;
