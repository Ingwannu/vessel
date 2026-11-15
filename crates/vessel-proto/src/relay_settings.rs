//! Staged, credential-free relay consent values (ADR 0096). Wiring moves
//! these into the public settings API and removes the test-only boundary.
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
pub(crate) const RELAY_SETTINGS_PROTOCOL: Option<ProtocolVersion> = None;

/// An explicit remote choice, never routes or credentials.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RelayChange {
    Enabled,
    Disabled,
    #[serde(other)]
    Unknown,
}
/// Aggregate health of the configured registrations, never remote diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RelayHealth {
    Connecting,
    Online,
    Retrying,
    Stopped,
    #[serde(other)]
    Unknown,
}
/// A configured generation cannot be both disabled and still stopping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub(crate) enum ConfiguredRelays {
    Unconfigured,
    Disabled,
    Disabling,
    Enabled {
        health: RelayHealth,
    },
    #[serde(other)]
    Unknown,
}
/// The separate fleet relationship may keep its callback registration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CallbackRelays {
    Absent,
    Present,
    #[serde(other)]
    Unknown,
}
/// This read value has no field capable of carrying a route or secret.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct RelaySettings {
    pub(crate) configured: ConfiguredRelays,
    pub(crate) fleet_callback: CallbackRelays,
}
impl RelaySettings {
    fn for_protocol(
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RelaySettingsError {
    NotConfigured,
    Stopping,
    Busy,
    Uncertain,
    #[serde(other)]
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
        let boundary = ProtocolVersion::new(25); // Synthetic boundary, no protocol reservation.
        assert_eq!(
            state.for_protocol(ProtocolVersion::new(24), Some(boundary)),
            None
        );
        assert_eq!(state.for_protocol(boundary, Some(boundary)), Some(state));
        assert_eq!(
            state.for_protocol(ProtocolVersion::new(u32::MAX), RELAY_SETTINGS_PROTOCOL),
            None
        );
    }
}
