//! What git knows about a folder's files: which exist and are not ignored
//! (the name index), which of some are ignored (folder listings), and
//! which lines match a search (`git grep`).

use std::collections::HashSet;
use std::path::Path;

use vessel_proto::workspace::{GrepCase, GrepMatch, GrepMode, GrepQuery, GrepResult, RelPath};

use super::patch::cut_text;
use super::{Git, GitError, Limits};

/// The most paths a name index holds.
pub(crate) const MAX_INDEXED: usize = 500_000;
/// The most matches a search returns.
pub(crate) const MAX_MATCHES: u32 = 500;
/// A search's matches when not asked.
pub(crate) const DEFAULT_MATCHES: u32 = 100;
/// The longest line text a match carries, in characters.
const MAX_MATCH_CHARS: usize = 512;
/// The most of `git grep`'s output read.
const MAX_GREP_BYTES: usize = 4 * 1024 * 1024;
/// The most bytes of paths given to one `check-ignore` run: Windows limits
/// a command line to 32,767 characters.
const MAX_ARGUMENT_BYTES: usize = 24 * 1024;

impl Git {
    /// Every tracked file and every untracked one that is not ignored, at
    /// most [`MAX_INDEXED`]; `true` with them when that was all of them.
    ///
    /// # Errors
    /// As the run's.
    pub async fn files(&self, dir: &Path) -> Result<(Vec<RelPath>, bool), GitError> {
        let output = self
            .command(dir)
            .args([
                "ls-files",
                "--cached",
                "--others",
                "--exclude-standard",
                "-z",
            ])
            .limits(Limits::LOCAL.with_stdout(64 * 1024 * 1024))
            .output()
            .await?;
        let cut = output.stdout_cut;
        let output = if cut { output } else { output.checked()? };
        let mut seen = HashSet::new();
        let mut paths = Vec::new();
        for field in output.stdout.split(|byte| *byte == 0) {
            if field.is_empty() {
                continue;
            }
            // A conflicted file is listed once per stage.
            let Ok(path) = String::from_utf8_lossy(field).parse::<RelPath>() else {
                continue;
            };
            if !seen.insert(path.clone()) {
                continue;
            }
            if paths.len() >= MAX_INDEXED {
                return Ok((paths, false));
            }
            paths.push(path);
        }
        Ok((paths, !cut))
    }

    /// Which of `paths` git ignores.
    ///
    /// # Errors
    /// As the runs'.
    pub async fn ignored(
        &self,
        dir: &Path,
        paths: &[RelPath],
    ) -> Result<HashSet<RelPath>, GitError> {
        let mut ignored = HashSet::new();
        let mut batch: Vec<&RelPath> = Vec::new();
        let mut bytes = 0_usize;
        for path in paths.iter().filter(|path| !path.as_str().starts_with(':')) {
            if bytes.saturating_add(path.as_str().len()) > MAX_ARGUMENT_BYTES && !batch.is_empty() {
                self.check_ignore(dir, &batch, &mut ignored).await?;
                batch.clear();
                bytes = 0;
            }
            bytes = bytes.saturating_add(path.as_str().len()).saturating_add(1);
            batch.push(path);
        }
        if !batch.is_empty() {
            self.check_ignore(dir, &batch, &mut ignored).await?;
        }
        Ok(ignored)
    }

    async fn check_ignore(
        &self,
        dir: &Path,
        batch: &[&RelPath],
        ignored: &mut HashSet<RelPath>,
    ) -> Result<(), GitError> {
        let output = self
            .command(dir)
            .args(["check-ignore", "--"])
            // `check-ignore` refuses literal pathspecs; a name that could
            // read as pathspec magic (`:(…)`) is left out of `batch` and
            // shown as not ignored, which only changes a label.
            .env("GIT_LITERAL_PATHSPECS", "0")
            .args(batch.iter().map(|path| path.as_str()))
            .output()
            .await?;
        // 1: none is ignored; 0: some are; anything else is a failure.
        if !matches!(output.code, Some(0 | 1)) {
            return Err(GitError::Failed {
                code: output.code,
                message: super::tail(&output.stderr_text()),
            });
        }
        // One path a line (`-z` needs standard input, which no run has); a
        // name git would quote (one holding `"`) does not parse and is shown
        // as not ignored.
        ignored.extend(
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .filter_map(|line| line.parse().ok()),
        );
        Ok(())
    }

