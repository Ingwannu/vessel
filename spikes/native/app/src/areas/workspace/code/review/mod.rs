//! Review comments on the Changes diff (ADR 0124, P6b-2a): the comment
//! being written and the shell's actions on the kept comments (the view's
//! `workspace::review`, kept in the drafts document).
//!
//! [Decision Log]
//! - Purpose: Paseo's inline review (`review/surface.tsx`,
//!   `review/editor`): a "+" on a diff line opens an editor under it, a
//!   saved comment stays under its line with edit and delete, and on a
//!   phone the editor is a sheet (`review/comment-sheet.tsx`).
//! - Constraint: the Changes list is windowed, its rows measured once; a
//!   row whose thread grew or shrank must be measured again, in place.
//!   One editor at a time, as Paseo's editor model holds one.
//! - Choice: the editor (its target, the comment it edits, its field and
//!   the list row it sits under) lives on the Changes state; every action
//!   asks the list to measure that row again, and writes the drafts
//!   document through the composer's debounce.
//! - Cost: an edit started in one comparison is dropped when the
//!   comparison changes (the key changes; Paseo's editor belongs to one
//!   draft key too).

use chrono::Utc;
use vessel_app_kit::field::FieldId;
use vessel_app_kit::ui::{Context, Window};
use vessel_app_kit::{FormMinimum, TextField};
use vessel_app_view::workspace::changes::ScopeKind;
use vessel_app_view::workspace::diff_rows::DiffRow;
use vessel_app_view::workspace::review::{
    CommentId, ReviewKey, ReviewMode, ReviewSide, ReviewTarget, Saved,
};
use vessel_proto::workspace::{LineKind, RelPath};

use crate::shell::Shell;

pub(super) mod rows;
#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod tests;

/// The Changes list's review: the editor, and the phone's sheet it opens in.
pub(super) struct Review {
    /// The comment being written or edited.
    pub(super) editor: Option<Editor>,
    /// The phone's sheet.
    pub(super) dialog: vessel_app_kit::dialog::DialogState,
}

impl Review {
    pub(super) fn new(cx: &mut Context<'_, Shell>) -> Self {
        Self {
            editor: None,
            dialog: vessel_app_kit::dialog::DialogState::new(cx),
        }
    }
}

/// The comment being written or edited.
pub(super) struct Editor {
    /// The key it was opened under.
    pub(super) key: ReviewKey,
    /// The line it is for.
    pub(super) target: ReviewTarget,
    /// The comment it edits; none for a new one.
    pub(super) comment: Option<CommentId>,
    /// Its text.
    pub(super) field: TextField,
    /// The text as last typed (Comment is off while it cannot be saved).
    pub(super) body: String,
    /// The list row it is drawn under.
    pub(super) row: usize,
}

/// The review mode of a comparison.
pub(super) const fn mode_of(scope: ScopeKind) -> ReviewMode {
    match scope {
        ScopeKind::Base => ReviewMode::Base,
        ScopeKind::Uncommitted => ReviewMode::Uncommitted,
        ScopeKind::Staged => ReviewMode::Staged,
        ScopeKind::Unstaged => ReviewMode::Unstaged,
    }
}

/// Where a comment on `line` of `path` sits: a removed line on the old
/// side, an added or unchanged one on the new side (Paseo's unified
/// diff); a hunk's header, a cut, or "No newline" takes none.
pub(super) fn target_of(path: &RelPath, line: &DiffRow) -> Option<ReviewTarget> {
    let DiffRow::Line {
        line: kind,
        old,
        new,
        ..
    } = line
    else {
        return None;
    };
    let (side, number) = match kind {
        LineKind::Removed => (ReviewSide::Old, *old),
        LineKind::Added | LineKind::Context => (ReviewSide::New, *new),
        LineKind::NoNewline => return None,
    };
    Some(ReviewTarget {
        path: path.clone(),
        side,
        line: number.and_then(std::num::NonZeroU32::new)?,
    })
}

