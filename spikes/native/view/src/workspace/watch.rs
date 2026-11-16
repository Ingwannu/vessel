//! The file shown in the Files tab follows the disk while it is on screen
//! (Paseo's `file-panels/file/handle.ts`, `setObserving` and `receive`):
//! one finite `file.observe` at a time (protocol 27, ADR 0105), anchored
//! on its last answer, decides when the panel reads the file again. The
//! asking and the reading are the app's; when to do either is decided
//! here.
//!
//! [Decision Log]
//! - Purpose: a file changed by an agent, git or another editor shows its
//!   new content (a clean editor) or the conflict (a modified one) without
//!   waiting for the next save, as Paseo's live file subscription does.
//! - Constraint: the node's observation is one finite call, not a stream:
//!   it answers when the file differs from `after`, or after its heartbeat
//!   with the same state, or at once (a delegated call, no wait share
//!   left). A loop must not ask back to back.
//! - Alternatives: a timer that reads the file (ADR 0083 rejected it);
//!   a watch per file ever opened (Paseo observes only a tab on screen).
//! - Choice: Paseo's rule that a version this panel already shows is no
//!   change (its `seen` set: a save of this editor comes back as the
//!   editor's own version); anything else that differs from the anchor,
//!   or from the content shown on the first answer, is a change. Nothing
//!   is asked before a read of the file has landed, so the first answer is
//!   always judged against what the panel shows (an answer before it
//!   could anchor a newer disk the panel never read). The rest of
//!   [`pace::QUIET`] is waited out after every answer. The node's
//!   `unavailable` (its observers all busy) is asked again after the
//!   pause, the anchor kept; any other failure stops the watch until the
//!   file is opened again, and the panel then behaves as before.
//! - Cost: one node observer and one wait share per shown file; one extra
//!   read after this editor's own save is avoided by the version check.
//!   The link has no cancel: an observation the panel no longer wants (a
//!   file left, the panel hidden) is ignored when it answers, and holds
//!   both until the node's 30 s heartbeat.

use std::time::{Duration, Instant};

use vessel_proto::workspace::panel_files::{ObserveFile, ObservedFile};
use vessel_proto::workspace::{ContentHash, RelPath, WorkspaceRoot};

use vessel_proto::ErrorCode;

use super::pace;
use super::problem::Problem;

/// Where the watch is.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
enum Phase {
    /// Nothing out: the panel may ask.
    #[default]
    Idle,
    /// This observation is out since `at`.
    Asked { ask: ObserveFile, at: Instant },
    /// Answered; the pause numbered `rest` runs before the next ask.
    Resting { rest: u64 },
    /// An observation failed: no more until the file is opened again.
    Stopped,
}

/// What an observation's answer asks of the panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Answered {
    /// Not the observation out (an earlier file's, or a stopped watch's):
    /// nothing to do.
    Stale,
    /// Nothing new: ask again once the pause has passed.
    Quiet {
        /// The pause's number, for [`Watch::rested`].
        rest: u64,
        /// How long it lasts.
        wait: Duration,
    },
    /// The file changed (or went missing): read it again, then ask again
    /// once the pause has passed.
    Changed {
        /// The pause's number, for [`Watch::rested`].
        rest: u64,
        /// How long it lasts.
        wait: Duration,
    },
    /// The observation failed: the watch stopped.
    Stopped,
}

/// The shown file's watch.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Watch {
    /// The last state the node reported, sent back as `after`.
    anchor: Option<ObservedFile>,
    /// The content hash of the last read the panel showed read-only.
    seen: Option<ContentHash>,
    /// Whether a read of the file has landed: no observation before it.
    read: bool,
    phase: Phase,
    /// The number of the latest pause.
    rests: u64,
}

impl Watch {
    /// A read of the file landed: the read-only content the panel shows
    /// now (`None`: not known, as for a file over 8 MiB or a failed read).
    /// The watch may ask from now on.
    pub fn seen(&mut self, hash: Option<ContentHash>) {
        self.seen = hash;
        self.read = true;
    }

    /// The next observation of `path` under `root`, when the watch is
    /// idle; `None` before a read of the file landed, and while one is
    /// out, resting or stopped.
    pub fn ask(&mut self, root: WorkspaceRoot, path: RelPath, now: Instant) -> Option<ObserveFile> {
        if !self.read || self.phase != Phase::Idle {
            return None;
        }
        let ask = ObserveFile {
            root,
            path,
            after: self.anchor.clone(),
        };
        self.phase = Phase::Asked {
            ask: ask.clone(),
            at: now,
        };
        Some(ask)
    }

    /// The pause numbered `rest` ended: the watch may ask again.
    pub fn rested(&mut self, rest: u64) {
        if self.phase == (Phase::Resting { rest }) {
            self.phase = Phase::Idle;
        }
    }

    /// Whether an observation failed and the watch stopped.
    #[must_use]
    pub fn stopped(&self) -> bool {
        self.phase == Phase::Stopped
    }

    /// `ask` answered at `now`. `editor`: the version the editor holds,
    /// when the file is edited (else what [`Self::seen`] last recorded is
    /// what the panel shows).
    pub fn answered(
        &mut self,
        ask: &ObserveFile,
        result: Result<ObservedFile, Problem>,
        editor: Option<&ContentHash>,
        now: Instant,
    ) -> Answered {
        let Phase::Asked { ask: out, at } = &self.phase else {
            return Answered::Stale;
        };
        if out != ask {
            return Answered::Stale;
        }
        let asked = *at;
        let observed = match result {
            Ok(observed) => observed,
            // The node's observers are all busy: ask again after the
            // pause, from the same anchor.
            Err(problem) if problem.code() == Some(ErrorCode::Unavailable) => {
                let (rest, wait) = self.rest(asked, now);
                return Answered::Quiet { rest, wait };
            }
            Err(_) => {
                self.phase = Phase::Stopped;
                return Answered::Stopped;
            }
        };
        let shown = editor.or(self.seen.as_ref());
        let changed = match (&self.anchor, &observed) {
            // What the panel shows already: this editor's own save, or a
            // read that saw it first.
            (_, ObservedFile::Ready { revision }) if shown == Some(&revision.hash) => false,
            // The first answer: different from what the panel shows, when
            // that is known.
            (None, ObservedFile::Ready { .. }) => shown.is_some(),
            (None, ObservedFile::Missing) => true,
            (Some(before), current) => before != current,
        };
        self.anchor = Some(observed);
        let (rest, wait) = self.rest(asked, now);
        if changed {
            Answered::Changed { rest, wait }
        } else {
            Answered::Quiet { rest, wait }
        }
    }

    /// Start the next pause after an observation asked at `asked`: its
    /// number and the rest of [`pace::QUIET`].
    fn rest(&mut self, asked: Instant, now: Instant) -> (u64, Duration) {
        self.rests = self.rests.wrapping_add(1);
        let rest = self.rests;
        self.phase = Phase::Resting { rest };
        (rest, pace::quiet_left(asked, now).unwrap_or(Duration::ZERO))
    }
}

#[cfg(test)]
#[path = "watch_tests.rs"]
mod tests;
