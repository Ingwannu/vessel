//! Folder suggestions from the node user's home (ADR 0112): Paseo's
//! `searchDirectoryEntries` as its Add project flow calls it (no `cwd`:
//! `server/session.ts` `handleDirectorySuggestionsRequest`): the home as the
//! root, `~` as its alias, rooted path queries, nothing for a blank query,
//! no hidden folder walked, no git ignore, a confident result ending the
//! walk after 5,000 entries, `~/Library` left out on macOS.
//!
//! [Decision Log]
//! - Purpose: Add project suggests folders as the user types, on any node.
//! - Constraint: ADR 0112's conditions: folders only, Paseo's limits and
//!   skip list, a walk bounded in entries and in time, no link followed out
//!   of the home, no file name in the answer.
//! - Alternatives: Paseo's whole ranking (tiers by segment, prefix and
//!   substring); following links whose target stays inside the home.
//! - Choice: Paseo's query plan (exact path first, a typed parent anchoring
//!   the walk, a bare `~` or `~/` browsing one level), its walk in breadth
//!   order, ranked by the same scorer `file.suggest` uses (ADR 0083's
//!   accepted difference). A link is never walked: every folder the walk
//!   enters is a real folder under the home, so nothing outside is read;
//!   an exact or typed path through a link is accepted only when its real
//!   place is under the home.
//! - Cost: a folder reachable only through a link inside the home is not
//!   discovered (it is when typed); the walk ends at [`WALK_TIME`] with
//!   what it found and `more`.

use std::collections::VecDeque;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, Instant};

use vessel_proto::workspace::host_folders::{FolderSuggestions, SuggestFolders, SuggestedFolder};

use super::suggestions::{IGNORED_NAMES, MAX_SCANNED_ENTRIES, MAX_SUGGESTION_DEPTH};
use super::{FileError, blocking};

/// Paseo's `confidentResultScanThreshold` for the home.
const CONFIDENT_AFTER: usize = 5_000;
/// The walk's time bound (ADR 0112; Paseo's has none): a slow or huge
/// home answers with what was found by then.
pub const WALK_TIME: Duration = Duration::from_secs(3);

/// Folders under `home` for `request`, best first.
///
/// # Errors
/// Only a stopped blocking task: an unreadable folder is passed over, and
/// a home that cannot be resolved suggests nothing, as Paseo's.
pub async fn suggest_folders(
    home: &Path,
    request: &SuggestFolders,
) -> Result<FolderSuggestions, FileError> {
    let home = home.to_path_buf();
    let query = request.query.as_str().to_owned();
    let limit = usize::from(request.limit.unwrap_or_default().get());
    blocking(move || Ok(search(&home, &query, limit, WALK_TIME))).await
}

/// What the typed text asks for (Paseo's `parseQuery` with the rooted
/// policy and the `~` alias).
#[derive(Debug, PartialEq, Eq)]
struct Plan {
    /// Typed as a place: `~`, `~/…`, `./…` or an absolute path.
    rooted: bool,
    /// The folder part of a rooted query, relative to the home.
    parent: String,
    /// The query relative to the home, as ranked.
    normalized: String,
}

fn plan(home: &Path, query: &str) -> Option<Plan> {
    let typed = query.trim().replace('\\', "/");
    let mut rooted = false;
    let mut normalized = typed.clone();
    if normalized == "~" || normalized.starts_with("~/") {
        rooted = true;
        normalized = normalized
            .strip_prefix('~')
            .unwrap_or_default()
            .trim_start_matches('/')
            .to_owned();
    }
    if normalized.starts_with('/') {
        let browse = normalized.ends_with('/');
        let absolute = lexical(Path::new(&normalized));
        // Only a path under the home names a place here.
        let relative = absolute.strip_prefix(home).ok()?;
        rooted = true;
        normalized = slashed(relative);
        if browse && !normalized.is_empty() {
            normalized.push('/');
        }
    }
    if normalized.starts_with("./") {
        rooted = true;
    }
    normalized = collapse(normalized.trim_start_matches("./"));
    if normalized == "." && (rooted || typed == ".") {
        normalized.clear();
    }
    if normalized.is_empty() && !rooted && typed != "." {
        return None;
    }
    let parent = if rooted {
        normalized
            .rfind('/')
            .and_then(|slash| normalized.get(..slash))
            .unwrap_or_default()
            .to_owned()
    } else {
        String::new()
    };
    Some(Plan {
        rooted: rooted || normalized.is_empty(),
        parent,
        normalized,
    })
}

/// `path` with `.` dropped and `..` taken back, without reading the disk.
fn lexical(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::ParentDir => {
                let _gone = out.pop();
            }
            Component::CurDir => {}
            Component::Prefix(_) | Component::RootDir | Component::Normal(_) => {
                out.push(component);
            }
        }
    }
    out
}

/// A relative path with `/` between its parts.
fn slashed(relative: &Path) -> String {
    relative
        .components()
        .filter_map(|part| part.as_os_str().to_str())
        .collect::<Vec<_>>()
        .join("/")
}

