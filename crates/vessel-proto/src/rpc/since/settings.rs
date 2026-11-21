//! Setting-specific admission prevents an older hop silently ignoring a
//! later field. The outer option refuses a feature still awaiting assignment.

use crate::settings::SetNodeSettings;

pub(super) fn fields_since(request: &SetNodeSettings) -> Option<Option<u32>> {
    // Agent drivers arrived in protocol 18.
    let drivers = request.drivers.map(|_| 18);
    // The plugin switch (ADR 0115) needs the plugin protocol.
    let plugins = match request.plugins_enabled {
        Some(_) => Some(crate::plugins::PLUGINS_PROTOCOL?.get()),
        None => None,
    };
    // The terminal agent hooks switch (ADR 0089) needs the
    // attention protocol: one number carries the whole feature.
    let hooks = match request.terminal_agent_hooks {
        Some(_) => Some(crate::notifications::ATTENTION_NOTIFICATIONS_PROTOCOL?.get()),
        None => None,
    };
    // The terminal profiles (ADR 0117): an older node would
    // keep its list and say nothing.
    let profiles = match request.terminal_profiles {
        Some(_) => Some(crate::terminal_profiles::TERMINAL_PROFILES_PROTOCOL?.get()),
        None => None,
    };
    let tools = match request.vessel_tools {
        Some(_) => Some(crate::mcp::VESSEL_TOOLS_PROTOCOL?.get()),
        None => None,
    };
    let base = drivers.max(plugins).max(hooks).max(profiles).max(tools);
    let metadata = if request.metadata.is_some() {
        Some(crate::GENERATED_NAMES?.get())
    } else {
        None
    };
    Some(base.max(metadata))
}
