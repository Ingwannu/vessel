//! Which targets this computer has (Paseo's `isInstalled`,
//! `supportsWorkspace` and `listAvailableEditorTargets`), read through a
//! [`Probe`] the app makes from the computer and tests make from a fake.

use std::io;
use std::path::{Path, PathBuf};

use super::commands::{Candidate, Env, candidates, mac_app, mac_bundles, name_forms, path_folders};
use super::{Platform, TargetId};

/// A folder entry's kind, for Xcode's look at a workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    /// A file.
    File,
    /// A folder (an `.xcodeproj` is one).
    Directory,
    /// Anything else.
    Other,
}

/// A folder entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// Its name.
    pub name: String,
    /// What it is.
    pub kind: EntryKind,
}

/// What the rules read from this computer.
pub trait Probe {
    /// The platform's rules.
    fn platform(&self) -> Platform;
    /// The variables a lookup reads.
    fn env(&self) -> &Env;
    /// Whether `path` exists (an app bundle is a folder).
    fn exists(&self, path: &Path) -> bool;
    /// Whether `path` is a file (a command; a folder of that name is not).
    fn is_file(&self, path: &Path) -> bool;
    /// `dir`'s entries.
    ///
    /// # Errors
    ///
    /// When the folder cannot be read.
    fn entries(&self, dir: &Path) -> io::Result<Vec<Entry>>;
    /// `path` with its links resolved, as the system names it.
    ///
    /// # Errors
    ///
    /// When it does not exist or cannot be read.
    fn canonical(&self, path: &Path) -> io::Result<PathBuf>;
}

/// `target`'s command on this computer: the first candidate found
/// (Paseo's `resolveCommand`).
#[must_use]
pub fn command(target: TargetId, probe: &dyn Probe) -> Option<PathBuf> {
    let platform = probe.platform();
    let env = probe.env();
    let folders = path_folders(platform, env);
    candidates(target, platform, env)
        .into_iter()
        .find_map(|candidate| match candidate {
            Candidate::Path(path) => probe.is_file(&path).then_some(path),
            Candidate::Name(name) => folders.iter().find_map(|folder| {
                name_forms(name, platform)
                    .into_iter()
                    .map(|form| folder.join(form))
                    .find(|path| probe.is_file(path))
            }),
        })
}

/// `target`'s macOS app bundle on this computer.
#[must_use]
pub fn mac_bundle(target: TargetId, probe: &dyn Probe) -> Option<PathBuf> {
    let app = mac_app(target)?;
    mac_bundles(app, probe.platform(), probe.env())
        .into_iter()
        .find(|bundle| probe.exists(bundle))
}

/// Whether this computer has `target`.
#[must_use]
pub fn installed(target: TargetId, probe: &dyn Probe) -> bool {
    let platform = probe.platform();
    match target {
        TargetId::Finder => platform == Platform::MacOs,
        TargetId::Explorer => platform == Platform::Windows,
        TargetId::FileManager => platform == Platform::Other,
        TargetId::Xcode => mac_bundle(target, probe).is_some(),
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
        | TargetId::WebStorm => {
            command(target, probe).is_some() || mac_bundle(target, probe).is_some()
        }
    }
}

/// The targets this computer has, in Paseo's order. Xcode is kept only
/// for an absolute `workspace` holding a document it opens.
#[must_use]
pub fn list(probe: &dyn Probe, workspace: Option<&Path>) -> Vec<TargetId> {
    let workspace = workspace.filter(|workspace| workspace.is_absolute());
    TargetId::ALL
        .into_iter()
        .filter(|target| installed(*target, probe))
        .filter(|target| match target {
            // Discovery is optional: an unreadable folder leaves Xcode out
            // (Paseo's `isUnavailableDirectoryError`).
            TargetId::Xcode => workspace
                .and_then(|workspace| xcode_document(workspace, probe).ok().flatten())
                .is_some(),
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
            | TargetId::WebStorm
            | TargetId::Finder
            | TargetId::Explorer
            | TargetId::FileManager => true,
        })
        .collect()
}

/// What Xcode is handed for `workspace` (Paseo's `findXcodeDocument`): an
/// `.xcworkspace`, else an `.xcodeproj`, else `Package.swift`. Among
/// several, the one named after the folder, else the first by name.
///
/// # Errors
///
/// When the folder cannot be read.
pub fn xcode_document(workspace: &Path, probe: &dyn Probe) -> io::Result<Option<PathBuf>> {
    let entries = probe.entries(workspace)?;
    let folder = workspace
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let pick = |extension: &str| -> Option<String> {
        let mut matches: Vec<&str> = entries
            .iter()
            .filter(|entry| entry.kind == EntryKind::Directory && entry.name.ends_with(extension))
            .map(|entry| entry.name.as_str())
            .collect();
        matches.sort_unstable();
        let preferred = format!("{folder}{extension}");
        if matches.contains(&preferred.as_str()) {
            return Some(preferred);
        }
        matches.first().map(|name| (*name).to_owned())
    };
    if let Some(name) = pick(".xcworkspace").or_else(|| pick(".xcodeproj")) {
        return Ok(Some(workspace.join(name)));
    }
    let manifest = entries
        .iter()
        .any(|entry| entry.kind == EntryKind::File && entry.name == "Package.swift");
    Ok(manifest.then(|| workspace.join("Package.swift")))
}
