//! Saving an edited file: the guard a save carries, and what its answer
//! means for the editor (ADR 0027's amendment; the page's
//! `data/workspace/save.ts`, ported with its cases).
//!
//! [Decision Log]
//! - Purpose: a save must never overwrite a change it did not see (an
//!   agent's, another editor's), and when it finds one it must show both
//!   states, keeping the edit, so a person chooses (GOAT's banner: reload
//!   from disk or overwrite).
//! - Constraint: `file.write` (protocol 11) is guarded by the SHA-256 of
//!   the content the edit started from, which the node compares on its side
//!   and refuses with `conflict`, writing nothing. A file over 8 MiB carries
//!   no hash; the editor edits at most 1 MiB, so an editable file always
//!   has one.
//! - Alternatives: a check of the disk before an unguarded write (a change
//!   landing between the two is lost); an unguarded write when the version
//!   has no hash (a write a person did not ask for); a retry of a conflict
//!   whose disk matches the edit's start (the node compared exactly that).
//! - Choice: the generic helper permits explicit `Any`; the source-editor
//!   helper confirms an existing whole text file and uses its fresh hash. A `conflict`
//!   is read back once (the disk's version and first page) for the banner,
//!   and is never followed by another write. The two calls are given as
//!   `FnOnce`, so a second write cannot be written.
//! - Cost: one more read after a refused save; none after a save that goes
//!   through.

use std::future::Future;

use vessel_proto::ErrorCode;
use vessel_proto::workspace::{FileContent, FileRead, FileVersion, WriteExpect};

use super::problem::Problem;

/// A save's answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Saved {
    /// Written; the file is now this version.
    Saved(FileVersion),
    /// The file is not what the edit started from; nothing was written.
    Conflict {
        /// The disk's version now, `None` when its state is unverified.
        disk: Option<FileVersion>,
        /// The disk's text, when it is text read whole in one page.
        disk_text: Option<String>,
        /// The text that was refused.
        mine: String,
    },
    /// A guarded write was rejected because the file is confirmed missing.
    Missing {
        /// The original draft remains owned by the editor.
        mine: String,
    },
    /// Not written, for another reason.
    Failed(SaveFailure),
    /// The write may have happened. Retain its original obligation; do not retry.
    Uncertain(Problem),
}

/// Why a save wrote nothing, other than a conflict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveFailure {
    /// The read carried no hash to guard by (the file was over 8 MiB).
    NoGuard,
    /// The node, or the way to it, refused.
    Call(Problem),
}

/// What a save of an edit that started from `base` sends as its guard;
/// `None` when it can carry none (no hash). `base` `None` is the person's
/// Overwrite.
#[must_use]
pub fn save_guard(base: Option<&FileVersion>) -> Option<WriteExpect> {
    match base {
        None => Some(WriteExpect::Any),
        Some(version) => version
            .hash
            .clone()
            .map(|hash| WriteExpect::Content { hash }),
    }
}

/// A refused write, read back for the banner.
#[derive(Clone, Copy)]
enum MissingReadback {
    LegacyConflict,
    Deleted,
}

fn conflict_of(read: Result<FileRead, Problem>, mine: String, missing: MissingReadback) -> Saved {
    match read {
        Ok(read) => {
            // Only a file read whole in its first page is compared line by
            // line: a longer one's first page against the whole edit would
            // count all the rest as added.
            let disk_text = match read.content {
                FileContent::Text {
                    text, next: None, ..
                } => Some(text),
                FileContent::Text { next: Some(_), .. }
                | FileContent::Binary
                | FileContent::Unknown => None,
            };
            Saved::Conflict {
                disk: Some(read.version),
                disk_text,
                mine,
            }
        }
        Err(problem)
            if problem.code() == Some(ErrorCode::NotFound)
                && matches!(missing, MissingReadback::Deleted) =>
        {
            Saved::Missing { mine }
        }
        Err(_problem) => Saved::Conflict {
            disk: None,
            disk_text: None,
            mine,
        },
    }
}

