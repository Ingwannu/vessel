//! Bundled skill mechanisms; node policy owns authorization and serialization.
//! This module neither reads environment nor chooses an account's agent homes.

mod bundle;

pub use bundle::{SkillAsset, SkillBundle, SkillBundles};

/// A bounded, portable bundled-skill operation failed.
#[derive(Debug, thiserror::Error)]
pub enum SkillError {
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
}
