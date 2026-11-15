//! Action labels and exact notices. A notice is reworded from its typed
//! reason each frame; raw output remains separate, never the guidance.

use vessel_proto::workspace::{FileState, GitRefusal, GitStatus};

use crate::workspace::git::{
    Action, DiscardDecision, DiscardEffect, Issue, StashScope, Unavailable,
};
use crate::workspace::git_quick::{Blocked, GitAction};
use crate::{GitFailureText, Lang};

impl Lang {
    /// A menu label, distinct from the primary control's short label.
    #[must_use]
    pub fn git_action_label(self, action: Action, status: &GitStatus) -> &'static str {
        let labels = self.git_labels();
        match action {
            Action::Quick(GitAction::Commit) => labels.commit_menu,
            Action::Quick(GitAction::Push)
                if status.upstream.is_none() && status.branch.is_some() =>
            {
                labels.push_upstream
            }
            Action::Quick(action) => self.workspace().action(action),
            Action::SwitchBranch => labels.switch_menu,
            Action::CreateBranch => labels.create_menu,
            Action::RenameBranch => labels.rename_menu,
            Action::SaveStash(StashScope::Tracked) => labels.stash_menu,
            Action::SaveStash(StashScope::WithUntracked) => labels.stash_untracked,
            Action::Stashes => labels.stashes_menu,
        }
    }

    /// An unavailable action's explanation, also shown under the primary
    /// control so its disabled appearance is not its only explanation.
    #[must_use]
    pub const fn git_unavailable(self, reason: Unavailable) -> &'static str {
        let labels = self.git_labels();
        match (self, reason) {
            (_, Unavailable::Writing) => labels.busy,
            (_, Unavailable::Reading) => labels.read_first,
            (_, Unavailable::StatusMissing) => labels.status_missing,
            (_, Unavailable::StatusFailed) => labels.status_failed,
            (_, Unavailable::NoBranch) => labels.no_branch,
            (Self::En, Unavailable::NoCommits) => {
                "This repository has no commits yet. Make its first commit before continuing."
            }
            (Self::Ko, Unavailable::NoCommits) => {
                "아직 커밋이 없는 저장소입니다. 첫 커밋을 만든 뒤 계속하세요."
            }
            (Self::En, Unavailable::NoChanges) => {
                "There are no local changes to stash. Make changes before stashing."
            }
            (Self::Ko, Unavailable::NoChanges) => {
                "임시 보관할 로컬 변경이 없습니다. 먼저 파일을 변경하세요."
            }
            (Self::En, Unavailable::Git(Blocked::Clean)) => {
                "There are no local changes. Make changes before committing."
            }
            (Self::Ko, Unavailable::Git(Blocked::Clean)) => {
                "로컬 변경이 없습니다. 커밋하려면 먼저 파일을 변경하세요."
            }
            (Self::En, Unavailable::Git(Blocked::DetachedPush)) => {
                "No branch is checked out. Switch to a branch before pushing."
            }
            (Self::Ko, Unavailable::Git(Blocked::DetachedPush)) => {
                "체크아웃된 브랜치가 없습니다. 푸시하려면 먼저 브랜치로 전환하세요."
            }
            (Self::En, Unavailable::Git(Blocked::DetachedPull)) => {
                "No branch is checked out. Switch to a branch before pulling."
            }
            (Self::Ko, Unavailable::Git(Blocked::DetachedPull)) => {
                "체크아웃된 브랜치가 없습니다. 풀하려면 먼저 브랜치로 전환하세요."
            }
            (Self::En, Unavailable::Git(Blocked::Diverged)) => {
                "This branch and its upstream have both changed. Merge or rebase before pushing."
            }
            (Self::Ko, Unavailable::Git(Blocked::Diverged)) => {
                "이 브랜치와 원격 추적 브랜치가 서로 다르게 바뀌었습니다. 푸시 전에 병합하거나 리베이스하세요."
            }
            (Self::En, Unavailable::Git(Blocked::Behind)) => {
                "The upstream has newer commits. Pull before pushing."
            }
            (Self::Ko, Unavailable::Git(Blocked::Behind)) => {
                "원격 추적 브랜치에 새 커밋이 있습니다. 푸시하기 전에 풀하세요."
            }
            (Self::En, Unavailable::Git(Blocked::NothingToPush)) => {
                "There are no local commits to push."
            }
            (Self::Ko, Unavailable::Git(Blocked::NothingToPush)) => "푸시할 로컬 커밋이 없습니다.",
            (Self::En, Unavailable::Git(Blocked::NoUpstream)) => {
                "This branch has no upstream. Push it to set one before pulling."
            }
            (Self::Ko, Unavailable::Git(Blocked::NoUpstream)) => {
                "이 브랜치에 원격 추적 브랜치가 없습니다. 풀하기 전에 푸시해서 설정하세요."
            }
        }
    }

    /// The entire retained failure, including paths/output the node named.
    #[must_use]
    pub fn git_issue(self, issue: &Issue) -> GitFailureText {
        match issue {
            Issue::Call(problem) => self.git_call_failure(problem),
            Issue::Message(error) => self.git_message_failure(error),
            Issue::Name(error) => self.git_name_failure(error),
            Issue::Switch(error) => self.git_switch_failure(error),
            Issue::Keys => self.git_keys_exhausted(),
            Issue::Refused(GitRefusal::Unknown) => {
                let (title, next) = match self {
                    Self::En => (
                        "Git refused this action for a reason this app cannot read.",
                        "Update Vessel on this machine and in the app, then try again.",
                    ),
                    Self::Ko => (
                        "Git이 이 앱이 읽을 수 없는 이유로 동작을 거부했습니다.",
                        "머신과 앱의 Vessel을 업데이트한 뒤 다시 시도하세요.",
                    ),
                };
                GitFailureText {
                    title: title.to_owned(),
                    next: next.to_owned(),
                    detail: None,
                }
            }
            Issue::Refused(refusal) => {
                let text = self.workspace().refusal(refusal);
                let mut details: Vec<String> = text.paths.iter().map(ToString::to_string).collect();
                details.extend(text.output.map(str::to_owned));
                GitFailureText {
                    title: text.title,
                    next: text.next,
                    detail: (!details.is_empty()).then(|| details.join("\n")),
                }
            }
        }
    }

    /// What a confirmed discard will destroy. Its exact path belongs in a
    /// separate code element, never glued to this body-face warning.
    #[must_use]
    pub fn git_discard_copy(self, decision: &DiscardDecision) -> String {
        let effect = match (self, decision.effect()) {
            (Self::En, DiscardEffect::Restore) => {
                "Staged and unstaged changes to this file will be lost. Git will restore the last commit. This cannot be undone."
            }
            (Self::Ko, DiscardEffect::Restore) => {
                "이 파일에서 스테이지한 변경과 스테이지하지 않은 변경이 사라지고 마지막 커밋으로 돌아갑니다. 되돌릴 수 없습니다."
            }
            (Self::En, DiscardEffect::DeleteUntracked | DiscardEffect::DeleteAdded) => {
                "This new file will be deleted, including all of its contents. This cannot be undone."
            }
            (Self::Ko, DiscardEffect::DeleteUntracked | DiscardEffect::DeleteAdded) => {
                "이 새 파일과 그 안의 모든 내용이 삭제됩니다. 되돌릴 수 없습니다."
            }
        };
        effect.to_owned()
    }

    /// Whole destructive warning; the entry’s message, ref and commit each
    /// have their own body/code elements in the confirmation.
    #[must_use]
    pub const fn git_drop_copy(self) -> &'static str {
        match self {
            Self::En => {
                "This saved entry will be deleted without applying it. This cannot be undone."
            }
            Self::Ko => "이 보관 항목을 적용하지 않고 삭제합니다. 되돌릴 수 없습니다.",
        }
    }

    /// Both sides of a mixed-status path remain visible as words.
    #[must_use]
    pub fn git_file_state(self, index: FileState, worktree: FileState) -> String {
        match self {
            Self::En => format!(
                "Staged: {}. Unstaged: {}.",
                self.git_file_word(index),
                self.git_file_word(worktree)
            ),
            Self::Ko => format!(
                "스테이지한 변경: {}. 스테이지하지 않은 변경: {}.",
                self.git_file_word(index),
                self.git_file_word(worktree)
            ),
        }
    }

    /// One side's change in one word: a commit form row's state (short).
    #[must_use]
    pub const fn git_file_word(self, state: FileState) -> &'static str {
        match (self, state) {
            (Self::En, FileState::Unmodified) => "unchanged",
            (Self::Ko, FileState::Unmodified) => "변경 없음",
            (Self::En, FileState::Modified) => "modified",
            (Self::Ko, FileState::Modified) => "수정",
            (Self::En, FileState::Added) => "added",
            (Self::Ko, FileState::Added) => "추가",
            (Self::En, FileState::Deleted) => "deleted",
            (Self::Ko, FileState::Deleted) => "삭제",
            (Self::En, FileState::Renamed) => "renamed",
            (Self::Ko, FileState::Renamed) => "이름 변경",
            (Self::En, FileState::Copied) => "copied",
            (Self::Ko, FileState::Copied) => "복사",
            (Self::En, FileState::TypeChanged) => "file type changed",
            (Self::Ko, FileState::TypeChanged) => "파일 형식 변경",
            (Self::En, FileState::Untracked) => "not tracked",
            (Self::Ko, FileState::Untracked) => "추적하지 않음",
            (Self::En, FileState::Conflicted) => "conflicted",
            (Self::Ko, FileState::Conflicted) => "충돌",
            (Self::En, FileState::Unknown) => "unrecognized change",
            (Self::Ko, FileState::Unknown) => "알 수 없는 변경",
        }
    }
}
