//! A node's settings that a client reads and changes (protocol 12): for now
//! how long an idle agent runs before it is parked (ADR 0038).
//!
//! [Decision Log]
//! - Purpose: parking was set only by `vesseld --park-after`, so a person
//!   in the app could neither see nor change it. Paseo has a card in a
//!   host's settings that turns idle cleanup on and off, kept by the
//!   daemon and applied at once.
//! - Choice: one settings object, read whole (`node.settings`, read) and
//!   changed field by field (`node.set_settings`, admin), so a later
//!   setting is one more optional field and an older client keeps working.
//!   The node keeps the same type as its file, so what a client sees is
//!   what survives a restart.
//! - Protocol 18 adds `drivers` (ADR 0057): which protocol drives each
//!   agent that has one of its own. A node of 17 answers without it, which
//!   reads as ACP for every agent, what it runs.

mod metadata;
pub use metadata::{
    InvalidMetadata, MAX_METADATA_PROVIDERS, MetadataEffort, MetadataProvider, MetadataSettings,
};

use serde::{Deserialize, Serialize};

use crate::harness::HarnessKind;
use crate::parking::ParkAfter;
use crate::restart::RestartRecovery;
use crate::terminal_profiles::Profiles;

/// The node's settings in force.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct NodeSettings {
    /// How long a structured session may stay idle before its agent is
    /// parked.
    pub park_after: ParkAfter,
    /// Which protocol drives each agent's new sessions (protocol 18);
    /// absent when every agent runs over ACP.
    #[serde(default, skip_serializing_if = "AgentDrivers::all_acp")]
    pub drivers: AgentDrivers,
    /// Startup policy, hidden until its protocol is assigned (ADR 0059).
    /// This value-only field does not enable an engine or register input.
    #[serde(default, skip_serializing_if = "RestartRecovery::is_default")]
    pub restart_recovery: RestartRecovery,
    /// Generated-name routes (ADR 0062): `Some` with an empty list is Automatic;
    /// `None` means this peer cannot generate.
    #[serde(
        default,
        deserialize_with = "MetadataSettings::deserialize_optional_view",
        skip_serializing_if = "Option::is_none"
    )]
    #[cfg_attr(feature = "ts", ts(as = "Option<MetadataSettings>", optional))]
    pub metadata: Option<MetadataSettings>,
    /// Whether installed plugins run on this node (ADR 0115, Paseo's
    /// `pluginsEnabled`, default off); hidden below `PLUGINS_PROTOCOL`.
    #[serde(default, skip_serializing_if = "core::ops::Not::not")]
    pub plugins_enabled: bool,
    /// Whether the node installs its terminal activity hooks into the
    /// agents' own configuration (ADR 0089, Paseo's
    /// `enableTerminalAgentHooks`, default off); hidden below
    /// `ATTENTION_NOTIFICATIONS_PROTOCOL`, as the feature's other fields are.
    #[serde(default, skip_serializing_if = "core::ops::Not::not")]
    pub terminal_agent_hooks: bool,
    /// The node's terminal profiles as kept (ADR 0117, Paseo's
    /// `terminalProfiles`): absent when never set, which a client shows as
    /// [`Profiles::shipped`]; empty when a person removed them all. Hidden
    /// below `TERMINAL_PROFILES_PROTOCOL`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal_profiles: Option<Profiles>,
}

impl NodeSettings {
    /// Project a reply for a peer, including a legacy settings mutation's
    /// response. Unassigned fields stay hidden even from current peers.
    #[must_use]
    pub fn for_protocol(&self, protocol: crate::ProtocolVersion) -> Option<Self> {
        let recovery =
            self.for_recovery_protocol(protocol, crate::restart::RESTART_RECOVERY_PROTOCOL);
        let base = recovery.as_ref().unwrap_or(self);
        let named = match base.for_protocol_with_names(protocol, crate::GENERATED_NAMES) {
            Some(projected) => Some(projected),
            None => recovery,
        };
        let base = named.as_ref().unwrap_or(self);
        let plugins = base
            .for_plugins_protocol(protocol, crate::plugins::PLUGINS_PROTOCOL)
            .or(named);
        let base = plugins.as_ref().unwrap_or(self);
        let hooks = base
            .for_hooks_protocol(
                protocol,
                crate::notifications::ATTENTION_NOTIFICATIONS_PROTOCOL,
            )
            .or(plugins);
        let base = hooks.as_ref().unwrap_or(self);
        base.for_profiles_protocol(
            protocol,
            crate::terminal_profiles::TERMINAL_PROFILES_PROTOCOL,
        )
        .or(hooks)
    }

