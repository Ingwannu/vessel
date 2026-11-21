//! `git status --porcelain=v2 --branch -z`, read into [`GitStatus`].
//!
//! The porcelain format is git's stable one for scripts (its documentation,
//! "Porcelain Format Version 2"); `-z` ends every record with NUL and
//! prints paths as they are, so no quoting is undone here.

use std::path::Path;

use vessel_proto::workspace::{
    BranchName, CommitId, Distance, FileState, GitOperation, GitStatus, RefName, RelPath,
    StatusEntry,
};

use super::{Git, GitError, Limits};

/// The most entries a status holds.
pub const MAX_ENTRIES: usize = 5000;

/// What one status run printed, before the operation and the base are
/// added.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Parsed {
    pub(crate) branch: Option<BranchName>,
    pub(crate) head: Option<CommitId>,
    pub(crate) upstream: Option<Distance>,
    pub(crate) files: Vec<StatusEntry>,
    pub(crate) more: bool,
    /// Paths this version cannot name (`RelPath`'s rules), left out.
    pub(crate) unnamed: u32,
}

impl Git {
    /// The status of the repository at `dir` (untracked files listed one by
    /// one, ignored ones not).
    ///
    /// # Errors
    /// As the run's; [`GitError::Unexpected`] for output this parser does
    /// not know.
    pub async fn status(&self, dir: &Path) -> Result<GitStatus, GitError> {
        let output = self
            .command(dir)
            .args([
                "status",
                "--porcelain=v2",
                "--branch",
                "-z",
                "--untracked-files=all",
                "--ignore-submodules=dirty",
            ])
            .limits(Limits::LOCAL)
            .output()
            .await?;
        let cut = output.stdout_cut;
        let output = if cut { output } else { output.checked()? };
        let mut parsed = parse(&output.stdout)?;
        parsed.more |= cut;
        let operation = self.operation(dir).await?;
        Ok(GitStatus {
            branch: parsed.branch,
            head: parsed.head,
            upstream: parsed.upstream,
            base: None,
            operation,
            files: parsed.files,
            more: parsed.more,
            // The node adds it for a peer that reads it (ADR 0123 step B).
            origin: None,
        })
    }

    /// How far `HEAD` is from `other` (commits each has that the other does
    /// not); `None` when `other` does not resolve.
    ///
    /// # Errors
    /// As the run's.
    pub async fn distance(
        &self,
        dir: &Path,
        other: &RefName,
    ) -> Result<Option<Distance>, GitError> {
        let output = self
            .command(dir)
            .args(["rev-list", "--left-right", "--count", "--end-of-options"])
            .arg(format!("HEAD...{other}"))
            .output()
            .await?;
        if !output.succeeded() {
            return Ok(None);
        }
        let text = String::from_utf8_lossy(&output.stdout);
        let mut counts = text.split_whitespace().map(str::parse::<u32>);
        match (counts.next(), counts.next()) {
            (Some(Ok(ahead)), Some(Ok(behind))) => Ok(Some(Distance {
                name: other.clone(),
                ahead,
                behind,
            })),
            _ => Err(GitError::Unexpected { what: "rev-list" }),
        }
    }

    /// The operation in progress in the repository at `dir`, from the
    /// files git leaves in its folder while one is.
    ///
    /// # Errors
    /// As the run's.
    pub async fn operation(&self, dir: &Path) -> Result<Option<GitOperation>, GitError> {
        let output = self
            .command(dir)
            .args(["rev-parse", "--absolute-git-dir"])
            .output()
            .await?
            .checked()?;
        let git_dir = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        tokio::task::spawn_blocking(move || {
            let dir = Path::new(&git_dir);
            let has = |name: &str| dir.join(name).exists();
            if has("rebase-merge") || has("rebase-apply") {
                Some(GitOperation::Rebase)
            } else if has("MERGE_HEAD") {
                Some(GitOperation::Merge)
            } else if has("CHERRY_PICK_HEAD") {
                Some(GitOperation::CherryPick)
            } else if has("REVERT_HEAD") {
                Some(GitOperation::Revert)
            } else if has("BISECT_LOG") {
                Some(GitOperation::Bisect)
            } else {
                None
            }
        })
        .await
        .map_err(|_panicked| GitError::Unexpected {
            what: "the repository's folder",
        })
    }
}