/// Runs of `/` as one.
fn collapse(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for character in text.chars() {
        if character == '/' && out.ends_with('/') {
            continue;
        }
        out.push(character);
    }
    out
}

/// A folder found, with what orders it.
struct Found {
    score: i64,
    depth: usize,
    path: PathBuf,
}

/// The walk's state: its bounds and what it found.
struct Walk<'home> {
    home: &'home Path,
    excluded: Vec<PathBuf>,
    needle: Vec<char>,
    query: String,
    deadline: Instant,
    scanned: usize,
    more: bool,
    found: Vec<Found>,
}

fn search(home: &Path, query: &str, limit: usize, time: Duration) -> FolderSuggestions {
    let empty = FolderSuggestions {
        folders: Vec::new(),
        more: false,
    };
    let Ok(home) = fs::canonicalize(home) else {
        return empty;
    };
    let Some(plan) = plan(&home, query) else {
        return empty;
    };
    let query = plan.normalized.to_lowercase();
    let mut walk = Walk {
        home: &home,
        excluded: excluded(&home),
        needle: query.chars().filter(|c| !c.is_whitespace()).collect(),
        query,
        deadline: Instant::now()
            .checked_add(time)
            .unwrap_or_else(Instant::now),
        scanned: 0,
        more: false,
        found: Vec::new(),
    };
    let exact = (plan.rooted && !plan.normalized.is_empty())
        .then(|| inside(&home, &home.join(plan.normalized.trim_end_matches('/'))))
        .flatten();
    if plan.normalized.is_empty() {
        walk.children(&home, &home, true);
    } else if !plan.rooted || plan.parent.is_empty() {
        walk.tree(&home, &home);
    } else {
        let visible = lexical(&home.join(&plan.parent));
        match inside(&home, &visible) {
            Some(real) if walk.is_excluded(&real) => walk.children(&visible, &real, false),
            Some(real) => walk.tree(&visible, &real),
            None => walk.tree(&home, &home),
        }
    }
    walk.answer(exact.map(|_| lexical(&home.join(&plan.normalized))), limit)
}

/// `path`'s real place, when it is a folder under `home` (or the home).
fn inside(home: &Path, path: &Path) -> Option<PathBuf> {
    let real = fs::canonicalize(path).ok()?;
    (real.starts_with(home) && real.is_dir()).then_some(real)
}

/// Paseo's `excludedDiscoveryPaths`: `~/Library` on macOS, whose privacy
/// prompts block a scan.
fn excluded(home: &Path) -> Vec<PathBuf> {
    if cfg!(target_os = "macos") {
        fs::canonicalize(home.join("Library"))
            .ok()
            .into_iter()
            .collect()
    } else {
        Vec::new()
    }
}

impl Walk<'_> {
    fn is_excluded(&self, real: &Path) -> bool {
        self.excluded.iter().any(|path| real.starts_with(path))
    }

    /// The folders in `real` worth walking: real folders (never a link),
    /// not hidden, not on Paseo's skip list, not left out.
    fn folders(&mut self, real: &Path, browsing_excluded: bool) -> Vec<String> {
        let Ok(entries) = fs::read_dir(real) else {
            return Vec::new();
        };
        let mut names = Vec::new();
        for entry in entries.flatten() {
            if self.scanned >= MAX_SCANNED_ENTRIES || Instant::now() >= self.deadline {
                self.more = true;
                break;
            }
            self.scanned = self.scanned.saturating_add(1);
            let (Ok(kind), Some(name)) = (
                entry.file_type(),
                entry.file_name().to_str().map(str::to_owned),
            ) else {
                continue;
            };
            if !kind.is_dir() || name.starts_with('.') || IGNORED_NAMES.contains(&name.as_str()) {
                continue;
            }
            if !browsing_excluded && self.is_excluded(&real.join(&name)) {
                continue;
            }
            names.push(name);
        }
        names.sort();
        names
    }

    /// One level under `visible` (Paseo's `searchChildren`).
    fn children(&mut self, visible: &Path, real: &Path, browse: bool) {
        let excluded = !browse && self.is_excluded(real);
        for name in self.folders(real, excluded) {
            self.consider(visible.join(name), 1);
        }
    }

    /// The tree under `visible`, by depth (Paseo's `searchTree`).
    fn tree(&mut self, visible: &Path, real: &Path) {
        let mut pending = VecDeque::from([(visible.to_path_buf(), real.to_path_buf(), 0_usize)]);
        while let Some((visible, real, depth)) = pending.pop_front() {
            let depth = depth.saturating_add(1);
            for name in self.folders(&real, false) {
                self.consider(visible.join(&name), depth);
                if depth < MAX_SUGGESTION_DEPTH {
                    pending.push_back((visible.join(&name), real.join(&name), depth));
                }
            }
            if self.more || self.confident() {
                break;
            }
        }
    }

    /// Paseo's `hasConfidentResult` after its threshold: a folder whose
    /// name holds the query.
    fn confident(&self) -> bool {
        self.scanned >= CONFIDENT_AFTER
            && self.found.iter().any(|found| {
                found
                    .path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.to_lowercase().contains(&self.query))
            })
    }

    fn consider(&mut self, path: PathBuf, depth: usize) {
        let Some(original) = path.strip_prefix(self.home).ok().map(slashed) else {
            return;
        };
        let lower = original.to_lowercase();
        let score = if self.query.is_empty() {
            Some(0)
        } else {
            crate::find::score(&self.needle, &lower, &original)
        };
        if let Some(score) = score {
            self.found.push(Found { score, depth, path });
        }
    }

    fn answer(mut self, exact: Option<PathBuf>, limit: usize) -> FolderSuggestions {
        // Best score, then shallower, then by path.
        self.found.sort_by(|a, b| {
            b.score
                .cmp(&a.score)
                .then(a.depth.cmp(&b.depth))
                .then_with(|| a.path.cmp(&b.path))
        });
        let mut paths: Vec<PathBuf> = exact.into_iter().collect();
        for found in self.found {
            if !paths.contains(&found.path) {
                paths.push(found.path);
            }
        }
        let more = self.more || paths.len() > limit;
        paths.truncate(limit);
        FolderSuggestions {
            folders: paths
                .into_iter()
                .map(|path| SuggestedFolder {
                    shown: shown(self.home, &path),
                    path,
                })
                .collect(),
            more,
        }
    }
}

