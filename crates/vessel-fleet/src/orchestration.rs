//! Bundled skill mechanisms; node policy owns authorization and serialization.
//! This module neither reads environment nor chooses an account's agent homes.

mod bundle;
mod diagnostic;
pub use diagnostic::{ManifestDecodeError, SelectionOwnerError, SkillIoError};

pub use bundle::{SkillAsset, SkillBundle, SkillBundles};

/// Ownership metadata is reserved from compiled skill contents.
pub(crate) const MANAGED_SKILL_MANIFEST: &str = ".vessel-managed-files.json";

/// A bounded, portable bundled-skill operation failed.
#[derive(Debug, thiserror::Error)]
pub enum SkillError {
    /// External writes prevented convergence within the bounded retry budget.
    #[error("skills changed repeatedly while saving the selection")]
    Unstable,
    /// The compiled bundle must obey the same safe-name rules as fleet files.
    #[error("invalid bundled skill name")]
    Name(#[from] vessel_proto::skills::InvalidSkillName),
    /// A path is rejected before it can reach a home.
    #[error("invalid bundled skill file path")]
    Path(#[from] vessel_proto::InvalidBundlePath),
    /// A complete bundle must stay within its resource budgets.
    #[error("bundled skills exceed the named resource budget")]
    Budget,
    /// The shared bounded wire names rejected a catalogue.
    #[error("invalid skill catalogue names")]
    Names(#[from] vessel_proto::skills::InvalidSkills),
    /// Multiple assets cannot claim one portable file.
    #[error("duplicate bundled skill file")]
    Duplicate,
    /// Every selected skill needs a matching frontmatter identity.
    #[error("bundled skill has no matching SKILL.md declaration")]
    Declaration,
    /// Only explicit absolute home roots may be used.
    #[error("skill home must be an absolute path without traversal")]
    Root,
    /// A link, special file or unexpected directory blocks this operation.
    #[error("a managed skill path is not a plain file or directory")]
    Blocked,
    /// A filesystem error is retained without quoting its private path.
    #[error("skill filesystem operation failed")]
    Io(#[from] SkillIoError),
    /// A manifest must be decoded at its owning edge.
    #[error("invalid managed skill manifest")]
    Manifest(#[from] ManifestDecodeError),
    /// A future manifest cannot be overwritten as the current version.
    #[error("unsupported managed skill manifest version")]
    ManifestVersion,
    /// Selection must come from the currently shipped catalogue.
    #[error("selection names a skill this release does not ship")]
    UnknownSkill,
    /// Cross-platform or privileged mode bits cannot be guessed during recovery.
    #[error("unsupported saved skill file permissions")]
    FileMode,
    /// A durable transaction cannot be safely associated with current roots/state.
    #[error("skill transaction recovery requires its original roots and selection")]
    Recovery,
    /// Another writer recreated a name; both versions are retained for recovery.
    #[error("skill transaction conflicts with a recreated directory; originals retained")]
    RecoveryConflict,
}

mod staged;
pub use staged::{SelectionError, SkillLocations, SkillManager};

impl From<std::io::Error> for SkillError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error.into())
    }
}
impl From<serde_json::Error> for SkillError {
    fn from(error: serde_json::Error) -> Self {
        Self::Manifest(error.into())
    }
}
