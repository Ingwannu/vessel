//! Words for each toolbar and dialog place, kept separate from raw data.

use crate::Lang;
use vessel_proto::workspace::CommitStage;

/// Fixed git screen words in one complete language table.
#[derive(Debug)]
pub struct GitLabels {
    /// Title of the confirmation when a branch switch would overwrite
    /// local changes (Paseo's `uncommittedTitle`).
    pub switch_dirty_title: &'static str,
    /// Its whole question, under the title (Paseo's `uncommittedMessage`).
    pub switch_dirty_message: &'static str,
    /// Its confirming button: stash, then switch (short).
    pub stash_and_switch: &'static str,
    /// Short label before the commit summary’s branch name.
    pub commit_branch: &'static str,
    /// Short label above the commit summary’s path list.
    pub commit_files: &'static str,
    /// Short empty value in the commit summary’s path list.
    pub commit_files_none: &'static str,
    /// Short preview value when no eligible paths were listed in a partial reply.
    pub commit_files_unlisted: &'static str,
    /// Short warning text beside the commit summary’s default branch.
    pub commit_default_branch: &'static str,
    /// Notice when the node could not include every changed path.
    pub changes_more: &'static str,

    /// The menu-only trigger's own tooltip.
    pub options_tip: &'static str,
    /// Short button label for rereading failed or unavailable Git status.
    pub refresh: &'static str,
    /// Changes-head status while the initial repository read is pending.
    pub reading: &'static str,
    /// Status sentence beside disabled actions while a write is pending.
    pub busy: &'static str,
    /// Status sentence while existing repository facts are refreshed.
    pub read_first: &'static str,
    /// Unavailable action guidance before any repository read has supplied facts.
    pub status_missing: &'static str,
    /// Unavailable action guidance after a failed read; retry is required.
    pub status_failed: &'static str,
    /// Explanation when there is no checked-out branch to rename.
    pub no_branch: &'static str,
    /// Branch facts beneath the trigger when no upstream is configured.
    pub no_upstream: &'static str,
    /// Short primary button label for the first push of a branch.
    pub push_upstream: &'static str,
    /// Commit dialog heading.
    pub commit_title: &'static str,
    /// Short menu action label that opens the commit dialog.
    pub commit_menu: &'static str,
    /// Short menu action label that opens the branch picker.
    pub switch_menu: &'static str,
    /// Short menu action label that opens the new-branch form.
    pub create_menu: &'static str,
    /// Short menu action label that opens the rename form.
    pub rename_menu: &'static str,
    /// Short menu action label for saving tracked changes.
    pub stash_menu: &'static str,
    /// Short menu action label for saving tracked and untracked changes.
    pub stash_untracked: &'static str,
    /// Short menu action label that opens the saved-changes list.
    pub stashes_menu: &'static str,
    /// Branch picker dialog heading.
    pub switch_title: &'static str,
    /// New-branch form heading.
    pub create_title: &'static str,
    /// Rename-branch form heading.
    pub rename_title: &'static str,
    /// Saved-changes dialog heading.
    pub stashes_title: &'static str,
    /// Branch picker loading sentence.
    pub reading_branches: &'static str,
    /// Saved-changes list loading sentence.
    pub reading_stashes: &'static str,
    /// Branch picker empty-state sentence.
    pub empty_branches: &'static str,
    /// Saved-changes list empty-state sentence.
    pub empty_stashes: &'static str,
    /// Branch picker warning when the machine returned a truncated list.
    pub branches_more: &'static str,
    /// Saved-changes warning when the machine returned a truncated list.
    pub stashes_more: &'static str,
    /// Short placeholder in the branch filter field.
    pub filter: &'static str,
    /// New-branch name field label.
    pub branch_name: &'static str,
    /// Rename-branch name field label.
    pub new_name: &'static str,
    /// Short button label that creates and switches branches.
    pub create: &'static str,
    /// Short button label that renames the checked-out branch.
    pub rename: &'static str,
    /// Commit-scope selector field label.
    pub commit_scope: &'static str,
    /// Short scope selector label for already staged files.
    pub staged: &'static str,
    /// Short scope selector label for all tracked files.
    pub tracked: &'static str,
    /// Short scope selector label for tracked and untracked files.
    pub everything: &'static str,
    /// Commit-message multiline field label.
    pub message: &'static str,
    /// Commit-message multiline field placeholder.
    pub message_hint: &'static str,
    /// Short file-row button label for staging the path.
    pub stage: &'static str,
    /// Short file-row button label for unstaging the path.
    pub unstage: &'static str,
    /// Short file-row button label that opens a destructive confirmation.
    pub discard: &'static str,
    /// Destructive discard confirmation heading.
    pub discard_title: &'static str,
    /// Short destructive confirmation button label for discarding a path.
    pub discard_confirm: &'static str,
    /// Destructive saved-entry deletion confirmation heading.
    pub drop_title: &'static str,
    /// Short destructive confirmation button label for deleting the saved entry.
    pub drop_confirm: &'static str,
    /// Short saved-entry row label that opens its deletion confirmation.
    pub drop: &'static str,
    /// Short saved-entry row button label for applying without deleting the entry.
    pub apply: &'static str,
    /// Short saved-entry row button label for applying and deleting the entry.
    pub pop: &'static str,
    /// Short dialog footer button label that closes without submitting.
    pub cancel: &'static str,
    /// Short dialog button label that closes the current surface.
    pub close: &'static str,
    /// Short list button label that retries a failed read.
    pub retry: &'static str,
    /// Heading above the machine-provided Git output; the output is not translated.
    pub details: &'static str,
    /// Commit file list empty-state sentence.
    pub empty_changes: &'static str,
}

