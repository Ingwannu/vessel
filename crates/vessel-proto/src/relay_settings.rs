//! Credential-free configured relay consent in the existing settings API.
//! The pending field version refuses mutations until the integrator assigns it.
//!
//! [Decision Log]
//! - Purpose: a client must distinguish disabled, stopping and unavailable.
//! - Constraint: routes, tokens, tickets and peer diagnostics are not settings
//!   reply values; callback registrations have an independent fleet lifetime.
//! - Alternatives: a URL-bearing config object or inferring state from an offer.
//! - Choice: closed choices and public health only, behind one pending constant.
//! - Why: unknown support cannot accidentally grant consent or mean disabled.
//! - Cost: a client must reread state after an uncertain change, never retry it.
use crate::ProtocolVersion;
use serde::{Deserialize, Serialize};

/// Assigned once by the integrator when the field is wired.
pub const RELAY_SETTINGS_PROTOCOL: Option<ProtocolVersion> = None;

/// An explicit remote choice, never routes or credentials.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum RelayChange {
    /// Explicitly consent to the operator-configured registrations.
    Enabled,
    /// Stop configured registrations, preserving other connection lifetimes.
    Disabled,
    /// A future choice never grants consent implicitly.
    #[serde(other)]
    Unknown,
}
/// Aggregate health of the configured registrations, never remote diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum RelayHealth {
    /// Connecting or registering with the configured relays.
    Connecting,
    /// At least one configured relay reports a registered link.
    Online,
    /// No relay is registered and a refused registration is being retried.
    Retrying,
    /// Every configured registration task has stopped.
    Stopped,
    /// A newer node's aggregate status.
    #[serde(other)]
    Unknown,
}
/// A configured generation cannot be both disabled and still stopping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum ConfiguredRelays {
    /// The operator supplied no configured relay routes.
    Unconfigured,
    /// Configured registrations are disabled and no task is still stopping.
    Disabled,
    /// Disable was requested but registration tasks have not all completed.
    Disabling,
    /// Configured registrations are enabled; health is not remote reachability.
    Enabled {
        /// Only aggregate registration health, with no remote diagnostics.
        health: RelayHealth,
    },
    /// A newer state, never interpreted as disabled or as consent.
    #[serde(other)]
    Unknown,
}
/// The separate fleet relationship may keep its callback registration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum CallbackRelays {
    /// No callback relationship is retained; this is not data-connection closure.
    Absent,
    /// An independent fleet callback relationship retains its registrations.
    Present,
    /// A newer node's callback relationship status.
    #[serde(other)]
    Unknown,
}
/// This read value has no field capable of carrying a route or secret.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct RelaySettings {
    /// Only the operator-configured registration lifetime.
    pub configured: ConfiguredRelays,
    /// Fleet callbacks have independent authority and lifetime.
    pub fleet_callback: CallbackRelays,
}
impl RelaySettings {
    pub(crate) fn for_protocol(
        self,
        protocol: ProtocolVersion,
        assigned: Option<ProtocolVersion>,
    ) -> Option<Self> {
        assigned
            .filter(|required| protocol >= *required)
            .map(|_| self)
    }
}
/// Stable recovery codes; Busy uses Conflict, never a new top-level error code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum RelaySettingsError {
    /// The operator supplied no route this choice could enable.
    #[error("this node has no configured relay")]
    NotConfigured,
    /// The node is shutting down and accepts no new change.
    #[error("this node is stopping; relay settings cannot be changed")]
    Stopping,
    /// A prior mutation or stopping generation still owns the bounded work.
    #[error("another relay change is still in progress; read its status")]
    Busy,
    /// The outcome is not confirmed; inspect status instead of replaying.
    #[error("could not confirm the relay change; read its status before trying again")]
    Uncertain,
    /// A newer node's recovery category.
    #[serde(other)]
    #[error("this relay settings choice is not supported")]
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_state_round_trips_without_route_or_credential_fields() {
        for configured in [
            ConfiguredRelays::Unconfigured,
            ConfiguredRelays::Disabled,
            ConfiguredRelays::Disabling,
            ConfiguredRelays::Unknown,
            ConfiguredRelays::Enabled {
                health: RelayHealth::Connecting,
            },
            ConfiguredRelays::Enabled {
                health: RelayHealth::Online,
            },
            ConfiguredRelays::Enabled {
                health: RelayHealth::Retrying,
            },
            ConfiguredRelays::Enabled {
                health: RelayHealth::Stopped,
            },
            ConfiguredRelays::Enabled {
                health: RelayHealth::Unknown,
            },
        ] {
            for fleet_callback in [
                CallbackRelays::Absent,
                CallbackRelays::Present,
                CallbackRelays::Unknown,
            ] {
                let value = RelaySettings {
                    configured,
                    fleet_callback,
                };
                let text = serde_json::to_string(&value).unwrap();
                assert_eq!(serde_json::from_str::<RelaySettings>(&text).unwrap(), value);
                let fields = serde_json::to_value(value).unwrap();
                let names: Vec<_> = fields
                    .as_object()
                    .unwrap()
                    .keys()
                    .map(String::as_str)
                    .collect();
                assert_eq!(names, ["configured", "fleet_callback"]);
                assert!(!format!("{value:?}").contains("ws://"));
            }
        }
    }
    #[test]
    fn future_choices_are_unknown_never_implicit_consent_or_disabled() {
        assert_eq!(
            serde_json::from_str::<RelayChange>("\"a_future_choice\"").unwrap(),
            RelayChange::Unknown
        );
        assert_eq!(
            serde_json::from_str::<ConfiguredRelays>(r#"{"state":"future"}"#).unwrap(),
            ConfiguredRelays::Unknown
        );
        for choice in [
            RelayChange::Enabled,
            RelayChange::Disabled,
            RelayChange::Unknown,
        ] {
            assert_eq!(
                serde_json::from_str::<RelayChange>(&serde_json::to_string(&choice).unwrap())
                    .unwrap(),
                choice
            );
        }
        for reason in [
            RelaySettingsError::NotConfigured,
            RelaySettingsError::Stopping,
            RelaySettingsError::Busy,
            RelaySettingsError::Uncertain,
            RelaySettingsError::Unknown,
        ] {
            assert_eq!(
                serde_json::from_str::<RelaySettingsError>(
                    &serde_json::to_string(&reason).unwrap()
                )
                .unwrap(),
                reason
            );
        }
        assert_eq!(
            serde_json::from_str::<RelaySettingsError>("\"future\"").unwrap(),
            RelaySettingsError::Unknown
        );
    }
    #[test]
    fn pending_and_older_readers_cannot_receive_an_unassigned_field() {
        let state = RelaySettings {
            configured: ConfiguredRelays::Disabled,
            fleet_callback: CallbackRelays::Present,
        };
        let boundary = crate::PROTOCOL_VERSION;
        let older = ProtocolVersion::new(boundary.get().checked_sub(1).unwrap());
        assert_eq!(state.for_protocol(older, Some(boundary)), None);
        assert_eq!(state.for_protocol(boundary, Some(boundary)), Some(state));
        assert_eq!(
            state.for_protocol(ProtocolVersion::new(u32::MAX), RELAY_SETTINGS_PROTOCOL),
            RELAY_SETTINGS_PROTOCOL.map(|_assigned| state)
        );
    }
}

#[cfg(test)]
mod settings_tests;
