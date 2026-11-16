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
//!   Store's opaque disk model preserves saved choices independently of
//!   wire projection; a generic reply never serializes that private document.
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
    /// Startup policy, hidden until its protocol is assigned (ADR 0098).
    /// An explicit settings read requests it; older replies omit it.
    #[serde(default, skip_serializing_if = "RestartRecovery::is_default")]
    pub restart_recovery: RestartRecovery,
}

impl NodeSettings {
    /// Project a reply for a peer, including a legacy settings mutation's
    /// response. Unassigned fields stay hidden even from current peers.
    #[must_use]
    pub fn for_protocol(&self, protocol: crate::ProtocolVersion) -> Option<Self> {
        self.for_recovery_protocol(protocol, crate::restart::RESTART_RECOVERY_PROTOCOL)
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
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GetNodeSettings {
    /// Explicit capability read; unavailable until recovery's protocol is assigned.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub include_restart_recovery: bool,
}

/// `node.set_settings`: change the settings it names; the others stay.
/// Needs `admin`. Answers the settings in force.
///
/// Store's private saved document reuses these field types with its own disk
/// codec. Every field remains tolerant on read: a later build's unknown field
/// must not prevent a downgrade from preserving the settings it understands.
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
    /// Startup policy patch. An absent patch preserves every saved choice.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restart_recovery: Option<crate::restart::SetRestartRecovery>,
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
