//! Denylist names retain future tools on disk without pretending they exist.

use std::{fmt, str::FromStr};

use super::{InvalidMcp, ToolId};

/// Byte cap for a tool name, bounding stored future identifiers too.
pub const MAX_TOOL_NAME_BYTES: usize = 128;

/// A validated future tool name, never used as an executable or path.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
pub struct ToolName(String);

impl ToolName {
    /// Text retained across older-node settings round trips.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for ToolName {
    type Err = InvalidMcp;
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        if text.is_empty()
            || text.len() > MAX_TOOL_NAME_BYTES
            || !text
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"_.:-".contains(&byte))
        {
            return Err(InvalidMcp::ToolName);
        }
        Ok(Self(text.to_owned()))
    }
}

impl fmt::Display for ToolName {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.write_str(self.as_str())
    }
}

/// A stored denylist entry: known here or retained for a newer build.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
pub enum StoredTool {
    /// A tool whose dispatch and policy share one identifier table.
    Known(ToolId),
    /// A future identifier retained on reads but refused in new writes.
    Unknown(ToolName),
}

impl StoredTool {
    /// The spelling that survives a settings read and write.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Known(tool) => tool.as_str(),
            Self::Unknown(name) => name.as_str(),
        }
    }
}

impl FromStr for StoredTool {
    type Err = InvalidMcp;
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match ToolId::parse(text) {
            Some(tool) => Ok(Self::Known(tool)),
            None => text.parse().map(Self::Unknown),
        }
    }
}

impl fmt::Display for StoredTool {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.write_str(self.as_str())
    }
}

crate::key::serde_via_text!(ToolName, StoredTool);
