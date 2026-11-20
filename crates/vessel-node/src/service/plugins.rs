//! ADR 0115's calls: the node's plugins, managed by its administrators.
//! The guard has checked the scope (Admin for every change, the logs and
//! a method call) and refused agents those; here each call goes to
//! `crate::plugins` and its error gets its wire kind, once.
//!
//! Text a caller sends is bounded here before it reaches the plugin host
//! (review, 2026-11-20): a source or a path is at most [`MAX_PATH_BYTES`],
//! a git ref [`MAX_REF_BYTES`], the same bounds a person's typing needs.

/// The longest source string or directory path a call may name.
const MAX_PATH_BYTES: usize = 4096;
/// The longest ref a call may name.
const MAX_REF_BYTES: usize = 256;

use vessel_proto::plugins::{
    DisablePlugin, EnablePlugin, GetPluginCatalog, GetPluginLogs, InspectPlugin, InstallPlugin,
    InvokePlugin, ListPlugins, PluginCatalog, PluginGeneration, PluginInspection, PluginList,
    PluginListItem, PluginLogs, PluginOutput, ReloadPlugin, RemovePlugin, WatchPlugins,
};
use vessel_proto::{Ack, ErrorCode, Scope, WireError};

use super::{Connection, NodeService};
use crate::plugins::{CallFailure, PluginError};
use crate::principal::Principal;

impl NodeService {
    /// `plugin.list`: every installation; paths and errors for an
    /// administrator only (ADR 0079).
    pub(crate) async fn list_plugins(
        &self,
        connection: &Connection,
        _: ListPlugins,
    ) -> Result<PluginList, WireError> {
        Ok(self.plugins.list(administers(connection.principal())).await)
    }

    /// `plugin.logs`: one installation's retained log tail.
    #[expect(
        clippy::unused_async,
        reason = "every call has the same async shape, so the guard's rows are uniform"
    )]
    pub(crate) async fn plugin_logs(
        &self,
        _: &Connection,
        request: GetPluginLogs,
    ) -> Result<PluginLogs, WireError> {
        self.plugins
            .logs(&request.plugin)
            .map_err(|error| wire(&error))
    }

    /// `plugin.inspect`: a directory's manifest identity.
    pub(crate) async fn inspect_plugin(
        &self,
        _: &Connection,
        request: InspectPlugin,
    ) -> Result<PluginInspection, WireError> {
        bounded(&request.path, MAX_PATH_BYTES, "path")?;
        self.plugins
            .inspect(&request.path)
            .await
            .map_err(|error| wire(&error))
    }

    /// `plugin.install`: from a directory; started when plugins are on.
    pub(crate) async fn install_plugin(
        &self,
        _: &Connection,
        request: InstallPlugin,
    ) -> Result<PluginListItem, WireError> {
        bounded(&request.source, MAX_PATH_BYTES, "source")?;
        if let Some(reference) = &request.reference {
            bounded(reference, MAX_REF_BYTES, "ref")?;
        }
        self.plugins
            .install(&request.source, request.id, request.reference.as_deref())
            .await
            .map_err(|error| wire(&error))
    }

    /// `plugin.enable`.
    pub(crate) async fn enable_plugin(
        &self,
        _: &Connection,
        request: EnablePlugin,
    ) -> Result<PluginListItem, WireError> {
        self.plugins
            .enable(&request.plugin)
            .await
            .map_err(|error| wire(&error))
    }

    /// `plugin.disable`.
    pub(crate) async fn disable_plugin(
        &self,
        _: &Connection,
        request: DisablePlugin,
    ) -> Result<PluginListItem, WireError> {
        self.plugins
            .disable(&request.plugin)
            .await
            .map_err(|error| wire(&error))
    }

    /// `plugin.reload`.
    pub(crate) async fn reload_plugin(
        &self,
        _: &Connection,
        request: ReloadPlugin,
    ) -> Result<PluginListItem, WireError> {
        self.plugins
            .reload(&request.plugin)
            .await
            .map_err(|error| wire(&error))
    }

    /// `plugin.remove`.
    pub(crate) async fn remove_plugin(
        &self,
        _: &Connection,
        request: RemovePlugin,
    ) -> Result<Ack, WireError> {
        self.plugins
            .remove(&request.plugin)
            .await
            .map(|()| Ack)
            .map_err(|error| wire(&error))
    }

    /// `plugin.watch`: a long poll on the plugin generation.
    pub(crate) async fn watch_plugins(
        &self,
        _: &Connection,
        request: WatchPlugins,
    ) -> Result<PluginGeneration, WireError> {
        Ok(self.plugins.watch(request.after).await)
    }

    /// `plugin.catalog`: the running installations' declarations. Needs
    /// `read`: they hold no path and no code (ADR 0115).
    pub(crate) async fn plugin_catalog(
        &self,
        _: &Connection,
        _: GetPluginCatalog,
    ) -> Result<PluginCatalog, WireError> {
        self.plugins.catalog().await.map_err(|error| wire(&error))
    }

    /// `plugin.invoke`: one declared method of a running helper.
    pub(crate) async fn invoke_plugin(
        &self,
        _: &Connection,
        request: InvokePlugin,
    ) -> Result<PluginOutput, WireError> {
        self.plugins
            .invoke(&request.plugin, request.method, request.input)
            .await
            .map(|output| PluginOutput { output })
            .map_err(|error| wire(&error))
    }

    /// The switch was saved: apply it (from `node.set_settings`).
    pub(crate) fn plugins_switched(&self, on: bool) {
        self.plugins.set_switch(on);
    }
}

