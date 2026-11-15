//! One shared directory-name invariant for fleet and bundled agent skills.

use crate::BundlePath;
use std::{fmt, str::FromStr};

/// A skill's portable directory name, excluding harness-private dot directories.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
pub struct SkillName(String);

/// A directory name cannot be used safely in all supported agent homes.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("skill name must be one portable non-dot directory component")]
pub struct InvalidSkillName;

impl SkillName {
    /// The validated directory component; never an arbitrary path.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl FromStr for SkillName {
    type Err = InvalidSkillName;
    fn from_str(name: &str) -> Result<Self, Self::Err> {
        const COMPONENTS: usize = 2;
        let portable = !name.is_empty()
            && !name.starts_with('.')
            && !name.contains(['/', '\\'])
            && format!("skills/{name}")
                .parse::<BundlePath>()
                .is_ok_and(|path| path.segments().count() == COMPONENTS);
        if !portable {
            return Err(InvalidSkillName);
        }
        Ok(Self(name.to_owned()))
    }
}
impl fmt::Display for SkillName {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.write_str(self.as_str())
    }
}
// Preserve fleet's textual comparison contract after sharing the name value.
impl PartialEq<str> for SkillName {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}
crate::key::serde_via_text!(SkillName);