    /// The terminal profiles (ADR 0117) stay hidden from a peer below their
    /// protocol, as the other later fields are.
    fn for_profiles_protocol(
        &self,
        protocol: crate::ProtocolVersion,
        assigned: Option<crate::ProtocolVersion>,
    ) -> Option<Self> {
        let readable = assigned.is_some_and(|version| protocol >= version);
        (!readable && self.terminal_profiles.is_some()).then(|| {
            let mut older = self.clone();
            older.terminal_profiles = None;
            older
        })
    }

    /// The terminal agent hooks switch (ADR 0089) stays hidden from a peer
    /// below the feature's protocol, as the plugin switch does below its own.
    fn for_hooks_protocol(
        &self,
        protocol: crate::ProtocolVersion,
        assigned: Option<crate::ProtocolVersion>,
    ) -> Option<Self> {
        let readable = assigned.is_some_and(|version| protocol >= version);
        (!readable && self.terminal_agent_hooks).then(|| {
            let mut older = self.clone();
            older.terminal_agent_hooks = false;
            older
        })
    }

    /// The plugin switch (ADR 0115) stays hidden from a peer below its
    /// protocol, as the other later fields are.
    fn for_plugins_protocol(
        &self,
        protocol: crate::ProtocolVersion,
        assigned: Option<crate::ProtocolVersion>,
    ) -> Option<Self> {
        let readable = assigned.is_some_and(|version| protocol >= version);
        (!readable && self.plugins_enabled).then(|| {
            let mut older = self.clone();
            older.plugins_enabled = false;
            older
        })
    }

    fn for_recovery_protocol(
        &self,
        protocol: crate::ProtocolVersion,
        assigned: Option<crate::ProtocolVersion>,
    ) -> Option<Self> {
        let supported = assigned.is_some_and(|required| protocol >= required);
        (!supported && !self.restart_recovery.is_default()).then(|| {
            let mut settings = self.clone();
            settings.restart_recovery = RestartRecovery::default();
            settings
        })
    }

    fn for_protocol_with_names(
        &self,
        protocol: crate::ProtocolVersion,
        names: Option<crate::ProtocolVersion>,
    ) -> Option<Self> {
        let readable = names.is_some_and(|version| protocol >= version);
        (!readable && self.metadata.is_some()).then(|| {
            let mut older = self.clone();
            older.metadata = None;
            older
        })
    }
}

/// Which protocol drives an agent's structured sessions (protocol 18, ADR
/// 0057).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum AgentDriver {
    /// The Agent Client Protocol: the agent's own ACP mode or its adapter.
    #[default]
    Acp,
    /// The agent's own protocol: Codex's app-server, Claude Code's
    /// stream-json, `OpenCode`'s server. Refused while this node's driver
    /// for that agent cannot ask a person for permissions.
    Direct,
    /// A choice a newer node offers; never accepted in a change, and run
    /// as `acp`.
    #[serde(other)]
    Unknown,
}

/// The driver of each agent that has a protocol of its own (protocol 18).
/// Other agents run over ACP.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct AgentDrivers {
    /// Claude Code's.
    #[serde(default)]
    pub claude_code: AgentDriver,
    /// Codex's.
    #[serde(default)]
    pub codex: AgentDriver,
    /// `OpenCode`'s.
    #[serde(default)]
    pub open_code: AgentDriver,
}

impl AgentDrivers {
    /// `kind`'s driver; `None` for an agent with no protocol of its own.
    #[must_use]
    pub const fn get(&self, kind: HarnessKind) -> Option<AgentDriver> {
        match kind {
            HarnessKind::ClaudeCode => Some(self.claude_code),
            HarnessKind::Codex => Some(self.codex),
            HarnessKind::OpenCode => Some(self.open_code),
            HarnessKind::Gemini => None,
        }
    }

    /// Whether every agent runs over ACP.
    #[must_use]
    pub fn all_acp(&self) -> bool {
        *self == Self::default()
    }
}

/// A change to [`AgentDrivers`]: the agents it names; the others stay.
/// Inside `settings.json` too, so it never refuses an unknown field (an
/// agent a later build names), as [`SetNodeSettings`] says.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SetAgentDrivers {
    /// Claude Code's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claude_code: Option<AgentDriver>,
    /// Codex's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub codex: Option<AgentDriver>,
    /// `OpenCode`'s.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub open_code: Option<AgentDriver>,
}

