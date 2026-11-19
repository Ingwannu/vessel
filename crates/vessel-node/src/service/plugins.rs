//! ADR 0115's calls: the node's plugins, managed by its administrators.
//! The guard has checked the scope (Admin for every change and the logs)
//! and refused agents those; here each call goes to `crate::plugins` and
//! its error gets its wire kind, once.

use vessel_proto::plugins::{
    DisablePlugin, EnablePlugin, GetPluginLogs, InspectPlugin, InstallPlugin, ListPlugins,
    PluginGeneration, PluginInspection, PluginList, PluginListItem, PluginLogs, ReloadPlugin,
    RemovePlugin, WatchPlugins,
};
use vessel_proto::{Ack, ErrorCode, Scope, WireError};

use super::{Connection, NodeService};
use crate::plugins::PluginError;
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
    pub(crate) async fn plugin_logs(
        &self,
        _: &Connection,
        request: GetPluginLogs,
    ) -> Result<PluginLogs, WireError> {
        self.plugins.logs(&request.plugin).map_err(wire)
    }

    /// `plugin.inspect`: a directory's manifest identity.
    pub(crate) async fn inspect_plugin(
        &self,
        _: &Connection,
        request: InspectPlugin,
    ) -> Result<PluginInspection, WireError> {
        self.plugins.inspect(&request.path).await.map_err(wire)
    }

    /// `plugin.install`: from a directory; started when plugins are on.
    pub(crate) async fn install_plugin(
        &self,
        _: &Connection,
        request: InstallPlugin,
    ) -> Result<PluginListItem, WireError> {
        self.plugins
            .install(&request.source, request.id, request.reference.as_deref())
            .await
            .map_err(wire)
    }

    /// `plugin.enable`.
    pub(crate) async fn enable_plugin(
        &self,
        _: &Connection,
        request: EnablePlugin,
    ) -> Result<PluginListItem, WireError> {
        self.plugins.enable(&request.plugin).await.map_err(wire)
    }

    /// `plugin.disable`.
    pub(crate) async fn disable_plugin(
        &self,
        _: &Connection,
        request: DisablePlugin,
    ) -> Result<PluginListItem, WireError> {
        self.plugins.disable(&request.plugin).await.map_err(wire)
    }

    /// `plugin.reload`.
    pub(crate) async fn reload_plugin(
        &self,
        _: &Connection,
        request: ReloadPlugin,
    ) -> Result<PluginListItem, WireError> {
        self.plugins.reload(&request.plugin).await.map_err(wire)
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
            .map_err(wire)
    }

    /// `plugin.watch`: a long poll on the plugin generation.
    pub(crate) async fn watch_plugins(
        &self,
        _: &Connection,
        request: WatchPlugins,
    ) -> Result<PluginGeneration, WireError> {
        Ok(self.plugins.watch(request.after).await)
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

/// The wire kind of each plugin error; the message is the node's sentence.
fn wire(error: PluginError) -> WireError {
    let code = match &error {
        PluginError::NotInstalled(_) => ErrorCode::NotFound,
        PluginError::Exists(_) | PluginError::Disabled(_) | PluginError::Off => ErrorCode::Conflict,
        PluginError::Source(_) | PluginError::Manifest(_) => ErrorCode::InvalidRequest,
        PluginError::Failed { .. } | PluginError::Stopping => ErrorCode::Unavailable,
        PluginError::Store(_) => ErrorCode::Internal,
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