const EN: GitLabels = GitLabels {
    switch_dirty_title: "Uncommitted changes",
    switch_dirty_message: "You have uncommitted changes. Stash them before switching branches?",
    stash_and_switch: "Stash & Switch",
    commit_branch: "Branch",
    commit_files: "Files",
    commit_files_none: "none",
    commit_files_unlisted: "Not listed",
    commit_default_branch: "Default branch",
    changes_more: "Only part of the changed file list is shown. More changed files exist in this repository.",
    options_tip: "Git actions",
    refresh: "Refresh",
    reading: "Reading repository status…",
    busy: "A Git action is running. Wait for it to finish.",
    read_first: "Repository status is being refreshed. Wait for it before continuing.",
    status_missing: "Repository status has not been read yet. Read it before continuing.",
    status_failed: "Repository status could not be read. Refresh it before continuing.",
    no_branch: "No branch is checked out. Switch to a branch before renaming it.",
    no_upstream: "No upstream is configured for this branch.",
    push_upstream: "Push and set upstream",
    commit_title: "Commit changes",
    commit_menu: "Commit…",
    switch_menu: "Switch branch…",
    create_menu: "New branch…",
    rename_menu: "Rename branch…",
    stash_menu: "Stash changes",
    stash_untracked: "Stash, with untracked files",
    stashes_menu: "Stashes…",
    switch_title: "Switch branch",
    create_title: "New branch",
    rename_title: "Rename branch",
    stashes_title: "Stashes",
    reading_branches: "Loading branches…",
    reading_stashes: "Loading the stash…",
    empty_branches: "No branches found.",
    empty_stashes: "Nothing is stashed in this repository.",
    branches_more: "The branch list is incomplete. More branches exist on the machine.",
    stashes_more: "The stash list is incomplete. More entries exist on the machine.",
    filter: "Filter branches…",
    branch_name: "Branch name",
    new_name: "New name",
    create: "Create and switch",
    rename: "Rename",
    commit_scope: "Changes to commit",
    staged: "Staged changes only",
    tracked: "All tracked changes",
    everything: "All changes, including new files",
    message: "Commit message",
    message_hint: "What this commit does",
    stage: "Stage",
    unstage: "Unstage",
    discard: "Discard…",
    discard_title: "Discard changes?",
    discard_confirm: "Discard changes",
    drop_title: "Drop this stash?",
    drop_confirm: "Drop stash",
    drop: "Drop…",
    apply: "Apply",
    pop: "Pop",
    cancel: "Cancel",
    close: "Close",
    retry: "Try again",
    details: "Git details",
    empty_changes: "There are no local changes.",
};

