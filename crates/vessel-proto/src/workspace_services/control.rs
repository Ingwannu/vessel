//! Typed mutation outcomes do not substitute for admission or actual drain.

use serde::{Deserialize, Serialize};

use super::{ServiceUnavailable, ServicesAdminSettings};

/// Domain refusals contain no private parser/provider or process error text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ServiceRefusal {
    /// A displayed metadata snapshot no longer describes the current state.
    StaleSnapshot,
    /// The services slot no longer matches the expected presence/revision.
    StaleSettings,
    /// The canonical workspace names no such configured script.
    NoSuchScript,
    /// A service-only request targeted a plain script.
    NotService,
    /// The canonical lifecycle cannot admit work now.
    WorkspaceInactive,
    /// Source trust blocks automatic execution.
    AutomationBlocked,
    /// The project file could not provide a usable definition.
    InvalidConfiguration,
    /// The requested approval digest is no longer current/authorized.
    ApprovalRequired,
    /// Finite admission capacity is already held by actual owners.
    Busy,
    /// Missing prerequisites refuse rather than produce an empty success.
    Unavailable {
        /// A typed cause carries no submitted URL or command text.
        reason: ServiceUnavailable,
    },
    /// A future refusal cannot become a successful mutation.
    #[serde(other)]
    Unknown,
}

/// Persisted settings and settled ingress resources are different facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ServiceSettingsApplication {
    /// The write succeeded; actual ingress/relay owners are still reconciling.
    Applying,
    /// The required owned transition and drain completed.
    Applied,
    /// A missing runtime cannot be reported as a completed publication.
    Unavailable {
        /// The setting does not waive a missing canonical producer.
        reason: ServiceUnavailable,
    },
    /// Unknown application progress does not permit reuse.
    #[serde(other)]
    Unknown,
}

/// Failed writes preserve prior state; a caller timeout is not a drain receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ServiceSettingsOutcome {
    /// No write or revocation was necessary for identical input.
    Unchanged {
        /// The eligible direct Admin receives the current bounded projection.
        settings: ServicesAdminSettings,
    },
    /// Only the sole settings owner can report a committed private-slot write.
    Saved {
        /// The new settings CAS projection.
        settings: ServicesAdminSettings,
        /// Persistence alone cannot mean actual transition/drain completed.
        application: ServiceSettingsApplication,
    },
    /// A refused request cannot cause lookup/effects before authorization.
    Refused {
        /// A safe domain classification after principal/protocol admission.
        reason: ServiceRefusal,
    },
    /// An unknown reply never acknowledges a mutation or revocation.
    #[serde(other)]
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace_services::ServiceGateway;

    #[test]
    fn committed_applying_is_distinct_from_applied_or_refused() {
        let settings = ServicesAdminSettings::new(None, ServiceGateway::LocalOnly, vec![]).unwrap();
        let saved = ServiceSettingsOutcome::Saved {
            settings: settings.clone(),
            application: ServiceSettingsApplication::Applying,
        };
        let applied = ServiceSettingsOutcome::Saved {
            settings,
            application: ServiceSettingsApplication::Applied,
        };
        assert_ne!(saved, applied);
        let json = serde_json::to_string(&saved).unwrap();
        assert_eq!(
            serde_json::from_str::<ServiceSettingsOutcome>(&json).unwrap(),
            saved
        );
        assert_eq!(
            serde_json::from_str::<ServiceSettingsOutcome>(r#"{"kind":"future"}"#).unwrap(),
            ServiceSettingsOutcome::Unknown
        );
        assert_eq!(
            serde_json::from_str::<ServiceRefusal>(r#"{"kind":"future"}"#).unwrap(),
            ServiceRefusal::Unknown
        );
    }
}