/// Whether `principal` manages this node's plugins: the node's owner or a
/// device holding `admin`. Agents and calls from other nodes do not.
fn administers(principal: &Principal) -> bool {
    match principal {
        Principal::Owner => true,
        Principal::Device { scopes, .. } => scopes.contains(&Scope::Admin),
        Principal::Agent { .. } | Principal::Delegated { .. } => false,
    }
}

/// Refuse `text` over `limit` bytes, naming the field.
fn bounded(text: &str, limit: usize, field: &str) -> Result<(), WireError> {
    if text.len() > limit {
        return Err(WireError::new(
            ErrorCode::InvalidRequest,
            format!("{field} is longer than {limit} bytes"),
        ));
    }
    Ok(())
}

/// The wire kind of each plugin error; the message is the node's sentence.
fn wire(error: &PluginError) -> WireError {
    let code = match error {
        PluginError::NotInstalled(_) | PluginError::NoMethods(_) => ErrorCode::NotFound,
        PluginError::Exists(_) | PluginError::Disabled(_) | PluginError::Off => ErrorCode::Conflict,
        PluginError::Source(_) | PluginError::Manifest(_) => ErrorCode::InvalidRequest,
        PluginError::Failed { .. } | PluginError::Stopping | PluginError::NotRunning(_) => {
            ErrorCode::Unavailable
        }
        PluginError::Call { failure, .. } => match failure {
            CallFailure::Undeclared(_) => ErrorCode::NotFound,
            CallFailure::TooLarge | CallFailure::Refused => ErrorCode::InvalidRequest,
            CallFailure::Unavailable => ErrorCode::Unavailable,
            CallFailure::Failed => ErrorCode::Internal,
        },
        PluginError::Store(_) | PluginError::Catalog(_) => ErrorCode::Internal,
    };
    if code == ErrorCode::Internal {
        tracing::warn!(%error, "a plugin change was not kept");
    }
    WireError::new(code, error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_text_is_refused_before_the_plugin_host() {
        assert!(bounded("~/plugins/notes", MAX_PATH_BYTES, "source").is_ok());
        let long = "a".repeat(MAX_PATH_BYTES + 1);
        let refused = bounded(&long, MAX_PATH_BYTES, "source").unwrap_err();
        assert_eq!(refused.code, ErrorCode::InvalidRequest);
        assert!(refused.message.contains("source"));
    }

    #[test]
    fn only_owners_and_admin_devices_administer_plugins() {
        assert!(administers(&Principal::Owner));
        let device = vessel_proto::DeviceId::random();
        assert!(administers(&Principal::Device {
            id: device,
            scopes: vec![Scope::Read, Scope::Admin],
        }));
        assert!(!administers(&Principal::Device {
            id: device,
            scopes: vec![Scope::Read, Scope::Operate],
        }));
    }
}
