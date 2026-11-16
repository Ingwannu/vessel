//! Additional sources share the reviewed local-read, HTTPS, admission and cooldown boundaries.
mod access;
mod credential;
use super::{LimitContext, context::TokenKey};
use crate::{AccountUsageOutcome, decode_account_usage};
pub use access::AccountSourceAccess;
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use std::{fmt, path::PathBuf, sync::Arc};
use tokio_util::sync::CancellationToken;
use vessel_proto::{limits::LimitFailure, usage_reports::BuiltinUsageSource};
/// Private same-sign-in comparison proof; never a public id, log or database value.
#[derive(Clone, PartialEq, Eq)]
pub struct AccountUsageIdentity([u8; 32]);
impl fmt::Debug for AccountUsageIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("AccountUsageIdentity([redacted])")
    }
}
/// Already sanitized source observation with private same-login provenance and actual cooldown.
#[derive(Debug)]
pub struct AccountUsageObservation {
    /// Parsed report or safe local/network refusal.
    pub result: Result<AccountUsageOutcome, LimitFailure>,
    /// Missing proof forbids retaining an earlier success.
    pub identity: Option<AccountUsageIdentity>,
    /// Vendor Retry-After, independent of the successful observation time.
    pub retry_at: Option<DateTime<Utc>>,
}
/// Recorded bodies normally exercise the real vendor parser; explicit fakes may seed generic values.
#[derive(Debug, Clone, Copy, Default)]
pub enum RecordedUsageFormat {
    /// A real vendor-shaped fixture, decoded by its one owner parser.
    #[default]
    Vendor,
    /// Test-only source values for forecasts/balances not supplied by every built-in vendor.
    #[cfg(feature = "fakes")]
    Report,
}
/// Tests default to Disabled; only a binary supplies live read capabilities.
#[derive(Clone, Debug, Default)]
pub enum AccountUsageProbe {
    /// No login, environment credential, certificate or vendor access.
    #[default]
    Disabled,
    /// Fixed built-in vendors and the owning machine's read-only sign-ins.
    Local(AccountSourceAccess),
    /// Synthetic sign-in and body inside a temporary root; no HTTP or certificates.
    Recorded {
        /// File and database candidates must canonicalize below this fixture root.
        root: PathBuf,
        /// Synthetic owning-account capabilities.
        access: AccountSourceAccess,
        /// Exact fixture source; other sources remain Unsupported.
        source: BuiltinUsageSource,
        /// Vendor-shaped recorded bytes within root.
        body: PathBuf,
        /// Recorded HTTP status; raw vendor errors are never returned.
        status: u16,
        /// Recorded Retry-After, if any.
        retry_after: Option<String>,
        /// Never selected by installed node arguments; generic report fixtures require fakes.
        format: RecordedUsageFormat,
    },
}
impl AccountUsageProbe {
    /// Unsupported sources never advertise working consent/refresh controls.
    #[must_use]
    pub fn supports(&self, source: BuiltinUsageSource) -> bool {
        if matches!(
            source,
            BuiltinUsageSource::Claude | BuiltinUsageSource::Codex
        ) {
            return false;
        }
        match self {
            Self::Disabled => false,
            Self::Local(_) => !cfg!(feature = "fakes"),
            Self::Recorded {
                source: recorded, ..
            } => *recorded == source,
        }
    }
    /// Read one explicitly consented source. Cancellation retains admission through blocking work.
    pub async fn read(
        &self,
        context: &LimitContext,
        source: BuiltinUsageSource,
        now: DateTime<Utc>,
        stop: &CancellationToken,
    ) -> AccountUsageObservation {
        if stop.is_cancelled() || !self.supports(source) {
            return failed(LimitFailure::Unsupported, None, None);
        }
        let admission = match context.admit() {
            Ok(held) => held,
            Err(reason) => return failed(reason, None, None),
        };
        let credential = match self.credential(&admission, source, stop).await {
            Ok(credential) => credential,
            Err(reason) => return failed(reason, None, None),
        };
        let mut hash = Sha256::new();
        hash.update(credential.token.as_bytes());
        hash.update([0]);
        hash.update(credential.region.identity());
        let identity = Some(AccountUsageIdentity(hash.finalize().into()));
        if credential.expires.is_some_and(|expires| expires <= now) {
            return failed(LimitFailure::Expired, identity, None);
        }
        if stop.is_cancelled() {
            return failed(LimitFailure::Unsupported, identity, None);
        }
        let body = match self
            .checked_body(context, &admission, source, &credential, now, stop)
            .await
        {
            Ok(body) => body,
            Err((reason, retry)) => return failed(reason, identity, retry),
        };
        let result = self.decode(source, &body).and_then(|outcome| {
            if safe(&outcome, &credential.token) {
                Ok(outcome)
            } else {
                Err(LimitFailure::ChangedResponse)
            }
        });
        AccountUsageObservation {
            result,
            identity,
            retry_at: None,
        }
    }
    fn decode(
        &self,
        source: BuiltinUsageSource,
        body: &[u8],
    ) -> Result<AccountUsageOutcome, LimitFailure> {
        #[cfg(feature = "fakes")]
        if matches!(
            self,
            Self::Recorded {
                format: RecordedUsageFormat::Report,
                ..
            }
        ) {
            if body.len() > crate::MAX_USAGE_VENDOR_BYTES {
                return Err(LimitFailure::ChangedResponse);
            }
            return serde_json::from_slice(body)
                .map(AccountUsageOutcome::Available)
                .map_err(|_private| LimitFailure::ChangedResponse);
        }
        decode_account_usage(source, body).map_err(|_private| LimitFailure::ChangedResponse)
    }
    async fn credential(
        &self,
        admission: &super::context::Admission,
        source: BuiltinUsageSource,
        stop: &CancellationToken,
    ) -> Result<credential::Credential, LimitFailure> {
        let (access, guard) = match self {
            Self::Local(access) => (access.clone(), None),
            Self::Recorded { access, root, .. } => (access.clone(), Some(root.clone())),
            Self::Disabled => return Err(LimitFailure::Unsupported),
        };
        let held = Arc::clone(admission);
        let cancelled = stop.clone();
        let found = tokio::task::spawn_blocking(move || {
            let _held = held;
            credential::read(&access, source, guard.as_deref(), &cancelled)
        })
        .await;
        match found {
            Ok(result) => result,
            Err(_task) => Err(LimitFailure::UnsafeLoginStore),
        }
    }
    async fn checked_body(
        &self,
        context: &LimitContext,
        admission: &super::context::Admission,
        source: BuiltinUsageSource,
        credential: &credential::Credential,
        now: DateTime<Utc>,
        stop: &CancellationToken,
    ) -> Result<zeroize::Zeroizing<Vec<u8>>, (LimitFailure, Option<DateTime<Utc>>)> {
        let lease = context
            .check(TokenKey::account(source, &credential.token), now)
            .map_err(|refusal| (refusal.reason, refusal.retry_at))?;
        let result = self
            .body(context, admission, source, credential, now, stop)
            .await;
        match result {
            Ok(body) => {
                lease.finish(None);
                Ok(body)
            }
            Err((reason, retry)) => {
                lease.finish(if reason == LimitFailure::RateLimited {
                    retry
                } else {
                    None
                });
                Err((reason, retry))
            }
        }
    }
}
fn failed(
    reason: LimitFailure,
    identity: Option<AccountUsageIdentity>,
    retry_at: Option<DateTime<Utc>>,
) -> AccountUsageObservation {
    AccountUsageObservation {
        result: Err(reason),
        identity,
        retry_at,
    }
}
fn safe(outcome: &AccountUsageOutcome, token: &str) -> bool {
    let AccountUsageOutcome::Available(report) = outcome else {
        return true;
    };
    let safe = |text: &str| !text.contains(token);
    report.plan.as_ref().is_none_or(|plan| safe(plan.as_str()))
        && report.windows.as_slice().iter().all(|window| {
            safe(window.id.as_str())
                && safe(window.label.as_str())
                && match &window.short_label {
                    Some(vessel_proto::usage_reports::UsageShortLabel::Text(label)) => {
                        safe(label.as_str())
                    }
                    Some(vessel_proto::usage_reports::UsageShortLabel::Hidden) | None => true,
                }
        })
        && report
            .balances
            .as_slice()
            .iter()
            .all(|balance| safe(balance.id.as_str()) && safe(balance.label.as_str()))
        && report.details.as_slice().iter().all(|detail| {
            safe(detail.id.as_str()) && safe(detail.label.as_str()) && safe(detail.value.as_str())
        })
}
mod body;
#[cfg(test)]
mod tests;
