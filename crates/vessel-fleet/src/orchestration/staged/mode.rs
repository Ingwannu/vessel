//! A rollback retains prior permission bits without accepting special privilege bits.

use crate::orchestration::SkillError;
use serde::{Deserialize, Serialize};

const PERMISSION_BITS: u32 = 0o777;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u32", into = "u32")]
pub(super) struct UnixMode(u32);
impl TryFrom<u32> for UnixMode {
    type Error = SkillError;
    fn try_from(bits: u32) -> Result<Self, SkillError> {
        if bits > PERMISSION_BITS {
            return Err(SkillError::FileMode);
        }
        Ok(Self(bits))
    }
}
impl From<UnixMode> for u32 {
    fn from(mode: UnixMode) -> Self {
        mode.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "platform", content = "permissions", rename_all = "snake_case")]
pub(super) enum SavedMode {
    Unix(UnixMode),
    Inherited,
}

impl SavedMode {
    pub(super) fn private() -> Result<Self, SkillError> {
        #[cfg(unix)]
        {
            Ok(Self::Unix(UnixMode::try_from(0o600)?))
        }
        #[cfg(not(unix))]
        {
            Ok(Self::Inherited)
        }
    }

    pub(super) fn read(file: &std::fs::File) -> Result<Self, SkillError> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            Ok(Self::Unix(UnixMode::try_from(
                file.metadata()?.permissions().mode() & PERMISSION_BITS,
            )?))
        }
        #[cfg(not(unix))]
        {
            let _metadata = file.metadata()?;
            Ok(Self::Inherited)
        }
    }
    pub(super) fn restore(self, file: &std::fs::File) -> Result<(), SkillError> {
        match self {
            Self::Unix(mode) => {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt as _;
                    file.set_permissions(std::fs::Permissions::from_mode(mode.0))?;
                }
                #[cfg(not(unix))]
                {
                    let _mode = mode;
                    return Err(SkillError::FileMode);
                }
            }
            Self::Inherited => {
                #[cfg(unix)]
                {
                    return Err(SkillError::FileMode);
                }
                #[cfg(not(unix))]
                {
                    let _metadata = file.metadata()?;
                }
            }
        }
        Ok(())
    }
}
