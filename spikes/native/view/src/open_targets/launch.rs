//! What opening a target starts (Paseo's per-target `launch`): the
//! command with its arguments, macOS's `open -a` for an app without a
//! command, or the file manager's reveal and open.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use super::guard::Checked;
use super::listing::{command, mac_bundle, xcode_document};
use super::{Probe, TargetId};

/// What to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Plan {
    /// Start `program` with `args`, detached.
    Start {
        /// The command, resolved once for this launch.
        program: PathBuf,
        /// Its arguments.
        args: Vec<OsString>,
    },
    /// Show the entry selected in the file manager.
    Reveal(PathBuf),
    /// Open the folder in the file manager.
    OpenFolder(PathBuf),
}

/// Why nothing could be started.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unavailable {
    /// The target is not on this computer (any more).
    NotInstalled(TargetId),
    /// Xcode found nothing it opens in the folder.
    NoXcodeDocument(PathBuf),
}

/// macOS's launcher of app bundles.
const OPEN: &str = "/usr/bin/open";

/// What opening `request` in `target` starts.
///
/// # Errors
///
/// The target is gone, or Xcode has no document in the folder.
pub fn plan(target: TargetId, request: &Checked, probe: &dyn Probe) -> Result<Plan, Unavailable> {
    match target {
        TargetId::Finder | TargetId::Explorer | TargetId::FileManager => {
            return Ok(request.file.as_ref().map_or_else(
                || Plan::OpenFolder(request.folder.clone()),
                |file| Plan::Reveal(file.clone()),
            ));
        }
        TargetId::Xcode => return xcode(request, probe),
        TargetId::Cursor
        | TargetId::Trae
        | TargetId::Kiro
        | TargetId::VsCode
        | TargetId::VsCodeInsiders
        | TargetId::VsCodium
        | TargetId::Zed
        | TargetId::Antigravity
        | TargetId::AndroidStudio
        | TargetId::IntellijIdea
        | TargetId::Aqua
        | TargetId::Clion
        | TargetId::DataGrip
        | TargetId::DataSpell
        | TargetId::GoLand
        | TargetId::PhpStorm
        | TargetId::PyCharm
        | TargetId::Rider
        | TargetId::RubyMine
        | TargetId::RustRover
        | TargetId::WebStorm => {}
    }
    if let Some(program) = command(target, probe) {
        return Ok(Plan::Start {
            program,
            args: args(target, request),
        });
    }
    if let Some(bundle) = mac_bundle(target, probe) {
        let mut paths = vec![request.folder.clone()];
        paths.extend(request.file.clone());
        return Ok(open_app(&bundle, &paths));
    }
    Err(Unavailable::NotInstalled(target))
}

/// Xcode opens the folder's document (and the file beside it, without its
/// line), through its bundle.
fn xcode(request: &Checked, probe: &dyn Probe) -> Result<Plan, Unavailable> {
    let document = xcode_document(&request.folder, probe)
        .ok()
        .flatten()
        .ok_or_else(|| Unavailable::NoXcodeDocument(request.folder.clone()))?;
    let bundle =
        mac_bundle(TargetId::Xcode, probe).ok_or(Unavailable::NotInstalled(TargetId::Xcode))?;
    let mut paths = vec![document];
    paths.extend(request.file.clone());
    Ok(open_app(&bundle, &paths))
}

/// `open -a <bundle> <paths…>`.
fn open_app(bundle: &Path, paths: &[PathBuf]) -> Plan {
    let mut args = vec![OsString::from("-a"), bundle.as_os_str().to_owned()];
    args.extend(paths.iter().map(|path| path.as_os_str().to_owned()));
    Plan::Start {
        program: PathBuf::from(OPEN),
        args,
    }
}

/// `file`, `file:line` or `file:line:column` (Paseo's `location`).
fn location(file: &Path, line: Option<u32>, column: Option<u32>) -> OsString {
    let mut at = file.as_os_str().to_owned();
    if let Some(line) = line {
        at.push(format!(":{line}"));
        if let Some(column) = column {
            at.push(format!(":{column}"));
        }
    }
    at
}

/// A target's arguments, in Paseo's shapes.
fn args(target: TargetId, request: &Checked) -> Vec<OsString> {
    let folder = request.folder.as_os_str().to_owned();
    let Some(file) = request.file.as_deref() else {
        return if target == TargetId::Kiro {
            vec!["ide".into(), folder]
        } else {
            vec![folder]
        };
    };
    match target {
        // `vscode.ts`: `<ws>`, `<ws> <file>` or `<ws> --goto file:line[:col]`.
        TargetId::Cursor
        | TargetId::Trae
        | TargetId::VsCode
        | TargetId::VsCodeInsiders
        | TargetId::VsCodium
        | TargetId::Antigravity
        | TargetId::Kiro => {
            let mut args = Vec::new();
            if target == TargetId::Kiro {
                args.push("ide".into());
            }
            args.push(folder);
            if request.line.is_some() {
                args.push("--goto".into());
                args.push(location(file, request.line, request.column));
            } else {
                args.push(file.as_os_str().to_owned());
            }
            args
        }
        // `zed.ts`: `<ws> file[:line[:col]]`.
        TargetId::Zed => vec![folder, location(file, request.line, request.column)],
        // `android-studio.ts`: `<ws> [--line N] [--column N] <file>`.
        TargetId::AndroidStudio => {
            let mut args = vec![folder];
            args.extend(line_flags(request));
            args.push(file.as_os_str().to_owned());
            args
        }
        // `intellij-idea.ts`: `[--line N] [--column N] <ws> <file>`.
        TargetId::IntellijIdea
        | TargetId::Aqua
        | TargetId::Clion
        | TargetId::DataGrip
        | TargetId::DataSpell
        | TargetId::GoLand
        | TargetId::PhpStorm
        | TargetId::PyCharm
        | TargetId::Rider
        | TargetId::RubyMine
        | TargetId::RustRover
        | TargetId::WebStorm => {
            let mut args = line_flags(request);
            args.push(folder);
            args.push(file.as_os_str().to_owned());
            args
        }
        // Planned elsewhere (`plan`): never started with arguments.
        TargetId::Xcode | TargetId::Finder | TargetId::Explorer | TargetId::FileManager => {
            vec![folder]
        }
    }
}

/// JetBrains' `--line N` and `--column N`, each when given.
fn line_flags(request: &Checked) -> Vec<OsString> {
    let mut flags = Vec::new();
    if let Some(line) = request.line {
        flags.extend(["--line".into(), line.to_string().into()]);
    }
    if let Some(column) = request.column {
        flags.extend(["--column".into(), column.to_string().into()]);
    }
    flags
}
