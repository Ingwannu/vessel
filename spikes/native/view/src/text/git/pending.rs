//! A pending row names the keyed request, never the status that happens
//! to arrive while it runs. The spinner replaces its leading glyph.

use crate::{
    Lang,
    workspace::{
        git::{IndexAction, StashChoice, WriteIntent},
        git_quick::GitAction,
    },
};

impl Lang {
    /// Short pending menu/confirmation caption. Paths and saved-entry refs
    /// remain separate code elements; no interpolated value enters the caption.
    #[must_use]
    pub const fn git_pending(self, intent: &WriteIntent) -> &'static str {
        match (self, intent) {
            (Self::En, WriteIntent::Unspecified) => "Working…",
            (Self::Ko, WriteIntent::Unspecified) => "작업 중…",
            (Self::En, WriteIntent::Quick(GitAction::Commit)) => "Committing…",
            (Self::Ko, WriteIntent::Quick(GitAction::Commit)) => "커밋 중…",
            (Self::En, WriteIntent::Quick(GitAction::Push)) => "Pushing…",
            (Self::Ko, WriteIntent::Quick(GitAction::Push)) => "푸시 중…",
            (Self::En, WriteIntent::Quick(GitAction::Pull)) => "Pulling…",
            (Self::Ko, WriteIntent::Quick(GitAction::Pull)) => "풀하는 중…",
            (Self::En, WriteIntent::Quick(GitAction::Fetch)) => "Fetching…",
            (Self::Ko, WriteIntent::Quick(GitAction::Fetch)) => "가져오는 중…",
            (Self::En, WriteIntent::SwitchBranch) => "Switching branch…",
            (Self::Ko, WriteIntent::SwitchBranch) => "브랜치 전환 중…",
            (Self::En, WriteIntent::CreateBranch) => "Creating branch…",
            (Self::Ko, WriteIntent::CreateBranch) => "브랜치 만드는 중…",
            (Self::En, WriteIntent::RenameBranch) => "Renaming branch…",
            (Self::Ko, WriteIntent::RenameBranch) => "브랜치 이름 바꾸는 중…",
            (Self::En, WriteIntent::SaveStash(_)) => "Stashing changes…",
            (Self::Ko, WriteIntent::SaveStash(_)) => "변경 임시 보관 중…",
            (
                Self::En,
                WriteIntent::Stash {
                    choice: StashChoice::Apply,
                    ..
                },
            ) => "Applying stash…",
            (
                Self::Ko,
                WriteIntent::Stash {
                    choice: StashChoice::Apply,
                    ..
                },
            ) => "보관한 변경 적용 중…",
            (
                Self::En,
                WriteIntent::Stash {
                    choice: StashChoice::Pop,
                    ..
                },
            ) => "Popping stash…",
            (
                Self::Ko,
                WriteIntent::Stash {
                    choice: StashChoice::Pop,
                    ..
                },
            ) => "보관한 변경 꺼내는 중…",
            (
                Self::En,
                WriteIntent::Stash {
                    choice: StashChoice::Drop,
                    ..
                },
            ) => "Dropping stash…",
            (
                Self::Ko,
                WriteIntent::Stash {
                    choice: StashChoice::Drop,
                    ..
                },
            ) => "보관한 변경 삭제 중…",
            (
                Self::En,
                WriteIntent::Index {
                    action: IndexAction::Stage,
                    ..
                },
            ) => "Staging…",
            (
                Self::Ko,
                WriteIntent::Index {
                    action: IndexAction::Stage,
                    ..
                },
            ) => "스테이징 중…",
            (
                Self::En,
                WriteIntent::Index {
                    action: IndexAction::Unstage,
                    ..
                },
            ) => "Unstaging…",
            (
                Self::Ko,
                WriteIntent::Index {
                    action: IndexAction::Unstage,
                    ..
                },
            ) => "스테이징 해제 중…",
            (Self::En, WriteIntent::Discard(_)) => "Discarding changes…",
            (Self::Ko, WriteIntent::Discard(_)) => "변경 버리는 중…",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::git::StashScope;
    use vessel_proto::workspace::StashEntry;

    #[test]
    fn every_intention_has_its_own_complete_caption_in_both_languages() {
        let entry = StashEntry {
            index: 0,
            message: "Saved".into(),
            commit: "1111111111111111111111111111111111111111".parse().unwrap(),
            time: "2026-11-13T00:00:00Z".parse().unwrap(),
        };
        let path: vessel_proto::workspace::RelPath = "src/file.rs".parse().unwrap();
        for (intent, en, ko) in [
            (WriteIntent::Unspecified, "Working…", "작업 중…"),
            (
                WriteIntent::Quick(GitAction::Commit),
                "Committing…",
                "커밋 중…",
            ),
            (WriteIntent::Quick(GitAction::Push), "Pushing…", "푸시 중…"),
            (
                WriteIntent::Quick(GitAction::Pull),
                "Pulling…",
                "풀하는 중…",
            ),
            (
                WriteIntent::Quick(GitAction::Fetch),
                "Fetching…",
                "가져오는 중…",
            ),
            (
                WriteIntent::SwitchBranch,
                "Switching branch…",
                "브랜치 전환 중…",
            ),
            (
                WriteIntent::CreateBranch,
                "Creating branch…",
                "브랜치 만드는 중…",
            ),
            (
                WriteIntent::RenameBranch,
                "Renaming branch…",
                "브랜치 이름 바꾸는 중…",
            ),
            (
                WriteIntent::SaveStash(StashScope::Tracked),
                "Stashing changes…",
                "변경 임시 보관 중…",
            ),
            (
                WriteIntent::SaveStash(StashScope::WithUntracked),
                "Stashing changes…",
                "변경 임시 보관 중…",
            ),
            (
                WriteIntent::Stash {
                    choice: StashChoice::Apply,
                    entry: entry.clone(),
                },
                "Applying stash…",
                "보관한 변경 적용 중…",
            ),
            (
                WriteIntent::Stash {
                    choice: StashChoice::Pop,
                    entry: entry.clone(),
                },
                "Popping stash…",
                "보관한 변경 꺼내는 중…",
            ),
            (
                WriteIntent::Stash {
                    choice: StashChoice::Drop,
                    entry,
                },
                "Dropping stash…",
                "보관한 변경 삭제 중…",
            ),
            (
                WriteIntent::Index {
                    action: IndexAction::Stage,
                    path: path.clone(),
                },
                "Staging…",
                "스테이징 중…",
            ),
            (
                WriteIntent::Index {
                    action: IndexAction::Unstage,
                    path: path.clone(),
                },
                "Unstaging…",
                "스테이징 해제 중…",
            ),
            (
                WriteIntent::Discard(path),
                "Discarding changes…",
                "변경 버리는 중…",
            ),
        ] {
            assert_eq!(Lang::En.git_pending(&intent), en);
            assert_eq!(Lang::Ko.git_pending(&intent), ko);
        }
    }
}