/// Save `text` over the file if it is still `base`, or anyway when `base`
/// is `None` (the person's Overwrite). `write` sends the write with its
/// guard; `read` reads the file's first page as the disk has it now. At
/// most one write is sent.
pub async fn save_text<W, WF, R, RF>(
    write: W,
    read: R,
    text: String,
    base: Option<&FileVersion>,
) -> Saved
where
    W: FnOnce(WriteExpect, String) -> WF,
    WF: Future<Output = Result<FileVersion, Problem>>,
    R: FnOnce() -> RF,
    RF: Future<Output = Result<FileRead, Problem>>,
{
    save_guarded(write, read, text, base, MissingReadback::LegacyConflict).await
}

async fn save_guarded<W, WF, R, RF>(
    write: W,
    read: R,
    text: String,
    base: Option<&FileVersion>,
    missing: MissingReadback,
) -> Saved
where
    W: FnOnce(WriteExpect, String) -> WF,
    WF: Future<Output = Result<FileVersion, Problem>>,
    R: FnOnce() -> RF,
    RF: Future<Output = Result<FileRead, Problem>>,
{
    let Some(expect) = save_guard(base) else {
        return Saved::Failed(SaveFailure::NoGuard);
    };
    let guarded = matches!(expect, WriteExpect::Content { .. });
    match write(expect, text.clone()).await {
        Ok(version) => Saved::Saved(version),
        // Only a guarded write's refusal is a conflict to show; an
        // Overwrite's failure is said as the node said it.
        Err(problem) if guarded && problem.code() == Some(ErrorCode::Conflict) => {
            conflict_of(read().await, text, missing)
        }
        Err(problem) if known_not_written(&problem) => Saved::Failed(SaveFailure::Call(problem)),
        Err(problem) => Saved::Uncertain(problem),
    }
}

/// Whether a file mutation was definitively refused before its effects.
/// Local/protocol admission and node authentication/policy are checked before
/// invoking filesystem work. An invoked connection refusal or filesystem error
/// is not proven no-effect, including a trailing result-version failure.
/// This does not release a different, previously uncertain original attempt.
#[must_use]
pub fn known_not_written(problem: &Problem) -> bool {
    matches!(problem, Problem::Older | Problem::Unsent)
        || matches!(
            problem.code(),
            Some(ErrorCode::Forbidden | ErrorCode::Unauthorized)
        )
}

/// Editor overwrite is restricted to a freshly confirmed existing text file.
/// The generic `save_text` remains available to callers that explicitly use
/// its unguarded-write contract; the source editor never sends `Any`.
///
/// The returned outcome preserves the draft on missing, uneditable, unknown,
/// or changed files. At most one write is attempted and none is replayed.
pub async fn save_existing_text<W, WF, R, RF>(
    write: W,
    read: R,
    text: String,
    base: Option<&FileVersion>,
) -> Saved
where
    W: FnOnce(WriteExpect, String) -> WF,
    WF: Future<Output = Result<FileVersion, Problem>>,
    R: FnOnce() -> RF,
    RF: Future<Output = Result<FileRead, Problem>>,
{
    if base.is_some() {
        return save_guarded(write, read, text, base, MissingReadback::Deleted).await;
    }
    let current = match read().await {
        Ok(current) => current,
        Err(problem) if problem.code() == Some(ErrorCode::NotFound) => {
            return Saved::Missing { mine: text };
        }
        Err(problem) => return Saved::Failed(SaveFailure::Call(problem)),
    };
    if current.version.size > super::editor::MAX_EDITABLE_BYTES
        || !matches!(current.content, FileContent::Text { next: None, .. })
    {
        return Saved::Conflict {
            disk: None,
            disk_text: None,
            mine: text,
        };
    }
    let Some(expect) = save_guard(Some(&current.version)) else {
        return Saved::Failed(SaveFailure::NoGuard);
    };
    match write(expect, text.clone()).await {
        Ok(version) => Saved::Saved(version),
        // The single read was used before overwrite. A new race must be
        // resolved by a new owner acquisition, never a blind force retry.
        Err(problem) if problem.code() == Some(ErrorCode::Conflict) => Saved::Conflict {
            disk: None,
            disk_text: None,
            mine: text,
        },
        Err(problem) if known_not_written(&problem) => Saved::Failed(SaveFailure::Call(problem)),
        Err(problem) => Saved::Uncertain(problem),
    }
}