/// Read porcelain v2 records.
pub(crate) fn parse(output: &[u8]) -> Result<Parsed, GitError> {
    let unexpected = || GitError::Unexpected { what: "status" };
    let mut parsed = Parsed::default();
    let mut upstream_name: Option<RefName> = None;
    let mut fields = output
        .split(|byte| *byte == 0)
        .filter(|field| !field.is_empty());
    while let Some(record) = fields.next() {
        let text = String::from_utf8_lossy(record);
        let (kind, rest) = text.split_once(' ').ok_or_else(unexpected)?;
        match kind {
            "#" => header(rest, &mut parsed, &mut upstream_name),
            "1" => {
                // XY sub mH mI mW hH hI path
                let mut parts = rest.splitn(8, ' ');
                let xy = parts.next().ok_or_else(unexpected)?;
                let path = parts.nth(6).ok_or_else(unexpected)?;
                push(&mut parsed, xy, path, None)?;
            }
            "2" => {
                // XY sub mH mI mW hH hI Xscore path, then the old path.
                let mut parts = rest.splitn(9, ' ');
                let xy = parts.next().ok_or_else(unexpected)?;
                let path = parts.nth(7).ok_or_else(unexpected)?;
                let old = fields.next().ok_or_else(unexpected)?;
                push(&mut parsed, xy, path, Some(&String::from_utf8_lossy(old)))?;
            }
            "u" => {
                // XY sub m1 m2 m3 mW h1 h2 h3 path
                let path = rest.splitn(10, ' ').nth(9).ok_or_else(unexpected)?;
                entry(
                    &mut parsed,
                    path,
                    None,
                    FileState::Conflicted,
                    FileState::Conflicted,
                );
            }
            "?" => entry(
                &mut parsed,
                rest,
                None,
                FileState::Unmodified,
                FileState::Untracked,
            ),
            // Ignored files are not asked for; skip any a configuration
            // prints anyway.
            "!" => {}
            _ => return Err(unexpected()),
        }
    }
    Ok(parsed)
}

fn header(rest: &str, parsed: &mut Parsed, upstream: &mut Option<RefName>) {
    let (key, value) = rest.split_once(' ').unwrap_or((rest, ""));
    match key {
        "branch.oid" => parsed.head = value.parse().ok(),
        // `(detached)` would parse as a branch name: parentheses are allowed.
        "branch.head" if value != "(detached)" => parsed.branch = value.parse().ok(),
        "branch.upstream" => *upstream = value.parse().ok(),
        "branch.ab" => {
            let mut counts = value
                .split(' ')
                .map(|count| count.trim_start_matches(['+', '-']).parse::<u32>().ok());
            if let (Some(name), Some(Some(ahead)), Some(Some(behind))) =
                (upstream.take(), counts.next(), counts.next())
            {
                parsed.upstream = Some(Distance {
                    name,
                    ahead,
                    behind,
                });
            }
        }
        _ => {}
    }
}

fn push(parsed: &mut Parsed, xy: &str, path: &str, old: Option<&str>) -> Result<(), GitError> {
    let mut letters = xy.chars();
    let (Some(x), Some(y)) = (letters.next(), letters.next()) else {
        return Err(GitError::Unexpected { what: "status" });
    };
    entry(parsed, path, old, state(x), state(y));
    Ok(())
}

fn entry(
    parsed: &mut Parsed,
    path: &str,
    old: Option<&str>,
    index: FileState,
    worktree: FileState,
) {
    if parsed.files.len() >= MAX_ENTRIES {
        parsed.more = true;
        return;
    }
    let Ok(path) = path.parse::<RelPath>() else {
        parsed.unnamed = parsed.unnamed.saturating_add(1);
        return;
    };
    parsed.files.push(StatusEntry {
        path,
        old_path: old.and_then(|old| old.parse().ok()),
        index,
        worktree,
    });
}

