//! Resolve the approved owner at invocation, without PATH or shell lookup.

use std::path::{Path, PathBuf};

use vessel_proto::workspace::{ExecPathSyntax, PortScript, PortScriptPath};
use vessel_workspace::paths::Root;

use super::PortScriptError;

/// Prefix marker width, not a filesystem component length bound.
const NAMESPACE_PREFIX_BYTES: usize = 4;

/// Interpret the approved owner only using the node's native grammar.
pub(super) fn resolve(
    folder: &Path,
    program: &PortScript,
) -> Result<(PathBuf, PathBuf), PortScriptError> {
    resolve_on(Platform::current()?, folder, program)
}

/// The path policy is explicit so both native grammars are tested on each host.
#[derive(Clone, Copy)]
pub(super) enum Platform {
    /// Unix preserves backslashes as filename characters.
    Unix,
    /// Windows may otherwise insert a command interpreter for batch files.
    Windows,
}

impl Platform {
    fn current() -> Result<Self, PortScriptError> {
        if cfg!(windows) {
            Ok(Self::Windows)
        } else if cfg!(unix) {
            Ok(Self::Unix)
        } else {
            Err(PortScriptError::Platform)
        }
    }

    fn compatible(self, syntax: ExecPathSyntax) -> Result<(), PortScriptError> {
        match (self, syntax) {
            (Self::Unix, ExecPathSyntax::Unix | ExecPathSyntax::Either)
            | (Self::Windows, ExecPathSyntax::Windows | ExecPathSyntax::Either) => Ok(()),
            (Self::Unix, ExecPathSyntax::Windows) | (Self::Windows, ExecPathSyntax::Unix) => {
                Err(PortScriptError::Platform)
            }
        }
    }
}

/// Follow real filesystem links, then recheck the final native program name.
pub(super) fn resolve_on(
    platform: Platform,
    folder: &Path,
    program: &PortScript,
) -> Result<(PathBuf, PathBuf), PortScriptError> {
    let root = Root::open(folder).map_err(PortScriptError::Path)?;
    if matches!(platform, Platform::Windows) {
        windows_name(program.as_str())?;
    }
    let resolved = match program.path() {
        PortScriptPath::Relative(path) => root.existing(path).map_err(PortScriptError::Path)?,
        PortScriptPath::Absolute(path) => {
            platform.compatible(path.syntax())?;
            std::fs::canonicalize(path.as_str()).map_err(PortScriptError::Resolve)?
        }
    };
    if matches!(platform, Platform::Windows) {
        windows_name(resolved.to_str().ok_or(PortScriptError::NativePath)?)?;
    }
    if !std::fs::metadata(&resolved)
        .map_err(PortScriptError::Resolve)?
        .is_file()
    {
        return Err(PortScriptError::NotFile);
    }
    Ok((resolved, root.path().to_path_buf()))
}

/// Check both approved and canonical names: Windows may otherwise insert cmd.exe.
pub(super) fn windows_name(text: &str) -> Result<(), PortScriptError> {
    let file = text
        .rsplit(['/', '\\'])
        .next()
        .ok_or(PortScriptError::WindowsPath)?;
    let normal = file.trim_end_matches(['.', ' ']);
    if let Some((_, extension)) = normal.rsplit_once('.')
        && (extension.eq_ignore_ascii_case("cmd") || extension.eq_ignore_ascii_case("bat"))
    {
        return Err(PortScriptError::WindowsCommandScript);
    }
    // Namespace markers are roots, not dot-named filesystem components.
    let body = match text.as_bytes().get(..NAMESPACE_PREFIX_BYTES) {
        Some([b'/' | b'\\', b'/' | b'\\', b'.' | b'?', b'/' | b'\\']) => text
            .get(NAMESPACE_PREFIX_BYTES..)
            .ok_or(PortScriptError::WindowsPath)?,
        _ => text,
    };
    for (index, component) in body.split(['/', '\\']).enumerate() {
        let drive = index == 0
            && component.strip_suffix(':').is_some_and(|letter| {
                let mut chars = letter.chars();
                chars.next().is_some_and(|c| c.is_ascii_alphabetic()) && chars.next().is_none()
            });
        if component.trim_end_matches(['.', ' ']) != component
            || (component.contains(':') && !drive)
        {
            return Err(PortScriptError::WindowsPath);
        }
    }
    Ok(())
}
