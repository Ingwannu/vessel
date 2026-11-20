//! Plugin identity and native declarative contracts (ADR 0079).
//!
//! These values perform no I/O and grant no authority. The manager calls
//! (ADR 0115, `manage`) are registered at [`PLUGINS_PROTOCOL`]; the helper
//! frames stay private to the node.

mod address;
mod attachment;
mod budget;
mod catalog;
mod declarations;
mod document;
mod hook;
mod identity;
mod json_error;
mod logs;
mod manage;
mod payload;
mod process;
mod theme;

pub use address::{
    ContributionAddress, ContributionKind, PanelLocation, PluginAddress, PluginPanelContext,
    RuntimeGeneration,
};
pub use attachment::{
    AttachmentError, AttachmentItem, AttachmentItemId, AttachmentItemKey, AttachmentSearchResult,
    AttachmentSourceAddress, AttachmentUrl, ContextAttachment, MAX_ATTACHMENT_ITEMS,
};
pub use identity::{
    Caption, ContentText, ContributionId, FileExtension, FileNamePattern, InvalidPluginValue,
    PluginId, RpcName, ThemeName,
};
pub use json_error::PluginJsonError;
pub use logs::{
    InvalidLogText, LogEntry, LogSequence, LogStream, LogText, LogTruncation, MAX_LOG_BYTES,
    MAX_LOG_ENTRIES, MAX_LOG_LINE_BYTES,
};
pub use manage::{
    DisablePlugin, EnablePlugin, GetPluginLogs, InspectPlugin, InstallPlugin, ListPlugins,
    PluginGeneration, PluginInspection, PluginList, PluginListItem, PluginLogs, PluginSourceKind,
    PluginStatus, ReloadPlugin, RemovePlugin, WatchPlugins,
};
pub use theme::{Appearance, HexColor, ThemeColors, ThemeContribution};

/// The plugin manager calls (ADR 0115): `plugin.list`, `plugin.logs`,
/// `plugin.inspect`, `plugin.install`, `plugin.enable`, `plugin.disable`,
/// `plugin.reload`, `plugin.remove`, `plugin.watch`, and the node settings'
/// `plugins_enabled`. Drafted as 35; assigned 36 at its node-line merge,
/// after ADR 0088's `forge.repos` took 35.
pub const PLUGINS_PROTOCOL: Option<crate::ProtocolVersion> = Some(crate::ProtocolVersion::new(36));

pub use declarations::{
    Action, Availability, Button, ButtonBehavior, ChoiceOption, CommandContext, Contribution,
    ContributionContent, Control, ControlContent, MenuEntry, PanelContext, SidebarPlacement,
    StatusTone, Surface, ToggleState,
};
pub use document::{
    ClientDocument, DocumentError, MAX_ACTION_INPUT_BYTES, MAX_CONTRIBUTIONS, MAX_CONTROLS,
    MAX_DEPTH, MAX_DOCUMENT_BYTES, MAX_METHODS,
};

pub use hook::{DuplicateHook, HookName, HookSubscriptions, HookWait};
pub use payload::{HelperPayload, MAX_HELPER_PAYLOAD_BYTES, PayloadTooLarge};
pub use process::{
    HelperCall, HelperFailure, HelperProtocol, HelperToNode, InvocationId, MAX_HELPER_FRAME_BYTES,
    NodeToHelper, UnsupportedHelperProtocol,
};

#[cfg(test)]
#[path = "process_tests.rs"]
mod process_tests;

pub use catalog::{
    CatalogError, ClientCatalog, ClientInstallation, MAX_CATALOG_BYTES, MAX_CATALOG_INSTALLATIONS,
    MAX_CATALOG_THEMES,
};
