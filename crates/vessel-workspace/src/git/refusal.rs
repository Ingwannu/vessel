//! What git refused, read from its messages into a [`GitRefusal`] once,
//! here, so no client reads git's text (AGENTS.md section 6).
//!
//! Every run has `LC_ALL=C`, so the messages are git's English ones. A
//! message this module does not know ends as `remote_failed` with git's
//! last lines, credentials removed: shown to a person, never parsed by a
//! client.

use vessel_proto::workspace::{GitRefusal, RelPath, RepositoryConfigReason};

use super::{GitError, GitOutput, OwnConfigRefusal, redact};

/// The most paths a refusal carries.
pub(crate) const MAX_PATHS: usize = 100;

/// A refusal for a failed run, or `None` when its messages say nothing
/// this module knows.
pub(crate) fn known(output: &GitOutput) -> Option<GitRefusal> {
    let stderr = output.stderr_text();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let text = format!("{stdout}\n{stderr}");
    let has = |needle: &str| text.contains(needle);
    if has("terminal prompts disabled")
        || has("could not read Username")
        || has("could not read Password")
        || has("Authentication failed")
        || has("Permission denied (publickey")
        || has("Invalid username or password")
    {
        return Some(GitRefusal::NeedsCredential);
    }
    if has("(non-fast-forward)") || has("(fetch first)") || has("Updates were rejected") {
        return Some(GitRefusal::NonFastForward);
    }
    if has("Not possible to fast-forward") || has("Diverging branches") {
        return Some(GitRefusal::Diverged);
    }
    if has("would be overwritten by") {
        return Some(GitRefusal::WouldOverwrite {
            paths: indented_paths(&text),
        });
    }
    if has("CONFLICT (") {
        return Some(GitRefusal::Conflicts {
            paths: conflict_paths(&text),
        });
    }
    if has("Please tell me who you are")
        || has("Author identity unknown")
        || has("Committer identity unknown")
        || has("no email was given")
        || has("no name was given")
    {
        return Some(GitRefusal::NoIdentity);
    }
    if has("has no upstream branch") || has("There is no tracking information") {
        return Some(GitRefusal::NoUpstream);
    }
    if let Some(path) = quoted_after(&text, "is already checked out at '")
        .or_else(|| quoted_after(&text, "is already used by worktree at '"))
    {
        return Some(GitRefusal::CheckedOutElsewhere { path: path.into() });
    }
    if has("already exists") {
        return Some(GitRefusal::BranchExists);
    }
    if has("invalid reference")
        || has("not a valid object name")
        || has("did not match any")
        || has("not a valid branch name")
        || has("No such ref")
        || has("no branch named")
    {
        return Some(GitRefusal::NoSuchRef);
    }
    None
}

/// A refusal for a failed run: what [`known`] reads, otherwise its last
/// lines as `remote_failed`.
pub(crate) fn of(output: &GitOutput) -> GitRefusal {
    known(output).unwrap_or_else(|| GitRefusal::RemoteFailed {
        message: redact::tail(&output.stderr_text()),
    })
}

/// A run that did not finish, or was refused before it started, as a
/// refusal where one fits.
pub(crate) fn of_error(error: GitError) -> Result<GitRefusal, GitError> {
    match error {
        GitError::TimedOut { .. } => Ok(GitRefusal::TimedOut),
        GitError::OwnConfig { key, why } => Ok(GitRefusal::RepositoryConfig {
            key: key.parse().ok(),
            reason: reason(why),
        }),
        GitError::OwnConfigTooLarge => Ok(GitRefusal::RepositoryConfig {
            key: None,
            reason: RepositoryConfigReason::TooLarge,
        }),
        GitError::Spawn(_)
        | GitError::Wait(_)
        | GitError::Cancelled
        | GitError::Closed
        | GitError::Busy
        | GitError::OutputLimit
        | GitError::Failed { .. }
        | GitError::Unexpected { .. } => Err(error),
    }
}

