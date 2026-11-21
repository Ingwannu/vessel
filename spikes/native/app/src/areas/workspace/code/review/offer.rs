//! The review the open session's composer offers while the Changes diff
//! shows comments (Paseo's `usePublishWorkingDiffAttachment` and the
//! workspace attachments of `composer/attachments/workspace.tsx`): a
//! "Review · N comments" pill, removable in that composer, sent as ADR
//! 0124's `review` block; once sent, the key's comments are cleared.
//!
//! [Decision Log]
//! - Purpose: plan item 5 (F-E83): the person sends the review with their
//!   own words from the composer, as Paseo's composer does.
//! - Constraint: the native app shows one session at a time; its code
//!   panel's Changes is that session's workspace. The diff is read a file
//!   at a time, so a comment on a file not read yet has no rows.
//! - Choice: the pill lives in the open session's context tray, kept in
//!   step whenever the comments change or a file's diff is read; a file
//!   with comments that is not read yet is asked for. The tray's own
//!   rules keep a removed pill suppressed while newer snapshots of the
//!   same key replace it (source-owned, replaceable), and the pill goes
//!   when no comment is left in the diff.
//! - Cost: composers of sessions not open do not show the pill (Paseo
//!   shows it in every composer of the workspace; the native app draws
//!   one).

use vessel_app_view::composer::context::ContextAttachment;
use vessel_app_view::composer::drafts::DraftKey;
use vessel_app_view::workspace::changes::FileState;
use vessel_app_view::workspace::diff_rows::DiffBody;
use vessel_app_view::workspace::review::{Offer, snapshot};
use vessel_proto::workspace::RelPath;

use super::super::Summary;
use crate::shell::Shell;

impl Shell {
    /// The folder the open session works in (its card's), the review's root.
    fn review_root_path(&self) -> Option<String> {
        let open = self.areas.session.open.as_ref()?;
        self.fleet
            .machines
            .iter()
            .flat_map(|machine| machine.sessions.iter())
            .find(|card| card.session == open.session)
            .and_then(|card| card.detail.cwd.clone())
    }

    /// Ask for the diffs of changed files with comments not read yet.
    fn review_read_commented(&mut self) {
        let Some(key) = self.review_key() else {
            return;
        };
        let changes = &self.areas.workspace.code.changes;
        let Summary::Ready(summary) = &changes.summary else {
            return;
        };
        let mut unread: Vec<RelPath> = self
            .areas
            .composer
            .drafts
            .reviews()
            .comments(&key)
            .iter()
            .map(|comment| comment.target.path.clone())
            .filter(|path| {
                summary.files.iter().any(|file| file.path == *path)
                    && matches!(changes.states.get(path), None | Some(FileState::Unread))
            })
            .collect();
        unread.dedup();
        for path in &unread {
            self.read_diff(path, None);
        }
    }

    /// The review of the comments kept for the diff shown, over the rows
    /// read so far.
    pub(in super::super) fn review_offer(&self) -> Option<Offer> {
        let key = self.review_key()?;
        let comments = self.areas.composer.drafts.reviews().comments(&key);
        if comments.is_empty() {
            return None;
        }
        let root = self.review_root_path()?;
        let states = &self.areas.workspace.code.changes.states;
        let made = snapshot(&root, &key, comments, |path| match states.get(path) {
            Some(
                FileState::Read(DiffBody::Rows { rows, .. })
                | FileState::Loading(Some(DiffBody::Rows { rows, .. })),
            ) => Some(&**rows),
            Some(
                FileState::Read(_) | FileState::Loading(_) | FileState::Unread | FileState::Failed(_),
            )
            | None => None,
        })?;
        Some(Offer {
            key,
            snapshot: made,
        })
    }

    /// The open session's composer offers the review now shown, or no
    /// longer offers one.
    pub(crate) fn review_offer_sync(&mut self) {
        let Some(session) = self.areas.session.open.as_ref().map(|open| open.session) else {
            return;
        };
        let target = DraftKey::Session { session };
        self.review_read_commented();
        match self.review_offer() {
            Some(offer) => {
                let tray = self.areas.composer.contexts.tray(target);
                if tray
                    .visible()
                    .any(|held| matches!(held, ContextAttachment::Review(kept) if **kept == offer))
                {
                    return;
                }
                // A full tray keeps what the person chose; the review waits.
                drop(tray.add(ContextAttachment::Review(Box::new(offer))));
            }
            None => {
                if let Some(tray) = self.areas.composer.contexts.get_mut(target) {
                    tray.withdraw_reviews();
                }
            }
        }
    }
}
