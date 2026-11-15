//! Typed whole-field configuration keys (ADR 0064).
//! No file I/O, launch policy or independently maintained CLI key list.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use super::{AgentDrivers, NodeSettings, SetNodeSettings};
use crate::ProtocolVersion;
use crate::parking::ParkAfter;

macro_rules! setting_keys {
    ($($(#[$doc:meta])* $variant:ident: $field:ident => $value:ty [$class:ident; $probe:expr]),+ $(,)?) => {
        /// One top-level setting, shared by wire operations and CLI parsing.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
        #[cfg_attr(feature = "ts", derive(ts_rs::TS))]
        #[serde(rename_all = "snake_case")]
        pub enum NodeSettingKey {
            $($(#[$doc])* $variant,)+
            /// A newer peer's key may be displayed but never changed here.
            #[serde(other)]
            Unknown,
        }

        /// Borrow a typed field for output, without an untyped domain tree.
        #[derive(Debug, PartialEq, Eq, Serialize)]
        #[serde(untagged)]
        pub enum SettingValue<'a> {
            $($(#[$doc])* $variant(&'a $value),)+
        }

        impl NodeSettingKey {
            /// Known keys in reporting order; one declaration owns the map.
            pub const ALL: &'static [Self] = &[$(Self::$variant,)+];

            /// The whole-field name, never an invented dotted path.
            #[must_use]
            pub const fn as_str(self) -> &'static str {
                match self { $(Self::$variant => stringify!($field),)+ Self::Unknown => "unknown" }
            }

            /// Read through the shared type, without a second CLI key map.
            ///
            /// # Errors
            /// A newer key cannot be interpreted by this build.
            pub fn value(self, settings: &NodeSettings) -> Result<SettingValue<'_>, UnknownSettingKey> {
                // Startup recovery is value-only, with no patch key (ADR 0059).
                let NodeSettings { $($field,)+ restart_recovery: _ } = settings;
                match self { $(Self::$variant => Ok(SettingValue::$variant($field)),)+ Self::Unknown => Err(UnknownSettingKey::Unknown) }
            }

            /// Decode one complete field before a client can send its patch.
            ///
            /// # Errors
            /// Unknown keys/choices, malformed input and extra fields refuse.
            pub fn change(self, text: &str) -> Result<SetNodeSettings, super::input::InvalidSettingInput> {
                let change = match self {
                    $(Self::$variant => SetNodeSettings { $field: Some(super::input::field(text.as_bytes())?), ..SetNodeSettings::default() },)+
                    Self::Unknown => return Err(super::input::InvalidSettingInput::Unknown),
                };
                // Exhaustive patch coverage keeps future fields in the shared key map.
                let SetNodeSettings { $($field: _,)+ } = &change;
                super::input::known(change)
            }

            /// Remove only this saved whole-field override.
            ///
            /// # Errors
            /// Unknown keys cannot erase data this build does not understand.
            pub fn clear(self, saved: &mut SetNodeSettings) -> Result<(), UnknownSettingKey> {
                let SetNodeSettings { $($field,)+ } = saved;
                match self { $(Self::$variant => *$field = None,)+ Self::Unknown => return Err(UnknownSettingKey::Unknown) }
                Ok(())
            }

            /// Report an effective change in the field owner's declared class.
            ///
            /// # Errors
            /// Unknown keys cannot be treated as applied changes.
            pub fn report_change(self, before: &NodeSettings, after: &NodeSettings, report: &mut ReloadedNodeSettings) -> Result<(), UnknownSettingKey> {
                match self {
                    $(Self::$variant => { if before.$field != after.$field { report.$class.push(self); } },)+
                    Self::Unknown => return Err(UnknownSettingKey::Unknown),
                }
                Ok(())
            }

            /// Reuse the existing patch's field-version rules for reads.
            #[must_use]
            pub fn required_protocol(self) -> Option<ProtocolVersion> {
                match self {
                    $(Self::$variant => crate::Call::SetNodeSettings(SetNodeSettings {
                        $field: Some($probe), ..SetNodeSettings::default()
                    }).since(),)+
                    Self::Unknown => None,
                }
            }

        }

        impl FromStr for NodeSettingKey {
            type Err = UnknownSettingKey;
            fn from_str(text: &str) -> Result<Self, Self::Err> {
                match text { $(stringify!($field) => Ok(Self::$variant),)+ _ => Err(UnknownSettingKey::of(text)) }
            }
        }
    };
}

setting_keys! {
    /// Idle-agent parking; applied to the running runtime (ADR 0038).
    ParkAfter: park_after => ParkAfter [applied; ParkAfter::Never],
    /// Agent protocols; applied to new sessions (ADR 0057).
    Drivers: drivers => AgentDrivers [applied; super::SetAgentDrivers::default()],
}

impl fmt::Display for NodeSettingKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Refuse unsupported input without treating launch flags as saved keys.
#[derive(Debug, Clone, Copy, thiserror::Error)]
pub enum UnknownSettingKey {
    /// Listener changes need a new process.
    #[error("bind is a launch setting; use --bind on the next start")]
    Bind,
    /// The node name belongs to its launch.
    #[error("name is a launch setting; use --name on the next start")]
    Name,
    /// Relay configuration belongs to its launch.
    #[error("relay is a launch setting; use --relay on the next start")]
    Relay,
    /// Trusted proxy configuration belongs to its launch.
    #[error("trusted_proxy is a launch setting; use --trusted-proxy on the next start")]
    TrustedProxy,
    /// Handshake timing belongs to its launch.
    #[error(
        "agent handshake timing is a launch setting; use --agent-handshake-timeout on the next start"
    )]
    HandshakeTimeout,
    /// LSP permission roots belong to the operator's launch.
    #[error("LSP configuration is a launch setting; use the --lsp-* flags on the next start")]
    Lsp,
    /// No caller-chosen name is interpolated into this refusal.
    #[error("unknown saved setting; use a supported whole field")]
    Unknown,
}

impl UnknownSettingKey {
    fn of(text: &str) -> Self {
        match text {
            "bind" => Self::Bind,
            "name" => Self::Name,
            "relay" => Self::Relay,
            "trusted_proxy" => Self::TrustedProxy,
            "handshake_timeout" | "agent_handshake_timeout" => Self::HandshakeTimeout,
            "lsp_config" | "lsp_roots" | "lsp_full_roots" => Self::Lsp,
            _ => Self::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parking::ParkAfter;

    #[test]
    fn known_keys_round_trip_and_project_only_the_selected_field() {
        let settings = NodeSettings {
            park_after: ParkAfter::Never,
            drivers: AgentDrivers::default(),
            restart_recovery: crate::restart::RestartRecovery::default(),
        };
        for key in NodeSettingKey::ALL {
            assert_eq!(key.as_str().parse::<NodeSettingKey>().unwrap(), *key);
            let json = serde_json::to_string(&key.value(&settings).unwrap()).unwrap();
            let change = key.change(&json).unwrap();
            assert_eq!(
                key.required_protocol(),
                crate::Call::SetNodeSettings(change).since()
            );
        }
        let change = NodeSettingKey::ParkAfter
            .change(r#"{"kind":"never"}"#)
            .unwrap();
        assert_eq!(change.park_after, Some(ParkAfter::Never));
        assert_eq!(change.drivers, None);
        assert_eq!(NodeSettingKey::Unknown.required_protocol(), None);
        assert!(NodeSettingKey::Unknown.change("{}").is_err());
        assert!(NodeSettingKey::Unknown.value(&settings).is_err());
        assert!("drivers.codex".parse::<NodeSettingKey>().is_err());
        assert!(
            "bind"
                .parse::<NodeSettingKey>()
                .unwrap_err()
                .to_string()
                .contains("--bind")
        );
    }
}

/// A validated reset set; no empty, duplicated or unknown key can reach a write.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(transparent)]
pub struct SettingKeys(Vec<NodeSettingKey>);

impl SettingKeys {
    /// Every reset must name a setting, rather than silently doing nothing.
    pub const MINIMUM: usize = 1;
    /// The known key declaration also bounds a reset request.
    pub const MAXIMUM: usize = NodeSettingKey::ALL.len();

    /// Validate the entire reset before its first in-memory or persisted change.
    ///
    /// # Errors
    /// The set is empty, too long, duplicated or contains a newer unknown key.
    pub fn new(keys: Vec<NodeSettingKey>) -> Result<Self, InvalidSettingKeys> {
        if keys.len() < Self::MINIMUM || keys.len() > Self::MAXIMUM {
            return Err(InvalidSettingKeys::Count);
        }
        let mut seen = Vec::new();
        for key in &keys {
            if *key == NodeSettingKey::Unknown {
                return Err(InvalidSettingKeys::Unknown);
            }
            if seen.contains(key) {
                return Err(InvalidSettingKeys::Repeated);
            }
            seen.push(*key);
        }
        Ok(Self(keys))
    }

    /// Borrow only the already-validated keys while constructing the replacement.
    #[must_use]
    pub fn iter(&self) -> impl Iterator<Item = NodeSettingKey> + '_ {
        self.0.iter().copied()
    }
}

impl<'de> Deserialize<'de> for SettingKeys {
    fn deserialize<D: serde::Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        struct Keys;
        impl<'de> serde::de::Visitor<'de> for Keys {
            type Value = SettingKeys;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(
                    f,
                    "between {} and {} distinct known settings",
                    SettingKeys::MINIMUM,
                    SettingKeys::MAXIMUM
                )
            }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                mut sequence: A,
            ) -> Result<Self::Value, A::Error> {
                // Never allocate from the untrusted sequence's size hint.
                let mut keys = Vec::with_capacity(SettingKeys::MAXIMUM);
                while let Some(key) = sequence.next_element::<NodeSettingKey>()? {
                    if keys.len() >= SettingKeys::MAXIMUM {
                        return Err(serde::de::Error::custom(InvalidSettingKeys::Count));
                    }
                    keys.push(key);
                }
                SettingKeys::new(keys).map_err(serde::de::Error::custom)
            }
        }
        decoder.deserialize_seq(Keys)
    }
}

