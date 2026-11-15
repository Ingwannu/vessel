//! Paths, saved entries and commit dates have their own places rather
//! than reusing a badge label as a sentence or showing raw wire time.

use crate::devices::time::{Age, exact_time};
use crate::workspace::git_quick::GitAction;
use crate::{Lang, workspace::git::Action};
use chrono::{DateTime, Datelike as _, FixedOffset, Utc};
use vessel_proto::workspace::{GitOperation, GitStatus, StashEntry};

/// Saved-entry facts occupy separate code/time elements in the row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitStashText {
    /// Exact movable ref, in the code face; the entry also retains its pinned commit.
    pub reference: String,
    /// Whole relative-time sentence, in the body face.
    pub saved: String,
    /// Exact viewer-local time to the second, in the time caption's tooltip.
    pub exact: String,
}

impl Lang {
    /// The primary half's short action, distinct from "Commit…" in a menu.
    #[must_use]
    pub fn git_primary_label(self, action: Action, status: &GitStatus) -> &'static str {
        match action {
            Action::Quick(GitAction::Push) if status.upstream.is_none() => {
                self.git_labels().push_upstream
            }
            Action::Quick(action) => self.workspace().action(action),
            Action::SwitchBranch
            | Action::CreateBranch
            | Action::RenameBranch
            | Action::SaveStash(_)
            | Action::Stashes => self.git_labels().options_tip,
        }
    }

    /// A saved-entry row keeps its ref/commit separate from its time words.
    #[must_use]
    pub fn git_stash_meta(
        self,
        entry: &StashEntry,
        now: DateTime<Utc>,
        offset: FixedOffset,
    ) -> GitStashText {
        GitStashText {
            reference: format!("stash@{{{}}}", entry.index),
            saved: self.git_stash_saved(Age::of(entry.time, now, offset)),
            exact: exact_time(entry.time, offset),
        }
    }

    /// A saved-entry row's whole relative-time sentence, never assembled from
    /// the device page's compact fragments. Counts arrive as numbers.
    #[must_use]
    pub fn git_stash_saved(self, age: Age) -> String {
        match (self, age) {
            (Self::En, Age::Now) => "Saved just now".to_owned(),
            (Self::Ko, Age::Now) => "방금 보관".to_owned(),
            (Self::En, Age::Minutes(1)) => "Saved 1 minute ago".to_owned(),
            (Self::En, Age::Minutes(n)) => format!("Saved {n} minutes ago"),
            (Self::Ko, Age::Minutes(n)) => format!("{n}분 전에 보관"),
            (Self::En, Age::Hours(1)) => "Saved 1 hour ago".to_owned(),
            (Self::En, Age::Hours(n)) => format!("Saved {n} hours ago"),
            (Self::Ko, Age::Hours(n)) => format!("{n}시간 전에 보관"),
            (Self::En, Age::Days(1)) => "Saved 1 day ago".to_owned(),
            (Self::En, Age::Days(n)) => format!("Saved {n} days ago"),
            (Self::Ko, Age::Days(n)) => format!("{n}일 전에 보관"),
            (Self::En, Age::Date(at)) => format!("Saved on {}", at.format("%b %-d")),
            (Self::Ko, Age::Date(at)) => format!("{}월 {}일에 보관", at.month(), at.day()),
        }
    }

    /// An in-progress operation is a sentence with its next step, not
    /// the fragment ("a merge") used inside a refusal's larger sentence.
    #[must_use]
    pub fn git_operation_line(self, operation: GitOperation) -> String {
        match (self, operation) {
            (Self::En, GitOperation::Merge) =>
                "The repository is in the middle of a merge. Finish or abort it before changing branches.",
            (Self::Ko, GitOperation::Merge) =>
                "저장소가 병합 도중입니다. 브랜치를 바꾸기 전에 끝내거나 중단하세요.",
            (Self::En, GitOperation::Rebase) =>
                "The repository is in the middle of a rebase. Finish or abort it before changing branches.",
            (Self::Ko, GitOperation::Rebase) =>
                "저장소가 리베이스 도중입니다. 브랜치를 바꾸기 전에 끝내거나 중단하세요.",
            (Self::En, GitOperation::CherryPick) =>
                "The repository is in the middle of a cherry-pick. Finish or abort it before changing branches.",
            (Self::Ko, GitOperation::CherryPick) =>
                "저장소가 체리픽 도중입니다. 브랜치를 바꾸기 전에 끝내거나 중단하세요.",
            (Self::En, GitOperation::Revert) =>
                "The repository is in the middle of a revert. Finish or abort it before changing branches.",
            (Self::Ko, GitOperation::Revert) =>
                "저장소가 되돌리기 도중입니다. 브랜치를 바꾸기 전에 끝내거나 중단하세요.",
            (Self::En, GitOperation::Bisect) =>
                "The repository is in the middle of a bisect. Finish or abort it before changing branches.",
            (Self::Ko, GitOperation::Bisect) =>
                "저장소가 이분 탐색 도중입니다. 브랜치를 바꾸기 전에 끝내거나 중단하세요.",
        }.to_owned()
    }
}