const KO: GitLabels = GitLabels {
    switch_dirty_title: "커밋하지 않은 변경",
    switch_dirty_message: "커밋하지 않은 변경이 있습니다. 브랜치를 전환하기 전에 임시 보관할까요?",
    stash_and_switch: "임시 보관하고 전환",
    commit_branch: "브랜치",
    commit_files: "파일",
    commit_files_none: "없음",
    commit_files_unlisted: "목록에 없음",
    commit_default_branch: "기본 브랜치",
    changes_more: "변경 파일 목록의 일부입니다. 이 저장소에 변경된 파일이 더 있습니다.",
    options_tip: "Git 작업",
    refresh: "새로 고침",
    reading: "저장소 상태 읽는 중…",
    busy: "Git 동작을 실행 중입니다. 끝날 때까지 기다리세요.",
    read_first: "저장소 상태를 새로 읽고 있습니다. 끝난 뒤 계속하세요.",
    status_missing: "저장소 상태를 아직 읽지 않았습니다. 상태를 읽은 뒤 계속하세요.",
    status_failed: "저장소 상태를 읽지 못했습니다. 상태를 새로 읽은 뒤 계속하세요.",
    no_branch: "체크아웃된 브랜치가 없습니다. 이름을 바꾸려면 먼저 브랜치로 전환하세요.",
    no_upstream: "이 브랜치에 원격 추적 브랜치가 설정되지 않았습니다.",
    push_upstream: "푸시하고 원격 추적 설정",
    commit_title: "변경 커밋",
    commit_menu: "커밋…",
    switch_menu: "브랜치 전환…",
    create_menu: "새 브랜치…",
    rename_menu: "브랜치 이름 변경…",
    stash_menu: "변경 임시 보관",
    stash_untracked: "추적하지 않는 파일까지 임시 보관",
    stashes_menu: "임시 보관 목록…",
    switch_title: "브랜치 전환",
    create_title: "새 브랜치",
    rename_title: "브랜치 이름 변경",
    stashes_title: "임시 보관 목록",
    reading_branches: "브랜치 읽는 중…",
    reading_stashes: "임시 보관 목록 읽는 중…",
    empty_branches: "찾은 브랜치가 없습니다.",
    empty_stashes: "이 저장소에 임시 보관한 변경이 없습니다.",
    branches_more: "브랜치 목록의 일부입니다. 이 머신에 브랜치가 더 있습니다.",
    stashes_more: "임시 보관 목록의 일부입니다. 이 머신에 항목이 더 있습니다.",
    filter: "브랜치 검색…",
    branch_name: "브랜치 이름",
    new_name: "새 이름",
    create: "만들고 전환",
    rename: "이름 변경",
    commit_scope: "커밋할 변경",
    staged: "스테이징한 변경만",
    tracked: "추적 중인 파일의 모든 변경",
    everything: "새 파일을 포함한 모든 변경",
    message: "커밋 메시지",
    message_hint: "이 커밋이 하는 일",
    stage: "스테이지",
    unstage: "스테이지 취소",
    discard: "변경 버리기…",
    discard_title: "변경을 버릴까요?",
    discard_confirm: "변경 버리기",
    drop_title: "임시 보관을 삭제할까요?",
    drop_confirm: "임시 보관 삭제",
    drop: "삭제…",
    apply: "적용",
    pop: "적용하고 보관 삭제",
    cancel: "취소",
    close: "닫기",
    retry: "다시 시도",
    details: "Git의 상세 출력",
    empty_changes: "로컬 변경이 없습니다.",
};

impl Lang {
    /// The commit scope’s whole value in its select and choice list.
    #[must_use]
    pub const fn git_commit_scope(self, scope: CommitStage) -> &'static str {
        let labels = self.git_labels();
        match scope {
            CommitStage::Staged => labels.staged,
            CommitStage::Tracked => labels.tracked,
            CommitStage::Everything => labels.everything,
        }
    }

    /// Toolbar and dialog words in the current language.
    #[must_use]
    pub const fn git_labels(self) -> &'static GitLabels {
        match self {
            Self::En => &EN,
            Self::Ko => &KO,
        }
    }
}

#[cfg(test)]
mod scope_tests {
    use super::*;

    #[test]
    fn the_commit_scope_choices_are_whole_labels_in_both_languages() {
        let scopes = [
            CommitStage::Staged,
            CommitStage::Tracked,
            CommitStage::Everything,
        ];
        assert_eq!(
            scopes.map(|scope| Lang::En.git_commit_scope(scope)),
            [
                "Staged changes only",
                "All tracked changes",
                "All changes, including new files",
            ]
        );
        assert_eq!(
            scopes.map(|scope| Lang::Ko.git_commit_scope(scope)),
            [
                "스테이징한 변경만",
                "추적 중인 파일의 모든 변경",
                "새 파일을 포함한 모든 변경",
            ]
        );
        assert_eq!(
            Lang::En.git_labels().changes_more,
            "Only part of the changed file list is shown. More changed files exist in this repository."
        );
        assert_eq!(
            Lang::Ko.git_labels().changes_more,
            "변경 파일 목록의 일부입니다. 이 저장소에 변경된 파일이 더 있습니다."
        );
    }
}