/// A porcelain status letter.
const fn state(letter: char) -> FileState {
    match letter {
        'M' => FileState::Modified,
        'A' => FileState::Added,
        'D' => FileState::Deleted,
        'R' => FileState::Renamed,
        'C' => FileState::Copied,
        'T' => FileState::TypeChanged,
        'U' => FileState::Conflicted,
        _ => FileState::Unmodified,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_record_kind_is_read() {
        let output = [
            "# branch.oid 1234567890abcdef1234567890abcdef12345678",
            "# branch.head main",
            "# branch.upstream origin/main",
            "# branch.ab +2 -1",
            "1 .M N... 100644 100644 100644 aaaaaaa aaaaaaa src/lib.rs",
            "1 A. N... 000000 100644 100644 0000000 bbbbbbb new file.txt",
            "2 R. N... 100644 100644 100644 ccccccc ccccccc R100 b.txt",
            "a.txt",
            "u UU N... 100644 100644 100644 100644 d d d conflict.rs",
            "? notes/todo.md",
            "! target/",
            "",
        ]
        .join("\0");
        let parsed = parse(output.as_bytes()).unwrap();
        assert_eq!(parsed.branch.unwrap().as_str(), "main");
        assert_eq!(parsed.head.unwrap().as_str().len(), 40);
        let upstream = parsed.upstream.unwrap();
        assert_eq!(
            (upstream.name.as_str(), upstream.ahead, upstream.behind),
            ("origin/main", 2, 1)
        );
        let files: Vec<_> = parsed
            .files
            .iter()
            .map(|entry| {
                (
                    entry.path.as_str(),
                    entry.old_path.as_ref().map(RelPath::as_str),
                    entry.index,
                    entry.worktree,
                )
            })
            .collect();
        assert_eq!(
            files,
            [
                (
                    "src/lib.rs",
                    None,
                    FileState::Unmodified,
                    FileState::Modified
                ),
                (
                    "new file.txt",
                    None,
                    FileState::Added,
                    FileState::Unmodified
                ),
                (
                    "b.txt",
                    Some("a.txt"),
                    FileState::Renamed,
                    FileState::Unmodified
                ),
                (
                    "conflict.rs",
                    None,
                    FileState::Conflicted,
                    FileState::Conflicted
                ),
                (
                    "notes/todo.md",
                    None,
                    FileState::Unmodified,
                    FileState::Untracked
                ),
            ]
        );
    }

    #[test]
    fn a_detached_head_and_an_unborn_branch_have_no_name_or_commit() {
        let detached = parse(b"# branch.oid abcdef1234567\0# branch.head (detached)\0").unwrap();
        assert_eq!(detached.branch, None);
        assert!(detached.head.is_some());
        let unborn = parse(b"# branch.oid (initial)\0# branch.head main\0").unwrap();
        assert_eq!(unborn.head, None);
        assert_eq!(unborn.branch.unwrap().as_str(), "main");
    }

    #[test]
    fn entries_stop_at_the_bound_and_say_so() {
        let mut output = Vec::new();
        for index in 0..=MAX_ENTRIES {
            output.extend_from_slice(format!("? f{index}\0").as_bytes());
        }
        let parsed = parse(&output).unwrap();
        assert_eq!(parsed.files.len(), MAX_ENTRIES);
        assert!(parsed.more);
    }

    #[test]
    fn a_record_this_parser_does_not_know_is_an_error() {
        assert!(parse(b"9 what\0").is_err());
    }

    #[tokio::test]
    async fn a_real_repository_reports_its_changes_branch_and_operation() {
        let sandbox = crate::testing::Sandbox::new();
        let repo = sandbox.repo("r");
        let git = sandbox.git().await;
        let clean = git.status(&repo).await.unwrap();
        assert_eq!(clean.branch.unwrap().as_str(), "main");
        assert!(clean.files.is_empty());
        assert_eq!(clean.operation, None);

        crate::testing::Sandbox::write(&repo, "README.md", "changed\n");
        crate::testing::Sandbox::write(&repo, "new/file.txt", "x");
        crate::testing::Sandbox::write(&repo, "staged.txt", "s");
        sandbox.run(&repo, &["add", "staged.txt"]);
        crate::testing::Sandbox::write(&repo, ".gitignore", "build/\n");
        crate::testing::Sandbox::write(&repo, "build/out", "o");
        let status = git.status(&repo).await.unwrap();
        let mut files: Vec<_> = status
            .files
            .iter()
            .map(|entry| (entry.path.as_str(), entry.index, entry.worktree))
            .collect();
        files.sort_by_key(|entry| entry.0);
        assert_eq!(
            files,
            [
                (".gitignore", FileState::Unmodified, FileState::Untracked),
                ("README.md", FileState::Unmodified, FileState::Modified),
                ("new/file.txt", FileState::Unmodified, FileState::Untracked),
                ("staged.txt", FileState::Added, FileState::Unmodified),
            ],
            "ignored files are not listed"
        );

        sandbox.run(&repo, &["stash", "-q", "-u"]);
        sandbox.run(&repo, &["checkout", "-q", "-b", "other"]);
        crate::testing::Sandbox::write(&repo, "README.md", "theirs\n");
        sandbox.run(&repo, &["commit", "-q", "-am", "theirs"]);
        sandbox.run(&repo, &["checkout", "-q", "main"]);
        crate::testing::Sandbox::write(&repo, "README.md", "ours\n");
        sandbox.run(&repo, &["commit", "-q", "-am", "ours"]);
        let merge = std::process::Command::new("git")
            .current_dir(&repo)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", sandbox.path().join("gitconfig"))
            .args(["merge", "-q", "other"])
            .output()
            .unwrap();
        assert!(!merge.status.success(), "the merge conflicts");
        let merging = git.status(&repo).await.unwrap();
        assert_eq!(merging.operation, Some(GitOperation::Merge));
        assert_eq!(
            merging.files.first().map(|entry| entry.index),
            Some(FileState::Conflicted)
        );
        let base: RefName = "other".parse().unwrap();
        let distance = git.distance(&repo, &base).await.unwrap().unwrap();
        assert_eq!((distance.ahead, distance.behind), (1, 1));
        assert_eq!(
            git.distance(&repo, &"nope".parse().unwrap()).await.unwrap(),
            None
        );
    }
}