/// `path` with the home written `~`.
fn shown(home: &Path, path: &Path) -> String {
    match path.strip_prefix(home) {
        Ok(rest) if rest.as_os_str().is_empty() => "~".to_owned(),
        Ok(rest) => format!("~/{}", slashed(rest)),
        Err(_) => path.display().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn home() -> (tempfile::TempDir, PathBuf) {
        let temp = tempfile::tempdir().unwrap();
        let home = fs::canonicalize(temp.path()).unwrap();
        for folder in [
            "code/vessel/src",
            "code/paseo",
            "code/vessel/node_modules/left",
            "notes",
            ".hidden/code-secret",
        ] {
            fs::create_dir_all(home.join(folder)).unwrap();
        }
        fs::write(home.join("code/vessel/README.md"), "x").unwrap();
        (temp, home)
    }

    fn shown_of(found: &FolderSuggestions) -> Vec<&str> {
        found
            .folders
            .iter()
            .map(|folder| folder.shown.as_str())
            .collect()
    }

    #[test]
    fn a_name_finds_folders_never_files_hidden_or_skipped_ones() {
        let (_temp, home) = home();
        let found = search(&home, "vessel", 30, WALK_TIME);
        assert_eq!(shown_of(&found).first(), Some(&"~/code/vessel"));
        assert!(
            found
                .folders
                .iter()
                .all(|folder| folder.path.starts_with(&home))
        );
        let all = search(&home, "e", 100, WALK_TIME);
        let names = shown_of(&all);
        assert!(!names.iter().any(|name| name.contains("README")));
        assert!(!names.iter().any(|name| name.contains("node_modules")));
        assert!(!names.iter().any(|name| name.contains(".hidden")));
        assert!(search(&home, "  ", 30, WALK_TIME).folders.is_empty());
    }

    #[test]
    fn a_typed_place_lists_or_anchors_under_the_home_only() {
        let (_temp, home) = home();
        assert_eq!(
            shown_of(&search(&home, "~", 30, WALK_TIME)),
            ["~/code", "~/notes"]
        );
        let typed = search(&home, "~/code/pas", 30, WALK_TIME);
        assert_eq!(shown_of(&typed).first(), Some(&"~/code/paseo"));
        let absolute = format!("{}/code/", home.display());
        let listed = search(&home, &absolute, 30, WALK_TIME);
        assert_eq!(shown_of(&listed).first(), Some(&"~/code"));
        assert!(shown_of(&listed).contains(&"~/code/paseo"));
        assert!(search(&home, "/etc", 30, WALK_TIME).folders.is_empty());
        assert!(search(&home, "~/../", 30, WALK_TIME).folders.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn a_link_out_of_the_home_is_never_walked_or_named() {
        let (_temp, home) = home();
        let outside = tempfile::tempdir().unwrap();
        fs::create_dir(outside.path().join("outside-only")).unwrap();
        std::os::unix::fs::symlink(outside.path(), home.join("away")).unwrap();
        assert!(search(&home, "outside", 30, WALK_TIME).folders.is_empty());
        assert!(search(&home, "~/away/", 30, WALK_TIME).folders.is_empty());
    }

    #[test]
    fn the_limit_and_the_time_bound_say_more() {
        let (_temp, home) = home();
        let one = search(&home, "o", 1, WALK_TIME);
        assert_eq!(one.folders.len(), 1);
        assert!(one.more);
        let stopped = search(&home, "o", 30, Duration::ZERO);
        assert!(stopped.more);
    }
}
