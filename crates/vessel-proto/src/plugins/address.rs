//! A contribution address captures node and runtime lifetime before dispatch.

use serde::{Deserialize, Serialize};

use super::{ContributionId, PluginId};
use crate::workspace::RelPath;
use crate::{NodeId, SessionId};

/// A new generation is allocated whenever a helper or declaration installation
/// is replaced. This public correlation value is not an authorization token.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
#[serde(transparent)]
pub struct RuntimeGeneration(uuid::Uuid);

impl RuntimeGeneration {
    /// Allocate a fresh correlation value, including after a node restart.
    #[must_use]
    pub fn random() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

/// Resolving by plugin name alone must never substitute another host or process.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct PluginAddress {
    /// Authoritative node; a book alias is not identity.
    pub node: NodeId,
    /// Installed runtime id, which may differ from the manifest's default id.
    pub plugin: PluginId,
    /// Exact installation lifetime that supplied this action.
    pub generation: RuntimeGeneration,
}

/// Registry namespaces remain explicit when different kinds share a local id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ContributionKind {
    /// Host navigation.
    Screen,
    /// Host settings.
    SettingsScreen,
    /// Workspace/agent dock.
    Panel,
    /// Matching file tab.
    FilePanel,
    /// Sidebar header action; a footer may reuse the same local id.
    SidebarHeader,
    /// Sidebar footer action; separate registration namespace.
    SidebarFooter,
    /// Command center.
    Command,
    /// Composer command.
    SlashCommand,
    /// Workspace header.
    HeaderButton,
    /// Agent composer.
    ComposerPill,
    /// Composer attachment picker.
    AttachmentSource,
    /// Plugin timeline item.
    TimelineRenderer,
    /// Appearance picker.
    Theme,
}

/// Stable contribution selection plus the captured installation lifetime.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct ContributionAddress {
    /// Captured host and process generation.
    pub installation: PluginAddress,
    /// Typed contribution namespace.
    pub kind: ContributionKind,
    /// Stable local id within that namespace.
    pub contribution: ContributionId,
}

/// Runtime context supplied by the core workspace owner, never inferred from cwd.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PluginPanelContext {
    /// The pane's existing W11 workspace identity is authoritative.
    Workspace {},
    /// The pane's workspace owns this session, checked again on open and restore.
    Agent {
        /// Session displayed by this panel.
        agent: SessionId,
    },
    /// The pane's workspace owns this relative file path.
    File {
        /// Confined path; file access still uses the node's ordinary guards.
        path: RelPath,
    },
}

/// Core docking slots; plugins cannot introduce another layout engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum PanelLocation {
    /// Main workspace tabs.
    Workspace,
    /// Explorer side pane.
    Explorer,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actions_cannot_compare_equal_across_hosts_or_process_generations() {
        let address = PluginAddress {
            node: NodeId::random(),
            plugin: "example".parse().expect("fixture"),
            generation: RuntimeGeneration::random(),
        };
        let mut other = address.clone();
        other.node = NodeId::random();
        assert_ne!(address, other);
        other = address.clone();
        other.generation = RuntimeGeneration::random();
        assert_ne!(address, other);
        let encoded = serde_json::to_string(&address).expect("fixture encode");
        assert_eq!(
            serde_json::from_str::<PluginAddress>(&encoded).expect("fixture decode"),
            address
        );
    }

    #[test]
    fn file_context_refuses_escape_and_unknown_context_is_not_workspace() {
        assert!(
            serde_json::from_str::<PluginPanelContext>(r#"{"kind":"file","path":"../private"}"#)
                .is_err()
        );
        assert!(serde_json::from_str::<PluginPanelContext>(r#"{"kind":"future"}"#).is_err());
        assert!(
            serde_json::from_str::<PluginPanelContext>(
                r#"{"kind":"workspace","callback":"run()"}"#
            )
            .is_err()
        );
    }
}