#[cfg(test)]
mod tests {
    //! The page's `save.test.ts`, case for case.

    use std::cell::RefCell;
    use std::future::{Ready, ready};
    use std::pin::pin;
    use std::task::{Context, Poll, Waker};

    use vessel_proto::workspace::ContentHash;

    use super::*;

    /// Runs a future whose every await is ready at once (the fakes below).
    fn now<F: Future>(future: F) -> F::Output {
        let mut future = pin!(future);
        let mut context = Context::from_waker(Waker::noop());
        loop {
            if let Poll::Ready(output) = future.as_mut().poll(&mut context) {
                return output;
            }
        }
    }

    fn hash(digit: char) -> ContentHash {
        digit.to_string().repeat(64).parse().unwrap()
    }

    fn version(size: u64, modified: u64, digit: char) -> FileVersion {
        FileVersion {
            size,
            modified,
            hash: Some(hash(digit)),
        }
    }

    fn read_version() -> FileVersion {
        version(25, 1_762_934_400_123, 'a')
    }
    fn written() -> FileVersion {
        version(31, 1_762_934_460_456, 'c')
    }
    fn changed() -> FileVersion {
        version(40, 1_762_934_450_789, 'b')
    }

    fn refused(code: ErrorCode, message: &str) -> Problem {
        Problem::Node {
            code,
            message: message.to_owned(),
        }
    }

    /// A node holding one file: a guarded write lands only when the content
    /// is the one the guard names, as `file.write` does.
    struct Node {
        version: RefCell<FileVersion>,
        text: RefCell<String>,
        writes: RefCell<Vec<WriteExpect>>,
    }

    impl Node {
        fn new(version: FileVersion, text: &str) -> Self {
            Self {
                version: RefCell::new(version),
                text: RefCell::new(text.to_owned()),
                writes: RefCell::new(Vec::new()),
            }
        }

        fn write(&self, expect: &WriteExpect, text: String) -> Ready<Result<FileVersion, Problem>> {
            self.writes.borrow_mut().push(expect.clone());
            if let WriteExpect::Content { hash, .. } = expect
                && self.version.borrow().hash.as_ref() != Some(hash)
            {
                return ready(Err(refused(ErrorCode::Conflict, "the file changed")));
            }
            *self.version.borrow_mut() = written();
            *self.text.borrow_mut() = text;
            ready(Ok(written()))
        }

        fn read(&self) -> Ready<Result<FileRead, Problem>> {
            ready(Ok(FileRead {
                version: self.version.borrow().clone(),
                content: FileContent::Text {
                    text: self.text.borrow().clone(),
                    next: None,
                },
            }))
        }

        fn save(&self, text: &str, base: Option<&FileVersion>) -> Saved {
            now(save_text(
                |expect, text| self.write(&expect, text),
                || self.read(),
                text.to_owned(),
                base,
            ))
        }
    }

    #[test]
    fn a_save_s_guard_is_the_content_s_hash_the_edit_started_from() {
        assert_eq!(
            save_guard(Some(&read_version())),
            Some(WriteExpect::Content { hash: hash('a') })
        );
    }

    #[test]
    fn a_save_s_guard_is_anything_only_for_the_person_s_overwrite() {
        assert_eq!(save_guard(None), Some(WriteExpect::Any));
    }

    #[test]
    fn a_save_s_guard_is_none_when_the_read_carried_no_hash() {
        let unhashed = FileVersion {
            hash: None,
            ..read_version()
        };
        assert_eq!(save_guard(Some(&unhashed)), None);
    }

    #[test]
    fn goes_through_over_an_untouched_file_guarded_by_its_hash() {
        let node = Node::new(read_version(), "theirs");
        assert_eq!(
            node.save("mine", Some(&read_version())),
            Saved::Saved(written())
        );
        assert_eq!(
            *node.writes.borrow(),
            [WriteExpect::Content { hash: hash('a') }]
        );
    }

