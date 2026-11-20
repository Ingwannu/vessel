//! A node's plugin catalog and plugin method calls (ADR 0115, "Catalog and
//! invoke", 2026-11-20).
//!
//! [Decision Log]
//! - Purpose: Paseo's clients read a host's plugin catalog
//!   (`plugin.catalog.get`) and call a plugin's methods
//!   (`plugin.rpc.invoke`); the page and the app need the same two, so a
//!   machine's plugin themes, commands and slash commands reach them.
//! - Existing pieces: `ClientCatalog` (one node's validated declarations),
//!   `RpcName` and `RawJson`. Only the request and reply types are new.
//! - Scope: Paseo puts both under `daemon.manage`. Vessel has scopes Paseo
//!   has not: the catalog needs `read` (declarative data with bounded
//!   strings, no paths, no code), so a Read device still draws a machine's
//!   themes and the shape of its commands; invoking runs plugin code with
//!   the node's access, so it needs `admin` and is refused to agents.
//! - The reply carries the generation `plugin.watch` counts, so a client
//!   watches from the catalog it holds and reads it again when it moves,
//!   and whether this caller may invoke: clients show invoke-bound items
//!   only where they would run (coordinator ruling 2026-11-20), and a client
//!   does not otherwise know which scopes its credential holds.
//! - Invoke input and output are opaque JSON within the helper payload
//!   budget. As ADR 0079 asks of a plugin action, a call carries the
//!   generation of the catalog it came from and its context (none, a
//!   workspace, or an agent); the node refuses it when the installation has
//!   run again since (`conflict`, read the catalog again), and the actor
//!   when that generation did not declare the method. The helper receives
//!   `{input, context}`: Paseo's `onSelect` / `onSubmit` get the same
//!   context objects. A helper's own error text goes to its log, never into
//!   the reply.

use serde::{Deserialize, Serialize};

use super::{ClientCatalog, PluginGeneration, PluginId, RpcName, RuntimeGeneration};
use crate::{ProjectId, RawJson, SessionId, WorktreeId};

/// `plugin.catalog`: the running installations' client declarations. Needs
/// `read`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct GetPluginCatalog {}

/// Reply to `plugin.catalog`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PluginCatalog {
    /// The change count this catalog was read at (`plugin.watch`'s).
    pub generation: PluginGeneration,
    /// Every running installation's validated document; one that is not
    /// running is absent.
    pub catalog: ClientCatalog,
    /// Whether this caller may call `plugin.invoke` (the node's owner or a
    /// device holding `admin`; never an agent).
    pub may_invoke: bool,
}

/// `plugin.invoke`: call one method a running installation declared. Needs
/// `admin`; refused to agents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct InvokePlugin {
    /// The installation.
    pub plugin: PluginId,
    /// The run the caller's catalog entry came from.
    pub generation: RuntimeGeneration,
    /// A method that run declared.
    pub method: RpcName,
    /// The method's input, opaque to the node.
    pub input: RawJson,
    /// Where the action was taken.
    pub context: InvokeContext,
}

/// Where a plugin action was taken: Paseo's command contexts
/// (`CommandContext`, `PanelContext`), named by what a client holds. A
/// Paseo workspace is the directory agents work in; Vessel's is a project,
/// or a worktree of it (`SessionInfo.workspace`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum InvokeContext {
    /// No workspace or agent (a global command).
    Global,
    /// A project, or one of its worktrees.
    Workspace {
        /// The project.
        project: ProjectId,
        /// The worktree, when it is one.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        worktree: Option<WorktreeId>,
    },
    /// An agent (a session) in its workspace.
    Agent {
        /// The session's project.
        project: ProjectId,
        /// The session's worktree, when it runs in one.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        worktree: Option<WorktreeId>,
        /// The session.
        agent: SessionId,
    },
}

/// Reply to `plugin.invoke`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PluginOutput {
    /// What the method answered, opaque to the node.
    pub output: RawJson,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invoke_names_a_plugin_a_declared_method_and_any_json_input() {
        let generation = RuntimeGeneration::random();
        let call = |method: &str, extra: &str| {
            serde_json::from_str::<InvokePlugin>(&format!(
                r#"{{"plugin":"notes","generation":"{}","method":"{method}","input":{{"query":"메모"}},"context":{{"kind":"global"}}{extra}}}"#,
                serde_json::to_value(generation).unwrap().as_str().unwrap()
            ))
        };
        let read = call("notes.search", "").unwrap();
        assert_eq!(read.method.as_str(), "notes.search");
        assert_eq!(read.generation, generation);
        assert_eq!(read.context, InvokeContext::Global);
        assert!(call(".bad", "").is_err());
        assert!(call("a", r#","extra":true"#).is_err());
        let agent: InvokeContext = serde_json::from_value(serde_json::json!({
            "kind": "agent",
            "project": ProjectId::random(),
            "agent": SessionId::random(),
        }))
        .unwrap();
        assert!(matches!(agent, InvokeContext::Agent { worktree: None, .. }));
    }
}
