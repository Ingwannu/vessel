//! Provider/account association values, not evidence or authority.
//! The lifecycle actor owns the actual resolved launch and run attestation.
//! These values alone never join a configured provider to a legacy account.

use crate::{HarnessRef, ProfileName};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::ProviderId;

/// An explicit resolved launch selection supplied by its node owner.
/// Deliberately non-serializable: a client cannot submit this as an attestation.
/// It is still data; the actor must bind it to the actual ProcessLifetime/run,
/// immutable launch context and before/after options fence before projecting it.
#[derive(Clone, PartialEq, Eq)]
pub struct ProviderLaunchAssociation {
    provider: ProviderId,
    harness: HarnessRef,
    profile: ProfileName,
}

impl ProviderLaunchAssociation {
    /// Record only explicit resolved launch values. `profile` must be the actual
    /// Profile.name(), including a real implicit-default Profile when present.
    /// Never fill SessionInfo.profile=None with a guessed default name; an
    /// uncaptured profile cannot construct an attested association.
    #[must_use]
    pub const fn new(provider: ProviderId, harness: HarnessRef, profile: ProfileName) -> Self {
        Self {
            provider,
            harness,
            profile,
        }
    }

    /// Explicit selected provider, never inferred from implementation family.
    #[must_use]
    pub const fn provider(&self) -> &ProviderId {
        &self.provider
    }

    /// The actual launch's resolved harness identity.
    #[must_use]
    pub const fn harness(&self) -> &HarnessRef {
        &self.harness
    }

    /// The actual launch's resolved profile name.
    #[must_use]
    pub const fn profile(&self) -> &ProfileName {
        &self.profile
    }
}

impl std::fmt::Debug for ProviderLaunchAssociation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ProviderLaunchAssociation(<private launch selection>)")
    }
}

/// Reader-only association. No request accepts this as proof of account access.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum ProviderBinding {
    /// No unique current launch/profile attestation is available. This also
    /// covers stale/conflicting generations; never select an arbitrary live run.
    Unobserved,
    /// The actor validated the exact launch association and run before projection.
    Bound {
        /// The actual resolved identity, not the provider's implementation kind.
        harness: HarnessRef,
        /// The actual resolved Profile.name(), not a normalized SessionInfo guess.
        profile: ProfileName,
        /// Only an observation attested to this same association/run.
        sign_in: ProviderSignIn,
    },
}

/// A safe sign-in projection with no credential path, help command or account id.
/// A timestamp alone is not an attestation; that evidence stays with the actor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum ProviderSignIn {
    /// Not observed for this exact association/run. A successful catalogue
    /// handshake, another profile's latest status or credential existence is not proof.
    Unknown,
    /// An actual observation bound to the same run/context was retained.
    Observed {
        /// What that observation reported, without extrapolation.
        verdict: ProviderSignInVerdict,
        /// Observation time, never a synthetic creation/current timestamp.
        checked_at: DateTime<Utc>,
    },
}

/// Closed reported facts, not a promise that a future billable turn authenticates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum ProviderSignInVerdict {
    /// The provider status explicitly reports signed in; token validity remains
    /// the provider's to check on actual use.
    ReportedSignedIn,
    /// The actual status/handshake explicitly reports authentication required.
    SignInRequired,
    /// The actual launch policy explicitly needs no provider sign-in.
    NotRequired,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn association_retains_only_explicit_resolved_values_and_redacts_debug() {
        let association = ProviderLaunchAssociation::new(
            "custom-codex".parse().unwrap(),
            HarnessRef::Added("opaque-driver".parse().unwrap()),
            "resolved-profile".parse().unwrap(),
        );
        assert_eq!(association.provider().as_str(), "custom-codex");
        assert_eq!(
            association.harness(),
            &HarnessRef::Added("opaque-driver".parse().unwrap())
        );
        assert_eq!(association.profile().as_str(), "resolved-profile");
        let debug = format!("{association:?}");
        for private in ["custom-codex", "opaque-driver", "resolved-profile"] {
            assert!(!debug.contains(private));
        }
    }

    #[test]
    fn unknown_and_observed_verdicts_round_trip_without_a_default_account_guess() {
        let unknown = ProviderBinding::Unobserved;
        assert_eq!(
            serde_json::to_string(&unknown).unwrap(),
            r#"{"state":"unobserved"}"#
        );
        for sign_in in [
            ProviderSignIn::Unknown,
            ProviderSignIn::Observed {
                verdict: ProviderSignInVerdict::ReportedSignedIn,
                checked_at: DateTime::UNIX_EPOCH,
            },
            ProviderSignIn::Observed {
                verdict: ProviderSignInVerdict::SignInRequired,
                checked_at: DateTime::UNIX_EPOCH,
            },
            ProviderSignIn::Observed {
                verdict: ProviderSignInVerdict::NotRequired,
                checked_at: DateTime::UNIX_EPOCH,
            },
        ] {
            let bound = ProviderBinding::Bound {
                harness: HarnessRef::Added("pi".parse().unwrap()),
                profile: "captured-legacy-profile".parse().unwrap(),
                sign_in,
            };
            let encoded = serde_json::to_string(&bound).unwrap();
            assert_eq!(
                serde_json::from_str::<ProviderBinding>(&encoded).unwrap(),
                bound
            );
        }
        assert!(
            serde_json::from_str::<ProviderBinding>(
                r#"{"state":"bound","harness":"codex","sign_in":{"state":"unknown"}}"#
            )
            .is_err()
        );
    }
}
