//! Plugin identity and native declarative contracts (ADR 0079).
//!
//! These values perform no I/O and grant no authority. Calls stay unregistered
//! until their node service, clients and shared pending-method mechanism exist.

mod address;
mod attachment;
mod identity;
mod locale;
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
    Caption, ContentText, ContributionId, FileExtension, InvalidPluginValue, PluginId, RpcName,
    ThemeName,
};
pub use locale::{InvalidLocaleTag, LocaleTag, MAX_LOCALE_TAG_BYTES};
pub use theme::{Appearance, HexColor, ThemeColors, ThemeContribution};

/// The integrator assigns this only after management, lifecycle and native
/// declarations have real consumers. No current peer may use pending calls.
pub const PLUGINS: Option<crate::ProtocolVersion> = None;