    #[test]
    fn over_a_file_changed_meanwhile_is_a_conflict_showing_both_states_and_writes_nothing() {
        let node = Node::new(changed(), "an agent's change");
        assert_eq!(
            node.save("mine", Some(&read_version())),
            Saved::Conflict {
                disk: Some(changed()),
                disk_text: Some("an agent's change".to_owned()),
                mine: "mine".to_owned(),
            }
        );
        // One guarded write, refused: never a second one with `Any`.
        assert_eq!(
            *node.writes.borrow(),
            [WriteExpect::Content { hash: hash('a') }]
        );
        assert_eq!(*node.text.borrow(), "an agent's change");
    }

    #[test]
    fn over_a_file_gone_meanwhile_is_a_conflict_with_nothing_on_disk() {
        let saved = now(save_text(
            |_expect, _text| ready(Err(refused(ErrorCode::Conflict, "nothing is there"))),
            || ready(Err(refused(ErrorCode::NotFound, "not found"))),
            "mine".to_owned(),
            Some(&read_version()),
        ));
        assert_eq!(
            saved,
            Saved::Conflict {
                disk: None,
                disk_text: None,
                mine: "mine".to_owned(),
            }
        );
    }

    #[test]
    fn overwrites_only_when_the_person_asked() {
        let node = Node::new(changed(), "an agent's change");
        assert!(matches!(node.save("mine", None), Saved::Saved(_)));
        assert_eq!(*node.writes.borrow(), [WriteExpect::Any]);
    }

    #[test]
    fn writes_nothing_when_the_read_carried_no_hash_to_guard_by() {
        let node = Node::new(read_version(), "theirs");
        let unhashed = FileVersion {
            hash: None,
            ..read_version()
        };
        assert_eq!(
            node.save("mine", Some(&unhashed)),
            Saved::Failed(SaveFailure::NoGuard)
        );
        assert!(node.writes.borrow().is_empty());
    }

    #[test]
    fn says_another_refusal_as_the_node_said_it() {
        let saved = now(save_text(
            |_expect, _text| ready(Err(refused(ErrorCode::Forbidden, "read only"))),
            || -> Ready<Result<FileRead, Problem>> { panic!("not read") },
            "mine".to_owned(),
            Some(&read_version()),
        ));
        assert_eq!(
            saved,
            Saved::Failed(SaveFailure::Call(refused(
                ErrorCode::Forbidden,
                "read only"
            )))
        );
    }

    #[test]
    fn a_conflict_s_sides_are_compared_line_by_line_only_when_the_disk_s_file_was_read_whole() {
        let saved = now(save_text(
            |_expect, _text| ready(Err(refused(ErrorCode::Conflict, "changed"))),
            || {
                ready(Ok(FileRead {
                    version: changed(),
                    content: FileContent::Text {
                        text: "first page".to_owned(),
                        next: Some(262_144),
                    },
                }))
            },
            "mine".to_owned(),
            Some(&read_version()),
        ));
        assert!(matches!(
            saved,
            Saved::Conflict { disk: Some(disk), disk_text: None, .. } if disk == changed()
        ));
    }

    #[test]
    fn source_guarded_conflict_can_distinguish_confirmed_deletion() {
        assert_eq!(
            now(save_existing_text(
                |_, _| ready(Err(refused(ErrorCode::Conflict, "changed"))),
                || ready(Err(refused(ErrorCode::NotFound, "missing"))),
                "mine".into(),
                Some(&read_version()),
            )),
            Saved::Missing {
                mine: "mine".into()
            }
        );
    }

    #[test]
    fn a_post_publication_node_failure_keeps_the_original_effect_uncertain() {
        assert!(matches!(
            now(save_existing_text(
                |_, _| ready(Err(refused(ErrorCode::Internal, "result version failed"))),
                || -> Ready<Result<FileRead, Problem>> { panic!("not read") },
                "mine".into(),
                Some(&read_version()),
            )),
            Saved::Uncertain(_)
        ));
    }

    #[test]
    fn source_overwrite_uses_the_fresh_hash_and_never_any() {
        let node = Node::new(changed(), "external edit");
        let saved = now(save_existing_text(
            |expect, text| node.write(&expect, text),
            || node.read(),
            "mine".into(),
            None,
        ));
        assert_eq!(saved, Saved::Saved(written()));
        assert_eq!(
            *node.writes.borrow(),
            [WriteExpect::Content { hash: hash('b') }]
        );
    }