/// A reset refusal keeps arbitrary key text out of diagnostics.
#[derive(Debug, Clone, Copy, thiserror::Error)]
pub enum InvalidSettingKeys {
    /// The count is outside the key set's one named range.
    #[error(
        "reset must name between {} and {} settings",
        SettingKeys::MINIMUM,
        SettingKeys::MAXIMUM
    )]
    Count,
    /// A repeated key likely signals a caller mistake.
    #[error("reset repeats a setting")]
    Repeated,
    /// Never erase a key this build cannot interpret.
    #[error("reset names an unknown setting")]
    Unknown,
}

/// Remove saved overrides; unlike a patch's absent field, this means reset.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct UnsetNodeSettings {
    /// Validated whole fields to remove together.
    pub keys: SettingKeys,
}

/// Re-read saved configuration without changing the process's launch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ReloadNodeSettings {}

/// Report application honestly, preserving start-time and pinned classifications.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ReloadedNodeSettings {
    /// Changed saved values applied to the runtime.
    pub applied: Vec<NodeSettingKey>,
    /// Values the process can apply only at its next launch.
    pub restart_required: Vec<NodeSettingKey>,
    /// Values controlled by explicit launch overrides.
    pub override_controlled: Vec<NodeSettingKey>,
}

#[cfg(test)]
mod reset_tests {
    use super::*;
    use crate::{Call, Method, MethodShape, Request as _, Scope};

