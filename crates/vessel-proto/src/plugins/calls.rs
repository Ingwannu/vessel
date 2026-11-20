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
//!   watches from the catalog it holds and reads it again when it moves.
//! - Invoke input and output are opaque JSON within the helper payload
//!   budget; the node checks only that the method is declared by the
//!   installation's running generation. A helper's own error text goes to
//!   its log, never into the reply.

use serde::{Deserialize, Serialize};

use super::{ClientCatalog, PluginGeneration, PluginId, RpcName};
use crate::RawJson;

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
}

/// `plugin.invoke`: call one method a running installation declared. Needs
/// `admin`; refused to agents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct InvokePlugin {
    /// The installation.
    pub plugin: PluginId,
    /// A method its running generation declared.
    pub method: RpcName,
    /// The method's input, opaque to the node.
    pub input: RawJson,
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
        let call: InvokePlugin = serde_json::from_str(
            r#"{"plugin":"notes","method":"notes.search","input":{"query":"메모"}}"#,
        )
        .unwrap();
        assert_eq!(call.method.as_str(), "notes.search");
        assert!(
            serde_json::from_str::<InvokePlugin>(
                r#"{"plugin":"notes","method":".bad","input":null}"#
            )
            .is_err()
        );
        assert!(
            serde_json::from_str::<InvokePlugin>(
                r#"{"plugin":"notes","method":"a","input":1,"extra":true}"#
            )
            .is_err()
        );
    }
}
