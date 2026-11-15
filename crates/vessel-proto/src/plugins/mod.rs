//! Plugin identity and native declarative contracts (ADR 0079).
//!
//! These values perform no I/O and grant no authority. Calls stay unregistered
//! until their node service, clients and shared pending-method mechanism exist.

mod address;
mod attachment;
mod budget;
mod catalog;
mod declarations;
mod document;
mod identity;
mod json_error;
mod logs;
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
pub use theme::{Appearance, HexColor, ThemeColors, ThemeContribution};

/// The integrator assigns this only after management, lifecycle and native
/// declarations have real consumers. No current peer may use pending calls.
pub const PLUGINS: Option<crate::ProtocolVersion> = None;

pub use declarations::{
    Action, Availability, Button, ButtonBehavior, ChoiceOption, CommandContext, Contribution,
    ContributionContent, Control, ControlContent, MenuEntry, PanelContext, SidebarPlacement,
    StatusTone, Surface, ToggleState,
};
pub use document::{
    ClientDocument, DocumentError, MAX_ACTION_INPUT_BYTES, MAX_CONTRIBUTIONS, MAX_CONTROLS,
    MAX_DEPTH, MAX_DOCUMENT_BYTES, MAX_METHODS,
};

pub use catalog::{
    CatalogError, ClientCatalog, ClientInstallation, MAX_CATALOG_BYTES, MAX_CATALOG_INSTALLATIONS,
    MAX_CATALOG_THEMES,
};