    #[test]
    fn reset_keys_refuse_empty_duplicate_unknown_and_excess_before_work() {
        assert!(matches!(
            SettingKeys::new(Vec::new()),
            Err(InvalidSettingKeys::Count)
        ));
        assert!(matches!(
            SettingKeys::new(vec![NodeSettingKey::Unknown]),
            Err(InvalidSettingKeys::Unknown)
        ));
        let repeated = vec![NodeSettingKey::ParkAfter; SettingKeys::MAXIMUM];
        assert!(matches!(
            SettingKeys::new(repeated),
            Err(InvalidSettingKeys::Repeated)
        ));
        let excessive = vec![NodeSettingKey::ParkAfter; SettingKeys::MAXIMUM.saturating_add(1)];
        let error = SettingKeys::new(excessive.clone()).unwrap_err();
        assert!(
            error
                .to_string()
                .contains(&SettingKeys::MINIMUM.to_string())
        );
        assert!(
            error
                .to_string()
                .contains(&SettingKeys::MAXIMUM.to_string())
        );
        assert!(
            serde_json::from_str::<SettingKeys>(&serde_json::to_string(&excessive).unwrap())
                .is_err()
        );
        let keys = SettingKeys::new(NodeSettingKey::ALL.to_vec()).unwrap();
        assert_eq!(keys.iter().collect::<Vec<_>>(), NodeSettingKey::ALL);
    }

    #[test]
    fn config_methods_are_admin_and_unassigned_at_both_admission_accessors() {
        let keys = SettingKeys::new(vec![NodeSettingKey::ParkAfter]).unwrap();
        for call in [
            UnsetNodeSettings { keys }.into_call(),
            ReloadNodeSettings {}.into_call(),
        ] {
            let method = call.method();
            assert_eq!(method.scope(), Some(Scope::Admin));
            assert_eq!(method.shape(), MethodShape::Unary);
            assert_eq!(method.since(), super::super::NODE_CONFIG_PROTOCOL);
            assert_eq!(call.since(), super::super::NODE_CONFIG_PROTOCOL);
            assert_eq!(call.required_protocol(), super::super::NODE_CONFIG_PROTOCOL);
            let bytes = serde_json::to_vec(&call).unwrap();
            assert_eq!(serde_json::from_slice::<Call>(&bytes).unwrap(), call);
        }
        assert_eq!(Method::ReloadNodeSettings.as_str(), "node.reload");
    }

    #[test]
    fn reset_reply_still_projects_other_unassigned_settings_fields() {
        let settings: NodeSettings = serde_json::from_str(r#"{"park_after":{"kind":"never"},"restart_recovery":{"enabled":true,"prompt":"hidden-recovery-canary"}}"#).unwrap();
        let reply = crate::Reply::UnsetNodeSettings(settings);
        let projected = reply.for_protocol(crate::PROTOCOL_VERSION).unwrap_or(reply);
        let visible = crate::restart::RESTART_RECOVERY_PROTOCOL
            .is_some_and(|required| crate::PROTOCOL_VERSION >= required);
        assert_eq!(
            serde_json::to_string(&projected)
                .unwrap()
                .contains("hidden-recovery-canary"),
            visible
        );
    }
}