    /// Lines matching `query` in the files of `dir` (`git grep`; `repository`
    /// says whether `dir` is one, else `--no-index`), under `under` when
    /// given, binary files left out.
    ///
    /// # Errors
    /// As the run's.
    #[expect(
        clippy::too_many_arguments,
        reason = "one call's parameters, each a field of `file.grep`'s request"
    )]
    pub async fn grep(
        &self,
        dir: &Path,
        repository: bool,
        query: &GrepQuery,
        mode: GrepMode,
        case: GrepCase,
        under: Option<&RelPath>,
        limit: Option<u32>,
    ) -> Result<GrepResult, GitError> {
        let limit = limit.unwrap_or(DEFAULT_MATCHES).clamp(1, MAX_MATCHES);
        let mut command = self.command(dir).args([
            "grep",
            "-z",
            "-n",
            "--column",
            "-I",
            "--no-color",
            "--full-name",
        ]);
        command = if repository {
            command.arg("--untracked")
        } else {
            command.arg("--no-index")
        };
        command = command.arg(match mode {
            GrepMode::Literal => "--fixed-strings",
            GrepMode::Regex => "--extended-regexp",
        });
        let sensitive = match case {
            GrepCase::Sensitive => true,
            GrepCase::Insensitive => false,
            GrepCase::Smart => query.as_str().chars().any(char::is_uppercase),
        };
        if !sensitive {
            command = command.arg("--ignore-case");
        }
        command = command.arg("-e").arg(query.as_str()).arg("--");
        if let Some(under) = under.filter(|under| !under.is_root()) {
            command = command.arg(under.as_str());
        }
        let output = command
            .limits(Limits::LOCAL.with_stdout(MAX_GREP_BYTES))
            .output()
            .await?;
        // 1: nothing matched.
        if !output.stdout_cut && !matches!(output.code, Some(0 | 1)) {
            return Err(GitError::Failed {
                code: output.code,
                message: super::tail(&output.stderr_text()),
            });
        }
        let mut matches = Vec::new();
        let mut truncated = output.stdout_cut;
        let wanted = usize::try_from(limit).unwrap_or(usize::MAX);
        for line in output.stdout.split(|byte| *byte == b'\n') {
            let Some(found) = grep_line(line) else {
                continue;
            };
            if matches.len() >= wanted {
                truncated = true;
                break;
            }
            matches.push(found);
        }
        Ok(GrepResult {
            matches,
            more: truncated,
        })
    }
}

/// `path\0line\0column\0text`, the column turned from git's bytes into
/// characters.
fn grep_line(line: &[u8]) -> Option<GrepMatch> {
    let mut fields = line.splitn(4, |byte| *byte == 0);
    let path = String::from_utf8_lossy(fields.next()?).parse().ok()?;
    let number = String::from_utf8_lossy(fields.next()?).parse().ok()?;
    let column: usize = String::from_utf8_lossy(fields.next()?).parse().ok()?;
    let text = fields.next()?;
    let before = text.get(..column.saturating_sub(1)).unwrap_or(text);
    let column = u32::try_from(
        String::from_utf8_lossy(before)
            .chars()
            .count()
            .saturating_add(1),
    )
    .unwrap_or(u32::MAX);
    let text = String::from_utf8_lossy(text.strip_suffix(b"\r").unwrap_or(text));
    let (text, cut) = if text.chars().count() > MAX_MATCH_CHARS {
        let end = text
            .char_indices()
            .nth(MAX_MATCH_CHARS)
            .map_or(text.len(), |(index, _)| index);
        cut_text(&text, end)
    } else {
        (text.into_owned(), false)
    };
    Some(GrepMatch {
        path,
        line: number,
        column,
        text,
        cut,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::Sandbox;

    #[tokio::test]
    async fn files_ignores_and_searches() {
        let sandbox = Sandbox::new();
        let repo = sandbox.repo("r");
        let git = sandbox.git().await;
        Sandbox::write(&repo, ".gitignore", "target/\n");
        Sandbox::write(
            &repo,
            "src/main.rs",
            "fn main() {\n    // 한글 TODO here\n}\n",
        );
        Sandbox::write(&repo, "target/out.txt", "TODO hidden\n");
        Sandbox::write(&repo, "image.bin", "TODO\0binary");
        let (files, complete) = git.files(&repo).await.unwrap();
        assert!(complete);
        let mut names: Vec<_> = files.iter().map(RelPath::as_str).collect();
        names.sort_unstable();
        assert_eq!(
            names,
            [".gitignore", "README.md", "image.bin", "src/main.rs"]
        );

        let ignored = git
            .ignored(&repo, &["target".parse().unwrap(), "src".parse().unwrap()])
            .await
            .unwrap();
        assert!(ignored.contains(&"target".parse().unwrap()));
        assert!(!ignored.contains(&"src".parse().unwrap()));

        let query: GrepQuery = "TODO".parse().unwrap();
        let found = git
            .grep(
                &repo,
                true,
                &query,
                GrepMode::Literal,
                GrepCase::Smart,
                None,
                None,
            )
            .await
            .unwrap();
        assert_eq!(found.matches.len(), 1, "{found:?}");
        let hit = &found.matches[0];
        assert_eq!((hit.path.as_str(), hit.line), ("src/main.rs", 2));
        // `    // 한글 ` is ten characters (fourteen bytes) before the match.
        assert_eq!(hit.column, 11);
        let lower: GrepQuery = "todo".parse().unwrap();
        let smart = git
            .grep(
                &repo,
                true,
                &lower,
                GrepMode::Literal,
                GrepCase::Smart,
                None,
                None,
            )
            .await
            .unwrap();
        assert_eq!(smart.matches.len(), 1, "lower case searches without case");
        let none = git
            .grep(
                &repo,
                true,
                &query,
                GrepMode::Literal,
                GrepCase::Sensitive,
                Some(&"nope".parse().unwrap()),
                None,
            )
            .await
            .unwrap();
        assert!(none.matches.is_empty());
        // A query that looks like an option is a pattern.
        let dashed: GrepQuery = "--version".parse().unwrap();
        let dashed = git
            .grep(
                &repo,
                true,
                &dashed,
                GrepMode::Literal,
                GrepCase::Sensitive,
                None,
                None,
            )
            .await
            .unwrap();
        assert!(dashed.matches.is_empty());

        let folder = sandbox.path().join("plain");
        Sandbox::write(&folder, "notes.txt", "a TODO\n");
        let plain = git
            .grep(
                &folder,
                false,
                &query,
                GrepMode::Literal,
                GrepCase::Smart,
                None,
                Some(5),
            )
            .await
            .unwrap();
        assert_eq!(plain.matches.len(), 1);
    }
}
