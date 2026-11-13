//! Success prose cannot accidentally paint a commit id in the body face.
//! The exact typed outcome remains in the model; code facts have their own
//! elements, just as paths do in the reference's commit summary.

use crate::{
    GitHashText, Lang,
    workspace::git::{
        StashChoice, StashScope, Success, SuccessReceipt, UpstreamChange, WriteIntent,
    },
};

/// Commit facts below a successful action's body-face notice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitSuccessCommits {
    /// The exact saved entry the person acted on, not an index inferred from
    /// the newly returned list (Apply leaves that list unchanged).
    Stash {
        /// Movable reference, paired with the pinned commit.
        reference: String,
        /// Complete pinned commit and its short code caption.
        commit: GitHashText,
    },
    /// This outcome has no commit facts.
    None,
    /// The newly created commit.
    Created(GitHashText),
    /// A fast-forward's complete endpoints, never a half-present pair.
    Pulled {
        /// The checked-out commit before pulling.
        from: GitHashText,
        /// The checked-out commit after pulling.
        to: GitHashText,
    },
}

/// Whole localized notice and separately rendered exact code facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitSuccessText {
    /// Body-face sentence; never embeds a hash.
    pub message: String,
    /// Code-face values; the complete wire outcome stays in `Success` too.
    pub commits: GitSuccessCommits,
}

impl Lang {
    /// Short success notice beside the action. A commit subject is user text;
    /// branch/remote names are values, and hashes occupy separate code elements.
    #[must_use]
    pub fn git_success(self, receipt: &SuccessReceipt) -> GitSuccessText {
        let success = receipt.outcome();
        let intent = receipt.intent();
        let message = match (self, success) {
            (Self::En, Success::Committed(commit)) => format!("Created commit: {}", commit.subject),
            (Self::Ko, Success::Committed(commit)) => {
                format!("커밋을 만들었습니다: {}", commit.subject)
            }
            (Self::En, Success::Fetched) => "Fetched the remote's latest changes.".to_owned(),
            (Self::Ko, Success::Fetched) => "원격 저장소의 최신 변경을 가져왔습니다.".to_owned(),
            (
                Self::En,
                Success::Pushed {
                    remote,
                    branch,
                    upstream: UpstreamChange::Kept,
                },
            ) => format!("Pushed {branch} to {remote}."),
            (
                Self::En,
                Success::Pushed {
                    remote,
                    branch,
                    upstream: UpstreamChange::Set,
                },
            ) => format!("Pushed {branch} to {remote}. The upstream is now set."),
            (
                Self::Ko,
                Success::Pushed {
                    remote,
                    branch,
                    upstream: UpstreamChange::Kept,
                },
            ) => format!("{remote}의 {branch} 브랜치에 푸시했습니다."),
            (
                Self::Ko,
                Success::Pushed {
                    remote,
                    branch,
                    upstream: UpstreamChange::Set,
                },
            ) => format!("{remote}의 {branch} 브랜치에 푸시하고 원격 추적 브랜치를 설정했습니다."),
            (Self::En, Success::Pulled { .. }) => "Pulled the latest changes.".to_owned(),
            (Self::Ko, Success::Pulled { .. }) => "최신 변경을 풀했습니다.".to_owned(),
            (Self::En, Success::UpToDate) => "The branch is already up to date.".to_owned(),
            (Self::Ko, Success::UpToDate) => "이미 최신 상태인 브랜치입니다.".to_owned(),
            (Self::En, Success::Switched(branch)) => format!("Switched to {branch}."),
            (Self::Ko, Success::Switched(branch)) => format!("{branch} 브랜치로 전환했습니다."),
            (Self::En, Success::Renamed(branch)) => format!("Renamed the branch to {branch}."),
            (Self::Ko, Success::Renamed(branch)) => {
                format!("브랜치 이름을 변경했습니다. 새 이름: {branch}.")
            }
            (_, Success::Stashed(_)) => self.git_stash_success(intent).to_owned(),
        };
        let commits = match success {
            Success::Committed(commit) => GitSuccessCommits::Created(GitHashText::of(&commit.id)),
            Success::Pulled { from, to } => GitSuccessCommits::Pulled {
                from: GitHashText::of(from),
                to: GitHashText::of(to),
            },
            Success::Fetched
            | Success::Pushed { .. }
            | Success::UpToDate
            | Success::Switched(_)
            | Success::Renamed(_) => GitSuccessCommits::None,
            Success::Stashed(_) => match intent {
                WriteIntent::Stash { entry, .. } => GitSuccessCommits::Stash {
                    reference: format!("stash@{{{}}}", entry.index),
                    commit: GitHashText::of(&entry.commit),
                },
                WriteIntent::Unspecified
                | WriteIntent::Quick(_)
                | WriteIntent::SwitchBranch
                | WriteIntent::CreateBranch
                | WriteIntent::RenameBranch
                | WriteIntent::SaveStash(_)
                | WriteIntent::Index { .. }
                | WriteIntent::Discard(_) => GitSuccessCommits::None,
            },
        };
        GitSuccessText { message, commits }
    }

