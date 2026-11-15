//! Named launch bundle values (ADR 0076), distinct from account profiles.
//!
//! [Decision Log]
//! - Purpose: retain an ordered selection without making credentials or
//!   provider-specific JSON part of a preset.
//! - Choice: bounded immutable wrappers around the existing agent choices
//!   and identifiers; constructors and decoding share the same invariants.
//! - Constraint: these values register no call or existing reply field.
//!   Storage, catalogue probes and transport wiring remain separate work.

mod calls;
mod catalogue;
mod choices;
mod codec;
mod legacy;
mod list;
mod profile;
mod text;

pub use calls::{
    ApplicableLaunchProfiles, ApplySessionOptions, ApplySessionOptionsResult,
    GetApplicableLaunchProfiles, GetLaunchProfileOptions, LaunchConfigurationFailure,
    LaunchProfileRefusal, ListLaunchProfiles, ReplaceLaunchProfiles,
};
pub use catalogue::{LaunchAccount, LaunchAccountKind, LaunchCatalogue, LaunchCatalogueOptions};
pub use choices::{LaunchChoice, LaunchChoices};
pub use codec::LaunchPayload;
pub use legacy::{LegacyFavorite, LegacyFavorites, LegacyModelKey, LegacyProviderKey};
pub use list::{LaunchProfiles, LaunchProfilesSnapshot};
pub use profile::{LaunchProfile, LaunchProfileRevision, LaunchSelection};
pub use text::{
    ConfigurationNotice, LaunchAppearanceKey, LaunchColorKey, LaunchIconKey, LaunchProfileName,
    LaunchProfileNotes,
};

/// Unassigned until reviewed node, client, CLI and consumer wiring exists.
/// Quality B's announced method table is required before registering calls.
pub const NAMED_LAUNCH_PROFILES: Option<crate::ProtocolVersion> = None;

/// A structural refusal; no submitted text or account value is echoed.
#[derive(Debug, thiserror::Error)]
pub enum InvalidLaunchValue {
    /// A name must remain readable on one line.
    #[error(
        "a launch name must be nonblank, on one line, at most {} characters",
        LaunchProfileName::MAX_CHARS
    )]
    Name,
    /// Notes are public guidance, never arbitrary control sequences.
    #[error(
        "launch notes must be nonblank, without controls except line breaks/tabs, at most {} bytes",
        LaunchProfileNotes::MAX_BYTES
    )]
    Notes,
    /// Configuration notices have their own finite public-text boundary.
    #[error(
        "a configuration notice must be nonblank, without controls except line breaks/tabs, at most {} bytes",
        ConfigurationNotice::MAX_BYTES
    )]
    Notice,
    /// Appearance values are registry keys, not glyphs.
    #[error(
        "a launch appearance key must be ASCII letters, digits, underscores or hyphens, at most {} bytes",
        LaunchAppearanceKey::MAX_BYTES
    )]
    Appearance,
    /// Agent ids remain opaque, but bounded and nonempty.
    #[error(
        "a launch choice id must be nonempty without controls, at most {} bytes",
        LaunchChoice::MAX_ID_BYTES
    )]
    ChoiceId,
    /// Source model/provider identities stay opaque and bounded.
    #[error(
        "a legacy provider/model key must be nonempty without controls and at most {} bytes",
        LaunchChoice::MAX_ID_BYTES
    )]
    LegacyKey,
    /// Source inputs are never silently truncated before deduplication.
    #[error("legacy favorites exceed {} entries", LegacyFavorites::MAX_ENTRIES)]
    LegacyFavorites,
    /// The whole reported catalogue must fit its finite control/text budgets.
    #[error(
        "the reported launch catalogue exceeds its supported bounds or has inconsistent controls"
    )]
    Catalogue,
    /// One option is applied only once.
    #[error("launch choices repeat an option id")]
    DuplicateChoice,
    /// No unbounded configuration bundle.
    #[error("a launch selection exceeds {} choices", LaunchChoices::MAX_CHOICES)]
    Choices,
    /// Stable ids, rather than names, distinguish rows.
    #[error("launch profiles repeat an id")]
    DuplicateProfile,
    /// Refuse rather than return a truncated list.
    #[error("a launch list exceeds {} profiles", LaunchProfiles::MAX_PROFILES)]
    Profiles,
    /// Escaping and the maximum revision are included in this bound.
    #[error("a launch payload exceeds {} JSON bytes", LaunchPayload::MAX_BYTES)]
    Payload,
    /// Preserve the serializer's cause when it is not the byte cap.
    #[error("cannot encode launch values")]
    Encoding(#[source] serde_json::Error),
    /// Decoder diagnostics may contain untrusted field names/text.
    #[error("invalid launch value or unknown field")]
    Malformed,
}

#[cfg(test)]
mod tests;