/// The wire's reason for a refusal of the repository's own configuration.
const fn reason(why: OwnConfigRefusal) -> RepositoryConfigReason {
    match why {
        OwnConfigRefusal::Program => RepositoryConfigReason::Program,
        OwnConfigRefusal::Layout => RepositoryConfigReason::Layout,
        OwnConfigRefusal::Credential => RepositoryConfigReason::Credential,
        OwnConfigRefusal::Unnamable => RepositoryConfigReason::Unnamable,
        OwnConfigRefusal::TooMany => RepositoryConfigReason::TooMany,
    }
}

/// The tab-indented paths git lists under "would be overwritten".
fn indented_paths(text: &str) -> Vec<RelPath> {
    text.lines()
        .filter_map(|line| line.strip_prefix('\t'))
        .filter_map(|path| path.trim().parse().ok())
        .take(MAX_PATHS)
        .collect()
}

/// The paths of `CONFLICT (...): Merge conflict in <path>` lines.
fn conflict_paths(text: &str) -> Vec<RelPath> {
    text.lines()
        .filter_map(|line| line.split_once("Merge conflict in ").map(|(_, path)| path))
        .filter_map(|path| path.trim().parse().ok())
        .take(MAX_PATHS)
        .collect()
}

/// The text between `prefix` and the next `'`.
fn quoted_after(text: &str, prefix: &str) -> Option<String> {
    let (_, rest) = text.split_once(prefix)?;
    rest.split_once('\'').map(|(quoted, _)| quoted.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn failed(stderr: &str) -> GitOutput {
        GitOutput {
            code: Some(1),
            stdout: Vec::new(),
            stdout_cut: false,
            stderr: stderr.as_bytes().to_vec(),
        }
    }

    #[test]
    fn git_s_messages_become_typed_refusals() {
        for (stderr, refusal) in [
            (
                "fatal: could not read Username for 'https://github.com': terminal prompts disabled",
                GitRefusal::NeedsCredential,
            ),
            (
                " ! [rejected]        main -> main (fetch first)\nerror: failed to push some refs",
                GitRefusal::NonFastForward,
            ),
            (
                "fatal: Not possible to fast-forward, aborting.",
                GitRefusal::Diverged,
            ),
            (
                "*** Please tell me who you are.\n\nRun\n\n  git config --global user.email",
                GitRefusal::NoIdentity,
            ),
            (
                "fatal: The current branch topic has no upstream branch.",
                GitRefusal::NoUpstream,
            ),
            (
                "fatal: 'topic' is already used by worktree at '/tmp/w'",
                GitRefusal::CheckedOutElsewhere {
                    path: "/tmp/w".into(),
                },
            ),
            (
                "fatal: a branch named 'topic' already exists",
                GitRefusal::BranchExists,
            ),
            ("fatal: invalid reference: nope", GitRefusal::NoSuchRef),
        ] {
            assert_eq!(of(&failed(stderr)), refusal, "{stderr}");
        }
    }

    #[test]
    fn listed_paths_are_kept_and_unknown_text_is_redacted() {
        let overwrite = of(&failed(
            "error: Your local changes to the following files would be overwritten by checkout:\n\tREADME.md\n\tsrc/a.rs\nPlease commit",
        ));
        assert_eq!(
            overwrite,
            GitRefusal::WouldOverwrite {
                paths: vec!["README.md".parse().unwrap(), "src/a.rs".parse().unwrap()]
            }
        );
        let conflicts = of(&failed(
            "CONFLICT (content): Merge conflict in README.md\nerror: could not apply",
        ));
        assert_eq!(
            conflicts,
            GitRefusal::Conflicts {
                paths: vec!["README.md".parse().unwrap()]
            }
        );
        let other = of(&failed(
            "fatal: unable to access 'https://me:secret@host/r/': 500",
        ));
        let GitRefusal::RemoteFailed { message } = other else {
            panic!("{other:?}");
        };
        assert!(!message.contains("secret"), "{message}");
    }
}