    /// Each stash action has its own complete outcome sentence. The legacy
    /// unlabelled tracker can only say a request completed; visual callers
    /// pass the exact intention retained by `Writing::finish`.
    const fn git_stash_success(self, intent: &WriteIntent) -> &'static str {
        match (self, intent) {
            (Self::En, WriteIntent::SaveStash(StashScope::Tracked)) => {
                "Saved changes to tracked files."
            }
            (Self::Ko, WriteIntent::SaveStash(StashScope::Tracked)) => {
                "추적 중인 파일의 변경을 임시 보관했습니다."
            }
            (Self::En, WriteIntent::SaveStash(StashScope::WithUntracked)) => {
                "Saved changes, including new files."
            }
            (Self::Ko, WriteIntent::SaveStash(StashScope::WithUntracked)) => {
                "새 파일을 포함한 변경을 임시 보관했습니다."
            }
            (
                Self::En,
                WriteIntent::Stash {
                    choice: StashChoice::Apply,
                    ..
                },
            ) => "Applied the saved changes. The saved entry was kept.",
            (
                Self::Ko,
                WriteIntent::Stash {
                    choice: StashChoice::Apply,
                    ..
                },
            ) => "보관한 변경을 적용하고 보관 항목은 남겼습니다.",
            (
                Self::En,
                WriteIntent::Stash {
                    choice: StashChoice::Pop,
                    ..
                },
            ) => "Applied the saved changes and removed the saved entry.",
            (
                Self::Ko,
                WriteIntent::Stash {
                    choice: StashChoice::Pop,
                    ..
                },
            ) => "보관한 변경을 적용하고 보관 항목을 삭제했습니다.",
            (
                Self::En,
                WriteIntent::Stash {
                    choice: StashChoice::Drop,
                    ..
                },
            ) => "Deleted the saved entry without applying it.",
            (
                Self::Ko,
                WriteIntent::Stash {
                    choice: StashChoice::Drop,
                    ..
                },
            ) => "보관 항목을 적용하지 않고 삭제했습니다.",
            (
                Self::En,
                WriteIntent::Unspecified
                | WriteIntent::Quick(_)
                | WriteIntent::SwitchBranch
                | WriteIntent::CreateBranch
                | WriteIntent::RenameBranch
                | WriteIntent::Index { .. }
                | WriteIntent::Discard(_),
            ) => "Git completed the request.",
            (
                Self::Ko,
                WriteIntent::Unspecified
                | WriteIntent::Quick(_)
                | WriteIntent::SwitchBranch
                | WriteIntent::CreateBranch
                | WriteIntent::RenameBranch
                | WriteIntent::Index { .. }
                | WriteIntent::Discard(_),
            ) => "Git 요청을 완료했습니다.",
        }
    }
}
