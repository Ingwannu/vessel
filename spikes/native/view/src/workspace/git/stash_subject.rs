//! A saved entry's subject as git writes it: `On <branch>: <message>` for
//! a named save, `WIP on <branch>: <commit> <subject>` otherwise. The
//! branch is drawn as a branch (one line, cut in its middle) and the
//! message as words, so a long branch never breaks inside a sentence.
//!
//! [Decision Log]
//! - Purpose: the saved-entry list showed the whole subject as one
//!   wrapped sentence, breaking a long branch name mid-word.
//! - Constraints: the node sends git's subject as it is; a ref name can
//!   never contain `:` (`git check-ref-format`), so the first `": "` after
//!   the prefix ends the branch.
//! - Alternatives: show the subject whole (the break stays); ask the node
//!   for the branch (a wire change, not this track's).
//! - Choice: split only git's two fixed prefixes; anything else is the
//!   message, with no branch.
//! - Cost: an entry made by another tool with another format shows whole.

/// A subject's branch, when git's prefix names one, and the rest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StashSubject<'a> {
    /// The branch it was saved on.
    pub branch: Option<&'a str>,
    /// The message (or, for a save without one, the commit and its subject).
    pub message: &'a str,
}

/// Split git's stash subject.
#[must_use]
pub fn stash_subject(subject: &str) -> StashSubject<'_> {
    let named = subject
        .strip_prefix("On ")
        .or_else(|| subject.strip_prefix("WIP on "));
    match named.and_then(|rest| rest.split_once(": ")) {
        Some((branch, message)) if !branch.is_empty() => StashSubject {
            branch: Some(branch),
            message,
        },
        Some(_) | None => StashSubject {
            branch: None,
            message: subject,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_named_save_splits_into_its_branch_and_message() {
        assert_eq!(
            stash_subject("On feature/sidebar-레이아웃: 한글 메모: 초점 순서 확인 중"),
            StashSubject {
                branch: Some("feature/sidebar-레이아웃"),
                message: "한글 메모: 초점 순서 확인 중",
            }
        );
    }

    #[test]
    fn an_unnamed_save_keeps_its_commit_and_subject_as_the_message() {
        assert_eq!(
            stash_subject("WIP on main: 1a2b3c4 First commit"),
            StashSubject {
                branch: Some("main"),
                message: "1a2b3c4 First commit",
            }
        );
    }

    #[test]
    fn another_format_is_all_message() {
        for subject in ["saved by a tool", "On : nothing", "On main without a colon"] {
            assert_eq!(
                stash_subject(subject),
                StashSubject {
                    branch: None,
                    message: subject,
                }
            );
        }
    }
}
