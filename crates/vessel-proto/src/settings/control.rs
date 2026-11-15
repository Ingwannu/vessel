//! Typed whole-field configuration keys (ADR 0064).
//! No file I/O, launch policy or independently maintained CLI key list.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use super::{AgentDrivers, NodeSettings, SetNodeSettings};
use crate::ProtocolVersion;
use crate::parking::ParkAfter;

macro_rules! setting_keys {
    ($($(#[$doc:meta])* $variant:ident: $field:ident => $value:ty [$probe:expr]),+ $(,)?) => {
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
    ParkAfter: park_after => ParkAfter [ParkAfter::Never],
    /// Agent protocols; applied to new sessions (ADR 0057).
    Drivers: drivers => AgentDrivers [super::SetAgentDrivers::default()],
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