impl SetAgentDrivers {
    /// Each agent this change names, with its driver.
    pub fn named(&self) -> impl Iterator<Item = (HarnessKind, AgentDriver)> {
        [
            (HarnessKind::ClaudeCode, self.claude_code),
            (HarnessKind::Codex, self.codex),
            (HarnessKind::OpenCode, self.open_code),
        ]
        .into_iter()
        .filter_map(|(kind, driver)| driver.map(|driver| (kind, driver)))
    }

    /// `drivers` with this change laid over it.
    #[must_use]
    pub fn over(&self, drivers: Self) -> Self {
        Self {
            claude_code: self.claude_code.or(drivers.claude_code),
            codex: self.codex.or(drivers.codex),
            open_code: self.open_code.or(drivers.open_code),
        }
    }
}

/// `node.settings`: the node's settings in force. Needs `read`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GetNodeSettings {}

/// `node.set_settings`: change the settings it names; the others stay.
/// Needs `admin`. Answers the settings in force.
///
/// Also the type of `settings.json` (vessel-store's `settings_file`), so it
/// and every type inside it never refuse a field they do not know (no
/// `deny_unknown_fields`, a later choice read as `unknown`): a file a later
/// build wrote must still parse after a downgrade, keeping what this build
/// knows. vessel-store's `a_file_from_a_later_build_keeps_what_this_build_knows`
/// holds the rule.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SetNodeSettings {
    /// How long a structured session may stay idle before its agent is
    /// parked; `unknown` is refused.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub park_after: Option<ParkAfter>,
    /// Which protocol drives the agents it names (protocol 18); `unknown`
    /// is refused, and so is `direct` for an agent whose driver this node
    /// does not offer yet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drivers: Option<SetAgentDrivers>,
    /// Strict generated-name patch; an empty list resets automatic selection.
    /// Refused until `GENERATED_NAMES` is assigned at integration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<MetadataSettings>,
    /// Turn installed plugins on or off on this node, at once (ADR 0115).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugins_enabled: Option<bool>,
    /// Install or remove the terminal activity hooks in the agents' own
    /// configuration, at once (ADR 0089). Refused until
    /// `ATTENTION_NOTIFICATIONS_PROTOCOL` is assigned.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal_agent_hooks: Option<bool>,
    /// Replace the node's terminal profiles with this list, in this order
    /// (ADR 0117; Paseo patches the whole list too). Refused until
    /// `TERMINAL_PROFILES_PROTOCOL` is assigned.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal_profiles: Option<Profiles>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovery_projection_respects_a_synthetic_assigned_boundary() {
        let original = NodeSettings {
            park_after: ParkAfter::Never,
            drivers: AgentDrivers::default(),
            restart_recovery: RestartRecovery {
                enabled: true,
                prompt: "custom boundary prompt".parse().unwrap(),
            },
            metadata: None,
            plugins_enabled: false,
            terminal_agent_hooks: false,
            terminal_profiles: None,
        };
        let required = crate::PROTOCOL_VERSION;
        let older = crate::ProtocolVersion::new(required.get().checked_sub(1).unwrap());
        for (protocol, assignment) in [(older, Some(required)), (required, None)] {
            let projected = original
                .for_recovery_protocol(protocol, assignment)
                .unwrap();
            assert_eq!(projected.restart_recovery, RestartRecovery::default());
            assert!(
                !serde_json::to_string(&projected)
                    .unwrap()
                    .contains("restart_recovery")
            );
        }
        assert_eq!(
            original.for_recovery_protocol(required, Some(required)),
            None
        );
    }

    /// The plugin switch (ADR 0115) reaches only peers of its protocol and
    /// is absent from the wire while off.
    #[test]
    fn plugin_switch_is_hidden_below_its_protocol_and_absent_while_off() {
        let off: NodeSettings = serde_json::from_str(r#"{"park_after":{"kind":"never"}}"#).unwrap();
        assert!(!off.plugins_enabled);
        let on = NodeSettings {
            plugins_enabled: true,
            ..off.clone()
        };
        assert!(
            serde_json::to_string(&on)
                .unwrap()
                .contains("plugins_enabled")
        );
        let required = crate::ProtocolVersion::new(35);
        let older = crate::ProtocolVersion::new(34);
        assert_eq!(on.for_plugins_protocol(required, Some(required)), None);
        assert_eq!(
            on.for_plugins_protocol(older, Some(required)),
            Some(off.clone())
        );
        assert_eq!(on.for_plugins_protocol(required, None), Some(off));
        let change: SetNodeSettings = serde_json::from_str(r#"{"plugins_enabled":true}"#).unwrap();
        assert_eq!(change.plugins_enabled, Some(true));
    }

    /// The terminal agent hooks switch (ADR 0089) reaches only peers of the
    /// feature's protocol, is absent from the wire while off, and is hidden
    /// from every peer while the protocol is unassigned.
    #[test]
    fn hooks_switch_is_hidden_below_the_attention_protocol_and_absent_while_off() {
        let off: NodeSettings = serde_json::from_str(r#"{"park_after":{"kind":"never"}}"#).unwrap();
        assert!(!off.terminal_agent_hooks);
        assert!(
            !serde_json::to_string(&off)
                .unwrap()
                .contains("terminal_agent_hooks"),
            "absent while off"
        );
        let on = NodeSettings {
            terminal_agent_hooks: true,
            ..off.clone()
        };
        assert!(
            serde_json::to_string(&on)
                .unwrap()
                .contains("terminal_agent_hooks")
        );
        let required = crate::ProtocolVersion::new(41);
        let older = crate::ProtocolVersion::new(40);
        assert_eq!(on.for_hooks_protocol(required, Some(required)), None);
        assert_eq!(
            on.for_hooks_protocol(older, Some(required)),
            Some(off.clone())
        );
        assert_eq!(on.for_hooks_protocol(required, None), Some(off.clone()));
        if crate::notifications::ATTENTION_NOTIFICATIONS_PROTOCOL.is_none() {
            assert_eq!(on.for_protocol(crate::PROTOCOL_VERSION), Some(off));
        }
        let change: SetNodeSettings =
            serde_json::from_str(r#"{"terminal_agent_hooks":true}"#).unwrap();
        assert_eq!(change.terminal_agent_hooks, Some(true));
        // A change naming it, on or off, needs the attention protocol, and
        // is refused while that is unassigned (`rpc/since.rs`).
        for named in [Some(true), Some(false)] {
            let call = crate::Call::SetNodeSettings(SetNodeSettings {
                terminal_agent_hooks: named,
                ..SetNodeSettings::default()
            });
            match crate::notifications::ATTENTION_NOTIFICATIONS_PROTOCOL {
                None => assert_eq!(call.required_protocol(), None, "{named:?}"),
                Some(assigned) => {
                    assert!(call.required_protocol() >= Some(assigned), "{named:?}");
                }
            }
        }
        let unnamed = crate::Call::SetNodeSettings(SetNodeSettings::default());
        assert!(unnamed.required_protocol().is_some());
    }

    /// A node of 17 answers without `drivers`: every agent over ACP. A
    /// change names only the agents it changes.
    #[test]
    fn drivers_read_as_acp_when_absent_and_change_by_agent() {
        let older: NodeSettings =
            serde_json::from_str(r#"{"park_after":{"kind":"never"}}"#).unwrap();
        assert!(older.drivers.all_acp());
        assert_eq!(
            serde_json::to_string(&older).unwrap(),
            r#"{"park_after":{"kind":"never"}}"#
        );
        let change: SetNodeSettings =
            serde_json::from_str(r#"{"drivers":{"codex":"direct","open_code":"later"}}"#).unwrap();
        let drivers = change.drivers.unwrap();
        assert_eq!(
            drivers.named().collect::<Vec<_>>(),
            [
                (HarnessKind::Codex, AgentDriver::Direct),
                (HarnessKind::OpenCode, AgentDriver::Unknown)
            ]
        );
        let saved = SetAgentDrivers {
            claude_code: Some(AgentDriver::Acp),
            codex: Some(AgentDriver::Acp),
            open_code: None,
        };
        assert_eq!(
            drivers.over(saved),
            SetAgentDrivers {
                claude_code: Some(AgentDriver::Acp),
                codex: Some(AgentDriver::Direct),
                open_code: Some(AgentDriver::Unknown),
            }
        );
        let in_force = AgentDrivers {
            codex: AgentDriver::Direct,
            ..AgentDrivers::default()
        };
        assert_eq!(in_force.get(HarnessKind::Codex), Some(AgentDriver::Direct));
        assert_eq!(in_force.get(HarnessKind::Gemini), None);
        assert_eq!(
            serde_json::to_string(&in_force).unwrap(),
            r#"{"claude_code":"acp","codex":"direct","open_code":"acp"}"#
        );
    }
}

#[cfg(test)]
mod names_tests;
#[cfg(test)]
mod profiles_tests;
