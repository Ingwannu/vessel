//! Approved port-helper paths, decoded without resolving or invoking programs.
//! A private owner prevents a folder-root `RelPath` from becoming an executable.

use std::fmt;
use std::str::FromStr;

use super::{ExecPath, InvalidExecPath, InvalidRelPath, RelPath, trim_project_text};

/// A literal program selection whose path shape is validated before approval.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
pub struct PortScript(Program);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Program {
    Relative(RelPath),
    Absolute(ExecPath),
}

/// Inspect an approved selection; execution must take the owning `PortScript`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortScriptPath<'a> {
    /// Resolve beneath the checkout's own root, including a worktree's copy.
    Relative(&'a RelPath),
    /// Check native compatibility before invoking this approved absolute path.
    Absolute(&'a ExecPath),
}

/// Why project text cannot select a port-helper program.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum InvalidPortScript {
    /// A folder root cannot identify an executable.
    #[error("a port script must name a program, not an empty path")]
    Empty,
    /// Preserve the containment grammar's reason for refusing a relative name.
    #[error("a relative port-script path is invalid: {0}")]
    Relative(#[source] InvalidRelPath),
    /// Preserve an executable path's reason without normalizing its components.
    #[error("a port-script path is invalid: {0}")]
    Path(#[source] InvalidExecPath),
}

impl PortScript {
    /// Keep path inspection exhaustive without allowing unchecked construction.
    #[must_use]
    pub const fn path(&self) -> PortScriptPath<'_> {
        match &self.0 {
            Program::Relative(path) => PortScriptPath::Relative(path),
            Program::Absolute(path) => PortScriptPath::Absolute(path),
        }
    }

    /// The trimmed text the approval digest will cover, never a resolved filename.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match &self.0 {
            Program::Relative(path) => path.as_str(),
            Program::Absolute(path) => path.as_str(),
        }
    }
}

impl FromStr for PortScript {
    type Err = InvalidPortScript;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let text = trim_project_text(text);
        if text.is_empty() {
            return Err(InvalidPortScript::Empty);
        }
        let program = match text.parse::<ExecPath>() {
            Ok(path) => Program::Absolute(path),
            // An invalid absolute path must not be reinterpreted as relative.
            Err(InvalidExecPath::NotAbsolute) => {
                Program::Relative(text.parse().map_err(InvalidPortScript::Relative)?)
            }
            Err(error) => return Err(InvalidPortScript::Path(error)),
        };
        Ok(Self(program))
    }
}

impl fmt::Display for PortScript {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

crate::key::serde_via_text!(PortScript);

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error as _;

    #[test]
    fn relative_and_foreign_absolute_programs_remain_literal() {
        for text in ["port-tool", "tools/port-tool"] {
            let program: PortScript = text.parse().unwrap();
            assert_eq!(program.as_str(), text);
            assert!(matches!(program.path(), PortScriptPath::Relative(path) if !path.is_root()));
            let json = serde_json::to_string(&program).unwrap();
            assert_eq!(serde_json::from_str::<PortScript>(&json).unwrap(), program);
        }
        for text in [
            "/tools/port-tool",
            r"C:\tools\port.exe",
            r"\\host\share\port.exe",
        ] {
            let program: PortScript = text.parse().unwrap();
            assert_eq!(program.as_str(), text);
            assert!(matches!(program.path(), PortScriptPath::Absolute(_)));
            let json = serde_json::to_string(&program).unwrap();
            assert_eq!(serde_json::from_str::<PortScript>(&json).unwrap(), program);
        }
    }

    #[test]
    fn javascript_edge_trim_does_not_rewrite_the_program_name() {
        let program: PortScript = "\u{feff}\u{2028}tools/port tool\u{2029}\n".parse().unwrap();
        assert_eq!(program.as_str(), "tools/port tool");
        assert_eq!(program.to_string(), "tools/port tool");
        assert!("\u{0085}tools/port".parse::<PortScript>().is_err());
        // A lexical value is not a claim that Windows will run a batch file.
        assert!(r"C:\tools\port.CmD".parse::<PortScript>().is_ok());
    }

    #[test]
    fn neither_text_nor_serde_can_insert_a_root_or_unsafe_components() {
        for text in [
            "",
            " \u{feff}\n",
            ".",
            "../port",
            "tools//port",
            "tools/./port",
            ".git/port",
            "tools\\port",
            "C:port",
            "/",
            "/tools/../port",
            "tools/po\nrt",
            "tools/po\rrt",
            "tools/po\u{001b}rt",
            "tools/po\0rt",
        ] {
            assert!(text.parse::<PortScript>().is_err(), "{text:?}");
            let json = serde_json::to_string(text).unwrap();
            assert!(
                serde_json::from_str::<PortScript>(&json).is_err(),
                "{text:?}"
            );
        }
    }

    #[test]
    fn path_refusals_keep_their_original_source() {
        let relative = "../port".parse::<PortScript>().unwrap_err();
        assert_eq!(
            relative.source().unwrap().downcast_ref(),
            Some(&InvalidRelPath::Part)
        );
        let absolute = "/tools/../port".parse::<PortScript>().unwrap_err();
        assert_eq!(
            absolute.source().unwrap().downcast_ref(),
            Some(&InvalidExecPath::Component)
        );
    }
}