    #[test]
    fn source_overwrite_never_creates_a_missing_file() {
        let saved = now(save_existing_text(
            |_, _| -> Ready<Result<FileVersion, Problem>> { panic!("must not write") },
            || ready(Err(refused(ErrorCode::NotFound, "missing"))),
            "mine".into(),
            None,
        ));
        assert_eq!(
            saved,
            Saved::Missing {
                mine: "mine".into()
            }
        );
    }

    #[test]
    fn failed_preoverwrite_read_is_proven_not_sent() {
        let saved = now(save_existing_text(
            |_, _| -> Ready<Result<FileVersion, Problem>> { panic!("must not write") },
            || ready(Err(Problem::Unsent)),
            "mine".into(),
            None,
        ));
        assert_eq!(saved, Saved::Failed(SaveFailure::Call(Problem::Unsent)));
    }

    #[test]
    fn source_overwrite_refuses_binary_partial_and_oversize_snapshots() {
        for (version, content) in [
            (changed(), FileContent::Binary),
            (
                changed(),
                FileContent::Text {
                    text: "part".into(),
                    next: Some(4),
                },
            ),
            (
                version(
                    super::super::editor::MAX_EDITABLE_BYTES.saturating_add(1),
                    1,
                    'b',
                ),
                FileContent::Text {
                    text: "large".into(),
                    next: None,
                },
            ),
        ] {
            let saved = now(save_existing_text(
                |_, _| -> Ready<Result<FileVersion, Problem>> { panic!("must not write") },
                || ready(Ok(FileRead { version, content })),
                "mine".into(),
                None,
            ));
            assert_eq!(
                saved,
                Saved::Conflict {
                    disk: None,
                    disk_text: None,
                    mine: "mine".into()
                }
            );
        }
    }

    #[test]
    fn source_overwrite_race_is_not_retried_or_forced() {
        let node = Node::new(changed(), "external edit");
        let saved = now(save_existing_text(
            |expect, _| {
                node.writes.borrow_mut().push(expect);
                ready(Err(refused(ErrorCode::Conflict, "changed again")))
            },
            || node.read(),
            "mine".into(),
            None,
        ));
        assert_eq!(
            saved,
            Saved::Conflict {
                disk: None,
                disk_text: None,
                mine: "mine".into()
            }
        );
        assert_eq!(
            *node.writes.borrow(),
            [WriteExpect::Content { hash: hash('b') }]
        );
        assert_eq!(*node.text.borrow(), "external edit");
    }

    #[test]
    fn failed_conflict_readback_does_not_claim_deletion() {
        let saved = now(save_existing_text(
            |_, _| ready(Err(refused(ErrorCode::Conflict, "changed"))),
            || ready(Err(refused(ErrorCode::Forbidden, "unreadable"))),
            "mine".into(),
            Some(&read_version()),
        ));
        assert_eq!(
            saved,
            Saved::Conflict {
                disk: None,
                disk_text: None,
                mine: "mine".into()
            }
        );
    }
}

#[cfg(test)]
mod completion_tests {
    use super::*;
    use std::future::ready;
    // No executor is needed: both fake boundaries are immediately ready.
    fn now<F: Future>(future: F) -> F::Output {
        let mut future = std::pin::pin!(future);
        let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
        match future.as_mut().poll(&mut cx) {
            std::task::Poll::Ready(value) => value,
            std::task::Poll::Pending => panic!("ready fake"),
        }
    }
    #[test]
    fn refused_before_dispatch_is_distinct_from_an_unknown_completion() {
        let settle = |problem| async {
            save_text(
                |_, _| ready(Err(problem)),
                || ready(Err(Problem::Unsent)),
                "draft".into(),
                None,
            )
            .await
        };
        assert!(matches!(
            now(settle(Problem::Unsent)),
            Saved::Failed(SaveFailure::Call(Problem::Unsent))
        ));
        assert!(matches!(
            now(settle(Problem::Older)),
            Saved::Failed(SaveFailure::Call(Problem::Older))
        ));
        assert!(matches!(
            now(settle(Problem::Gone("transport lost".into()))),
            Saved::Uncertain(Problem::Gone(_))
        ));
    }
}
