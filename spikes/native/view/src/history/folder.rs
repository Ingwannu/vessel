//! Paseo's `resolveDirectoryLabel` and `formatDirectoryLabel`
//! (import-session-sheet-view-model.ts 164-189): how a row names the folder
//! its session ran in, when the sheet lists the whole host.
//!
//! [Decision Log]
//! - Purpose: a full path on every row repeats the project's root; Paseo
//!   names the project the folder is in and keeps only the rest.
//! - Rule: the deepest known project root that holds the folder wins; the
//!   root itself reads as the project's name, a folder under it as
//!   "{project} · {relative}", any other folder as its full path.
//! - Existing asset: `workspace::projects::project_among` already finds the
//!   deepest project holding a folder for the composer's tray (text compare,
//!   trailing slashes trimmed, `/work/shop2` not inside `/work/shop`). The
//!   label reuses it, so the tray and the sheet cannot disagree.
//! - Cost: the projects are the ones the app has read for the host; before
//!   they are read (or on a host without projects) every row shows its path,
//!   as Paseo's does before its project list arrives.

use crate::workspace::projects::project_among;
use std::path::Path;
use vessel_proto::workspace::ProjectInfo;

/// Between a project's name and the folder under its root (Paseo's " · ").
const SEPARATOR: &str = " \u{b7} ";
/// Either separator a stored path may end in (as `project_among` trims).
const SLASHES: [char; 2] = ['/', '\\'];

/// The row's folder line for `cwd`, given the host's projects.
#[must_use]
pub fn folder_label(cwd: &Path, projects: &[ProjectInfo]) -> String {
    let Some(project) = project_among(projects, Some(cwd)) else {
        return cwd.display().to_string();
    };
    let folder = cwd.to_string_lossy();
    let root = project.root.to_string_lossy();
    let relative = folder
        .trim_end_matches(SLASHES)
        .strip_prefix(root.trim_end_matches(SLASHES))
        .unwrap_or_default()
        .trim_start_matches(SLASHES);
    let name = project.name.as_str();
    if relative.is_empty() {
        name.to_owned()
    } else {
        format!("{name}{SEPARATOR}{relative}")
    }
}

#[cfg(test)]
mod tests;
