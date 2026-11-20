//! Plugin management calls (ADR 0115): list, logs, inspect, install,
//! enable, disable, reload, remove and a change long poll.
//!
//! [Decision Log]
//! - Purpose: Paseo's host Plugins page and `paseo plugin` manage a host's
//!   plugins (`packages/server/src/server/session.ts:2552-2714`); these are
//!   the same operations as typed node calls.
//! - Choice: one request type per method (the method table maps a request
//!   type to its one method), the target always an installation id, and the
//!   reply of a lifecycle change the installation as it is afterwards, so a
//!   client shows `running` or `failed` without a second read (Paseo answers
//!   the same item).
//! - `plugin.watch` is a long poll as `fleet.watch` is: the node has no
//!   node-wide event frame, and a generation that moves on any plugin change
//!   replaces Paseo's `plugin_catalog_changed` / `plugin_settings_changed`
//!   status pushes without one.
//! - Path and error text are the node's private facts: the node fills them
//!   only for an Admin caller (ADR 0079); the types say they may be absent.

use serde::{Deserialize, Serialize};

use super::{Caption, ContentText, LogEntry, PluginId};

/// `plugin.list`: every installation on this node. Needs `read`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct ListPlugins {}

/// Reply to `plugin.list`, in installation id order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PluginList {
    /// One row per installation.
    pub plugins: Vec<PluginListItem>,
}

/// Where an installation's files come from. Directory only until managed
/// sources land (ADR 0115, P4); a later kind is a new variant at its own
/// protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum PluginSourceKind {
    /// A directory on the node, never deleted by remove.
    Directory,
}

/// Paseo's three states of an installation (`PluginStatusSchema`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum PluginStatus {
    /// Its helper answered ready, or its client data validated.
    Running,
    /// Saved as disabled, or plugins are off on this node.
    Disabled,
    /// Enabled but it could not start or stopped on its own.
    Failed,
}

/// One installation as Paseo's `PluginListItem`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PluginListItem {
    /// The installation id (config key), not necessarily the manifest's.
    pub id: PluginId,
    /// The manifest's name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<Caption>,
    /// The manifest's description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub description: Option<ContentText>,
    /// The manifest's icon path, relative to the plugin's directory.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub icon: Option<String>,
    /// The directory on the node; only for an Admin caller.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub path: Option<String>,
    /// Where its files come from.
    pub source: PluginSourceKind,
    /// The saved intent, independent of the node's global switch.
    pub enabled: bool,
    /// What runs now.
    pub status: PluginStatus,
    /// Why it failed, in the node's words; only for an Admin caller.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub error: Option<String>,
}

/// `plugin.logs`: an installation's retained log tail. Needs `admin`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct GetPluginLogs {
    /// Which installation.
    pub plugin: PluginId,
}

/// Reply to `plugin.logs`, oldest first.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PluginLogs {
    /// At most Paseo's 500 entries and 256 KiB (`MAX_LOG_*`).
    pub entries: Vec<LogEntry>,
}

/// `plugin.inspect`: read a directory's manifest without installing it.
/// Needs `admin` (it reads a path on the node).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct InspectPlugin {
    /// An absolute path, or one starting with `~`.
    pub path: String,
}

/// Reply to `plugin.inspect`: the manifest's identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PluginInspection {
    /// The manifest's id, the default installation id.
    pub id: PluginId,
    /// The manifest's name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<Caption>,
    /// The manifest's description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub description: Option<ContentText>,
}

/// `plugin.install`: install from Paseo's source string. Needs `admin`.
/// This protocol takes a directory (absolute or `~`, optionally
/// `:<relative subpath>`); other source kinds are refused until managed
/// sources land.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct InstallPlugin {
    /// Paseo's source string.
    pub source: String,
    /// The installation id; the manifest's when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub id: Option<PluginId>,
    /// A Git ref; refused for a directory, as Paseo refuses it.
    #[serde(default, rename = "ref", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub reference: Option<String>,
}

macro_rules! target_request {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
        #[cfg_attr(feature = "ts", derive(ts_rs::TS))]
        #[serde(deny_unknown_fields)]
        pub struct $name {
            /// Which installation.
            pub plugin: PluginId,
        }
    };
}

target_request!(
    /// `plugin.enable`: save the installation as enabled and start it when
    /// plugins are on. Needs `admin`.
    EnablePlugin
);
target_request!(
    /// `plugin.disable`: save the installation as disabled and stop it.
    /// Needs `admin`.
    DisablePlugin
);
target_request!(
    /// `plugin.reload`: stop it and start it again from disk; may end
    /// failed, as Paseo's. Needs `admin`.
    ReloadPlugin
);
target_request!(
    /// `plugin.remove`: stop it and forget it, with its settings and logs;
    /// a directory source is left on disk. Needs `admin`.
    RemovePlugin
);

/// A number that moves on every plugin change on this node: an install, a
/// removal, a saved intent, a status, the switch, a catalog or settings.
/// Starts at zero for each node start.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize,
)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(transparent)]
pub struct PluginGeneration(u64);

impl PluginGeneration {
    /// Wrap a number.
    #[must_use]
    pub const fn new(number: u64) -> Self {
        Self(number)
    }

    /// The number.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    /// The next generation; saturates (2^64 changes do not happen).
    #[must_use]
    pub const fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }
}

/// `plugin.watch`: wait for a generation other than `after`. A long poll:
/// the node answers when one is published, or with its current generation
/// when its wait limit passes, and the client asks again. Needs `read`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct WatchPlugins {
    /// The generation the client has.
    pub after: PluginGeneration,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_spells_ref_as_paseo_and_refuses_unknown_fields() {
        let install: InstallPlugin =
            serde_json::from_str(r#"{"source":"~/p","id":"x","ref":"main"}"#).unwrap();
        assert_eq!(install.reference.as_deref(), Some("main"));
        assert_eq!(
            serde_json::to_value(&install).unwrap(),
            serde_json::json!({"source":"~/p","id":"x","ref":"main"})
        );
        assert!(serde_json::from_str::<InstallPlugin>(r#"{"source":"/p","extra":1}"#).is_err());
        assert!(serde_json::from_str::<EnablePlugin>(r#"{"plugin":"Bad"}"#).is_err());
    }

    #[test]
    fn list_item_omits_private_fields_when_absent() {
        let item = PluginListItem {
            id: "theme".parse().unwrap(),
            name: None,
            description: None,
            icon: None,
            path: None,
            source: PluginSourceKind::Directory,
            enabled: true,
            status: PluginStatus::Running,
            error: None,
        };
        assert_eq!(
            serde_json::to_value(&item).unwrap(),
            serde_json::json!({"id":"theme","source":"directory","enabled":true,"status":"running"})
        );
    }

    #[test]
    fn generation_moves_forward_and_saturates() {
        assert_eq!(PluginGeneration::default().next().get(), 1);
        assert_eq!(PluginGeneration::new(u64::MAX).next().get(), u64::MAX);
    }
}
