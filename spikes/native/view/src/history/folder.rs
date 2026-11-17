//! Paseo's `resolveDirectoryLabel` and `formatDirectoryLabel`
//! (import-session-sheet-view-model.ts 164-189): how a row names the folder
//! its session ran in, when the sheet lists the whole host.
//!
//! [Decision Log]
//! - Purpose: a full path on every row repeats the project's root; Paseo
//!   names the project the folder is in and keeps only the rest.
//! - Rule: the deepest known project root that contains the folder wins;
//!   the root itself reads as the project's name, a folder under it as
//!   "{project} · {relative}", any other folder as its full path.
//! - Choice: compare paths by component (`Path::strip_prefix`), not by
//!   text, so `/work/shop2` is not inside `/work/shop` and a trailing slash
//!   on either side changes nothing (Paseo trims trailing slashes and
//!   checks `root + "/"` for the same two reasons).
//! - Cost: the projects are the ones the app has read for the host; before
//!   they are read (or on a host without projects) every row shows its path,
//!   as Paseo's does before its project list arrives.

use std::path::Path;

/// Between a project's name and the folder under its root (Paseo's " · ").
const SEPARATOR: &str = " \u{b7} ";

/// The row's folder line for `cwd`, given each known project's root and name.
#[must_use]
pub fn folder_label<'a>(
    cwd: &Path,
    projects: impl IntoIterator<Item = (&'a Path, &'a str)>,
) -> String {
    let best = projects
        .into_iter()
        // An empty root contains nothing (Paseo skips it).
        .filter(|(root, _)| root.components().next().is_some())
        .filter_map(|(root, name)| {
            cwd.strip_prefix(root)
                .ok()
                .map(|relative| (root.components().count(), name, relative))
        })
        .max_by_key(|(depth, _, _)| *depth);
    match best {
        Some((_, name, relative)) if relative.as_os_str().is_empty() => name.to_owned(),
        Some((_, name, relative)) => format!("{name}{SEPARATOR}{}", relative.display()),
        None => cwd.display().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::folder_label;
    use std::path::Path;

    fn label(cwd: &str) -> String {
        let projects = [
            (Path::new("/work/shop"), "shop"),
            (Path::new("/work/atlas/"), "atlas"),
            (Path::new("/work/atlas/packages/ui"), "ui-kit"),
            (Path::new(""), "nothing"),
        ];
        folder_label(Path::new(cwd), projects)
    }

    #[test]
    fn a_folder_inside_a_project_names_the_project_and_the_rest() {
        assert_eq!(label("/work/shop/src/app"), "shop \u{b7} src/app");
    }

    #[test]
    fn a_project_root_reads_as_its_name_with_or_without_a_trailing_slash() {
        assert_eq!(label("/work/shop"), "shop");
        assert_eq!(label("/work/shop/"), "shop");
        assert_eq!(label("/work/atlas"), "atlas");
    }

    #[test]
    fn the_deepest_project_wins() {
        assert_eq!(label("/work/atlas/packages/ui/src"), "ui-kit \u{b7} src");
        assert_eq!(label("/work/atlas/docs"), "atlas \u{b7} docs");
    }

    #[test]
    fn any_other_folder_keeps_its_full_path() {
        assert_eq!(label("/work/shop2/src"), "/work/shop2/src");
        assert_eq!(label("/elsewhere"), "/elsewhere");
        assert_eq!(folder_label(Path::new("/work/shop/x"), []), "/work/shop/x");
    }
}
