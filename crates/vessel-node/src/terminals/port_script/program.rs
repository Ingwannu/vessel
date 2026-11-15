//! Resolve the approved owner at invocation, without PATH or shell lookup.

use std::path::{Path, PathBuf};

use vessel_proto::workspace::{ExecPathSyntax, PortScript, PortScriptPath};
use vessel_workspace::paths::Root;

use super::PortScriptError;

/// Prefix marker width, not a filesystem component length bound.
const NAMESPACE_PREFIX_BYTES: usize = 4;

pub(super) fn resolve(
    folder: &Path,
    program: &PortScript,
) -> Result<(PathBuf, PathBuf), PortScriptError> {
    let root = Root::open(folder).map_err(PortScriptError::Path)?;
    if cfg!(windows) {
        windows_name(program.as_str())?;
    }
    let resolved = match program.path() {
        PortScriptPath::Relative(path) => root.existing(path).map_err(PortScriptError::Path)?,
        PortScriptPath::Absolute(path) => {
            compatible(path.syntax())?;
            std::fs::canonicalize(path.as_str()).map_err(PortScriptError::Resolve)?
        }
    };
    if cfg!(windows) {
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

fn compatible(syntax: ExecPathSyntax) -> Result<(), PortScriptError> {
    match syntax {
        ExecPathSyntax::Either => Ok(()),
        ExecPathSyntax::Unix if cfg!(unix) => Ok(()),
        ExecPathSyntax::Windows if cfg!(windows) => Ok(()),
        ExecPathSyntax::Unix | ExecPathSyntax::Windows => Err(PortScriptError::Platform),
    }
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