impl Shell {
    /// The review key of the diff shown: its machine, root and comparison.
    pub(super) fn review_key(&self) -> Option<ReviewKey> {
        let code = &self.areas.workspace.code;
        let (machine, root) = code.root()?;
        Some(ReviewKey {
            machine: machine.to_owned(),
            root: *root,
            mode: mode_of(code.changes.scope),
            base: None,
            ignore_whitespace: false,
        })
    }

    /// Open the editor under `row` for a new comment on `target`, or for
    /// comment `comment` with its words.
    pub(super) fn review_open(
        &mut self,
        target: ReviewTarget,
        comment: Option<CommentId>,
        row: usize,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(key) = self.review_key() else {
            return;
        };
        let body = comment
            .and_then(|id| {
                let drafts = self.areas.composer.drafts.reviews();
                drafts.comments(&key).iter().find(|c| c.id == id)
            })
            .map(|kept| kept.body.clone())
            .unwrap_or_default();
        let words = self.words.review();
        let field = TextField::form_multiline_with_source(
            words.placeholder(),
            |shell: &mut Self, _id, _text, window, cx| shell.review_save(window, cx),
            Self::review_typed,
            FormMinimum::Default,
            window,
            cx,
        );
        field.set_accessible_label(words.label(), cx);
        if !body.is_empty() {
            field.set_text(&body, window, cx);
        }
        field.focus(window, cx);
        let before = self
            .areas
            .workspace
            .code
            .changes
            .review
            .editor
            .replace(Editor {
                key,
                target,
                comment,
                field,
                body,
                row,
            });
        if let Some(before) = before {
            self.review_measure(before.row);
        }
        self.review_measure(row);
        cx.notify();
    }

    fn review_typed(&mut self, _id: FieldId, text: String, cx: &mut Context<'_, Self>) {
        if let Some(editor) = self.areas.workspace.code.changes.review.editor.as_mut() {
            editor.body = text;
            cx.notify();
        }
    }

    /// Close the editor without saving (Escape, Cancel, the sheet's scrim).
    pub(super) fn review_cancel(&mut self, cx: &mut Context<'_, Self>) {
        if let Some(editor) = self.areas.workspace.code.changes.review.editor.take() {
            self.review_measure(editor.row);
            cx.notify();
        }
    }

    /// Save the editor's words (Comment, Save, Mod+Enter); nothing while
    /// they cannot be saved.
    pub(super) fn review_save(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) {
        let Some(editor) = self.areas.workspace.code.changes.review.editor.as_ref() else {
            return;
        };
        let body = editor.field.text(cx);
        let saved = self.areas.composer.drafts.reviews_mut().save(
            &editor.key,
            editor.comment,
            editor.target.clone(),
            &body,
            Utc::now(),
        );
        match saved {
            Saved::Added(_) | Saved::Edited(_) => {
                if let Some(editor) = self.areas.workspace.code.changes.review.editor.take() {
                    self.review_measure(editor.row);
                }
                self.drafts_changed(cx);
                cx.notify();
            }
            Saved::Blank | Saved::TooLong => {}
        }
    }

    /// Delete comment `id`, drawn in `row`; its open edit goes with it.
    pub(super) fn review_delete(&mut self, id: CommentId, row: usize, cx: &mut Context<'_, Self>) {
        let Some(key) = self.review_key() else {
            return;
        };
        if self
            .areas
            .workspace
            .code
            .changes
            .review
            .editor
            .as_ref()
            .is_some_and(|editor| editor.comment == Some(id))
        {
            self.areas.workspace.code.changes.review.editor = None;
        }
        if self.areas.composer.drafts.reviews_mut().delete(&key, id) {
            self.drafts_changed(cx);
        }
        self.review_measure(row);
        cx.notify();
    }

    /// Row `row` of the Changes list measured again: its thread changed.
    fn review_measure(&self, row: usize) {
        self.areas
            .workspace
            .code
            .changes
            .list
            .changed(row..row.saturating_add(1));
    }
}
