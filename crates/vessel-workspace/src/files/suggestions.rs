//! Directory pickers reuse file listing's root checks and git ignore
//! classification. Ignored trees are pruned before traversal, rather
//! than filtering results after spending the scan budget inside them.

use std::cmp::Reverse;
use std::collections::VecDeque;

use vessel_proto::workspace::EntryKind;
use vessel_proto::workspace::suggestions::{
    EntrySuggestions, SuggestEntries, SuggestedEntry, SuggestedEntryKind, SuggestionKinds,
    SuggestionMatch,
};

use super::{FileError, list, list_marked};
use crate::{Git, paths::Root};

/// Paseo's maximum discovery depth.
pub const MAX_SUGGESTION_DEPTH: usize = 12;
/// Paseo's maximum entries inspected in one discovery request.
pub const MAX_SCANNED_ENTRIES: usize = 20_000;
/// Hidden directories Paseo deliberately searches inside a workspace.
const SEARCHABLE_HIDDEN: &[&str] = &[
    ".agents",
    ".claude",
    ".codex",
    ".cursor",
    ".github",
    ".opencode",
    ".paseo",
    ".vscode",
];
/// Generated trees excluded by the reference even outside git repositories.
const IGNORED_NAMES: &[&str] = &[
    "node_modules",
    "venv",
    "env",
    "virtualenv",
    "dist",
    "build",
    "target",
    "out",
    "coverage",
    "vendor",
    "__pycache__",
    ".git",
];

/// Discover usable paths within the caller's already-authorized root.
///
/// # Errors
/// Invalid result limits and unreadable directories fail explicitly.
pub async fn suggest(
    git: Option<&Git>,
    root: &Root,
    request: &SuggestEntries,
) -> Result<EntrySuggestions, FileError> {
    let limit = request.limit.unwrap_or_default().get();
    let query = request.query.as_str().to_lowercase();
    let needle: Vec<char> = query.chars().filter(|c| !c.is_whitespace()).collect();
    let mut pending = VecDeque::from([(request.directory.clone(), 0_usize)]);
    let mut scanned = 0_usize;
    let mut more = false;
    let mut matches = Vec::new();
    while let Some((directory, depth)) = pending.pop_front() {
        let listing = match git {
            Some(git) => list_marked(git, root, &directory, false).await?,
            None => list(root, &directory).await?,
        };
        more |= listing.more;
        for entry in listing.entries {
            scanned = scanned.saturating_add(1);
            if scanned > MAX_SCANNED_ENTRIES {
                more = true;
                break;
            }
            if !discoverable(&entry.name, entry.kind) {
                continue;
            }
            let path = directory
                .child(&entry.name)
                .map_err(FileError::InvalidEntry)?;
            if entry.kind == EntryKind::Dir && !query.is_empty() {
                if depth < MAX_SUGGESTION_DEPTH {
                    pending.push_back((path.clone(), depth.saturating_add(1)));
                } else {
                    more = true;
                }
            }
            if accepts(request.kinds, entry.kind) {
                let lower = path.as_str().to_lowercase();
                let score = match request.matching {
                    SuggestionMatch::Fuzzy => crate::find::score(&needle, &lower, path.as_str()),
                    SuggestionMatch::Suffix => lower.ends_with(&query).then_some(0),
                };
                if let Some(score) = score {
                    let kind = match entry.kind {
                        EntryKind::Dir => SuggestedEntryKind::Directory,
                        EntryKind::File => SuggestedEntryKind::File,
                        EntryKind::Symlink | EntryKind::Other | EntryKind::Unknown => continue,
                    };
                    matches.push((Reverse(score), path, kind));
                }
            }
        }
        if scanned > MAX_SCANNED_ENTRIES {
            break;
        }
    }
    matches.sort_by(|a, b| (&a.0, &a.1).cmp(&(&b.0, &b.1)));
    more |= matches.len() > usize::from(limit);
    matches.truncate(usize::from(limit));
    Ok(EntrySuggestions {
        entries: matches
            .into_iter()
            .map(|(_, path, kind)| SuggestedEntry { path, kind })
            .collect(),
        more,
    })
}

fn discoverable(name: &str, kind: EntryKind) -> bool {
    if IGNORED_NAMES.contains(&name) {
        return false;
    }
    match kind {
        EntryKind::Dir => !name.starts_with('.') || SEARCHABLE_HIDDEN.contains(&name),
        EntryKind::File => true,
        EntryKind::Symlink | EntryKind::Other | EntryKind::Unknown => false,
    }
}

const fn accepts(kinds: SuggestionKinds, kind: EntryKind) -> bool {
    match (kinds, kind) {
        (SuggestionKinds::Files | SuggestionKinds::Both, EntryKind::File)
        | (SuggestionKinds::Directories | SuggestionKinds::Both, EntryKind::Dir) => true,
        (SuggestionKinds::Files, EntryKind::Dir)
        | (SuggestionKinds::Directories, EntryKind::File)
        | (_, EntryKind::Symlink | EntryKind::Other | EntryKind::Unknown) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vessel_proto::workspace::RelPath;
    use vessel_proto::workspace::suggestions::SuggestionQuery;
    use vessel_proto::{ProjectId, workspace::WorkspaceRoot};

    #[tokio::test]
    async fn discovers_directories_and_allowlisted_hidden_trees_with_exact_limits() {
        let temp = tempfile::tempdir().unwrap();
        for name in ["src", ".github", ".private", "node_modules"] {
            std::fs::create_dir(temp.path().join(name)).unwrap();
            std::fs::write(temp.path().join(name).join("child.rs"), "x").unwrap();
        }
        let root = Root::open(temp.path()).unwrap();
        let mut request = SuggestEntries {
            root: WorkspaceRoot::Project {
                project: ProjectId::random(),
            },
            directory: RelPath::root(),
            query: SuggestionQuery::try_from(String::new()).unwrap(),
            kinds: SuggestionKinds::Directories,
            matching: SuggestionMatch::Fuzzy,
            limit: None,
        };
        let found = suggest(None, &root, &request).await.unwrap();
        assert_eq!(found.entries.len(), 2);
        request.query = SuggestionQuery::try_from("child.rs".to_owned()).unwrap();
        request.kinds = SuggestionKinds::Files;
        let found = suggest(None, &root, &request).await.unwrap();
        assert_eq!(found.entries.len(), 2);
        assert!(
            found
                .entries
                .iter()
                .all(|entry| !entry.path.as_str().contains("node_modules")
                    && !entry.path.as_str().contains(".private"))
        );
        assert!(vessel_proto::workspace::suggestions::SuggestionLimit::try_from(0).is_err());
    }
}
