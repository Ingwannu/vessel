use crate::workspace::git::{Action, DiscardDecision, Issue, StashScope, Unavailable};
use crate::workspace::git_quick::{Blocked, GitAction};
use crate::{GitFailureText, Lang};
use vessel_proto::workspace::{FileState, GitRefusal, GitStatus, StatusEntry, WorkspaceRoot};

#[test]
fn each_unavailable_reason_is_a_whole_explanation_in_both_languages() {
    let cases = [
        (
            Unavailable::Writing,
            "A Git action is running. Wait for it to finish.",
            "Git 동작을 실행 중입니다. 끝날 때까지 기다리세요.",
        ),
        (
            Unavailable::Reading,
            "Repository status is being refreshed. Wait for it before continuing.",
            "저장소 상태를 새로 읽고 있습니다. 끝난 뒤 계속하세요.",
        ),
        (
            Unavailable::StatusMissing,
            "Repository status has not been read yet. Read it before continuing.",
            "저장소 상태를 아직 읽지 않았습니다. 상태를 읽은 뒤 계속하세요.",
        ),
        (
            Unavailable::StatusFailed,
            "Repository status could not be read. Refresh it before continuing.",
            "저장소 상태를 읽지 못했습니다. 상태를 새로 읽은 뒤 계속하세요.",
        ),
        (
            Unavailable::NoBranch,
            "No branch is checked out. Switch to a branch before renaming it.",
            "체크아웃된 브랜치가 없습니다. 이름을 바꾸려면 먼저 브랜치로 전환하세요.",
        ),
        (
            Unavailable::NoCommits,
            "This repository has no commits yet. Make its first commit before continuing.",
            "아직 커밋이 없는 저장소입니다. 첫 커밋을 만든 뒤 계속하세요.",
        ),
        (
            Unavailable::NoChanges,
            "There are no local changes to stash. Make changes before stashing.",
            "임시 보관할 로컬 변경이 없습니다. 먼저 파일을 변경하세요.",
        ),
        (
            Unavailable::Git(Blocked::Clean),
            "There are no local changes. Make changes before committing.",
            "로컬 변경이 없습니다. 커밋하려면 먼저 파일을 변경하세요.",
        ),
        (
            Unavailable::Git(Blocked::DetachedPush),
            "No branch is checked out. Switch to a branch before pushing.",
            "체크아웃된 브랜치가 없습니다. 푸시하려면 먼저 브랜치로 전환하세요.",
        ),
        (
            Unavailable::Git(Blocked::DetachedPull),
            "No branch is checked out. Switch to a branch before pulling.",
            "체크아웃된 브랜치가 없습니다. 풀하려면 먼저 브랜치로 전환하세요.",
        ),
        (
            Unavailable::Git(Blocked::Diverged),
            "This branch and its upstream have both changed. Merge or rebase before pushing.",
            "이 브랜치와 원격 추적 브랜치가 서로 다르게 바뀌었습니다. 푸시 전에 병합하거나 리베이스하세요.",
        ),
        (
            Unavailable::Git(Blocked::Behind),
            "The upstream has newer commits. Pull before pushing.",
            "원격 추적 브랜치에 새 커밋이 있습니다. 푸시하기 전에 풀하세요.",
        ),
        (
            Unavailable::Git(Blocked::NothingToPush),
            "There are no local commits to push.",
            "푸시할 로컬 커밋이 없습니다.",
        ),
        (
            Unavailable::Git(Blocked::NoUpstream),
            "This branch has no upstream. Push it to set one before pulling.",
            "이 브랜치에 원격 추적 브랜치가 없습니다. 풀하기 전에 푸시해서 설정하세요.",
        ),
    ];
    for (reason, english, korean) in cases {
        assert_eq!(Lang::En.git_unavailable(reason), english);
        assert_eq!(Lang::Ko.git_unavailable(reason), korean);
    }
}

#[test]
fn every_file_state_is_spoken_on_both_sides_of_a_status_row() {
    let cases = [
        (
            FileState::Unmodified,
            "Staged: unchanged. Unstaged: unchanged.",
            "스테이지한 변경: 변경 없음. 스테이지하지 않은 변경: 변경 없음.",
        ),
        (
            FileState::Modified,
            "Staged: modified. Unstaged: modified.",
            "스테이지한 변경: 수정. 스테이지하지 않은 변경: 수정.",
        ),
        (
            FileState::Added,
            "Staged: added. Unstaged: added.",
            "스테이지한 변경: 추가. 스테이지하지 않은 변경: 추가.",
        ),
        (
            FileState::Deleted,
            "Staged: deleted. Unstaged: deleted.",
            "스테이지한 변경: 삭제. 스테이지하지 않은 변경: 삭제.",
        ),
        (
            FileState::Renamed,
            "Staged: renamed. Unstaged: renamed.",
            "스테이지한 변경: 이름 변경. 스테이지하지 않은 변경: 이름 변경.",
        ),
        (
            FileState::Copied,
            "Staged: copied. Unstaged: copied.",
            "스테이지한 변경: 복사. 스테이지하지 않은 변경: 복사.",
        ),
        (
            FileState::TypeChanged,
            "Staged: file type changed. Unstaged: file type changed.",
            "스테이지한 변경: 파일 형식 변경. 스테이지하지 않은 변경: 파일 형식 변경.",
        ),
        (
            FileState::Untracked,
            "Staged: not tracked. Unstaged: not tracked.",
            "스테이지한 변경: 추적하지 않음. 스테이지하지 않은 변경: 추적하지 않음.",
        ),
        (
            FileState::Conflicted,
            "Staged: conflicted. Unstaged: conflicted.",
            "스테이지한 변경: 충돌. 스테이지하지 않은 변경: 충돌.",
        ),
        (
            FileState::Unknown,
            "Staged: unrecognized change. Unstaged: unrecognized change.",
            "스테이지한 변경: 알 수 없는 변경. 스테이지하지 않은 변경: 알 수 없는 변경.",
        ),
    ];
    for (state, english, korean) in cases {
        assert_eq!(Lang::En.git_file_state(state, state), english);
        assert_eq!(Lang::Ko.git_file_state(state, state), korean);
    }
    assert_eq!(
        Lang::En.git_file_state(FileState::Modified, FileState::Untracked),
        "Staged: modified. Unstaged: not tracked."
    );
    assert_eq!(
        Lang::Ko.git_file_state(FileState::Modified, FileState::Untracked),
        "스테이지한 변경: 수정. 스테이지하지 않은 변경: 추적하지 않음."
    );
}

#[test]
fn refusal_details_preserve_the_paths_and_the_entire_hook_output() {
    let issue = Issue::Refused(GitRefusal::HookFailed {
        output: "first line\nsecond line\n".to_owned(),
    });
    assert_eq!(
        Lang::En.git_issue(&issue),
        GitFailureText {
            title: "A hook of the repository refused it.".to_owned(),
            next: "Read what the hook said, fix it, and try again.".to_owned(),
            detail: Some("first line\nsecond line\n".to_owned()),
        }
    );
    assert_eq!(
        Lang::Ko.git_issue(&issue),
        GitFailureText {
            title: "저장소의 훅이 거부했습니다.".to_owned(),
            next: "훅이 남긴 말을 읽고 고친 뒤 다시 시도하세요.".to_owned(),
            detail: Some("first line\nsecond line\n".to_owned()),
        }
    );
    let issue = Issue::Refused(GitRefusal::WouldOverwrite {
        paths: vec!["src/old.rs".parse().unwrap(), "src/new.rs".parse().unwrap()],
    });
    assert_eq!(
        Lang::En.git_issue(&issue),
        GitFailureText {
            title: "Local changes would be overwritten.".to_owned(),
            next: "Commit or stash them first.".to_owned(),
            detail: Some("src/old.rs\nsrc/new.rs".to_owned()),
        }
    );
    assert_eq!(
        Lang::Ko.git_issue(&issue),
        GitFailureText {
            title: "로컬 변경이 덮어써집니다.".to_owned(),
            next: "먼저 커밋하거나 스태시하세요.".to_owned(),
            detail: Some("src/old.rs\nsrc/new.rs".to_owned()),
        }
    );
}

#[test]
fn an_unrecognized_refusal_uses_app_words_instead_of_the_pages_instruction() {
    let issue = Issue::Refused(GitRefusal::Unknown);
    assert_eq!(
        Lang::En.git_issue(&issue),
        GitFailureText {
            title: "Git refused this action for a reason this app cannot read.".to_owned(),
            next: "Update Vessel on this machine and in the app, then try again.".to_owned(),
            detail: None,
        }
    );
    assert_eq!(
        Lang::Ko.git_issue(&issue),
        GitFailureText {
            title: "Git이 이 앱이 읽을 수 없는 이유로 동작을 거부했습니다.".to_owned(),
            next: "머신과 앱의 Vessel을 업데이트한 뒤 다시 시도하세요.".to_owned(),
            detail: None,
        }
    );
}

fn status() -> GitStatus {
    GitStatus {
        branch: Some("feature".parse().unwrap()),
        head: None,
        upstream: None,
        base: None,
        operation: None,
        files: vec![],
        more: false,
    }
}

#[test]
fn each_menu_place_has_its_own_label_and_first_push_says_what_it_sets() {
    let cases = [
        (Action::Quick(GitAction::Commit), "Commit…", "커밋…"),
        (
            Action::Quick(GitAction::Push),
            "Push and set upstream",
            "푸시하고 원격 추적 설정",
        ),
        (Action::Quick(GitAction::Pull), "Pull", "풀"),
        (Action::Quick(GitAction::Fetch), "Fetch", "페치"),
        (Action::SwitchBranch, "Switch branch…", "브랜치 전환…"),
        (Action::CreateBranch, "New branch…", "새 브랜치…"),
        (Action::RenameBranch, "Rename branch…", "브랜치 이름 변경…"),
        (
            Action::SaveStash(StashScope::Tracked),
            "Stash changes",
            "변경 임시 보관",
        ),
        (
            Action::SaveStash(StashScope::WithUntracked),
            "Stash, with untracked files",
            "추적하지 않는 파일까지 임시 보관",
        ),
        (Action::Stashes, "Stashes…", "임시 보관 목록…"),
    ];
    for (action, english, korean) in cases {
        assert_eq!(Lang::En.git_action_label(action, &status()), english);
        assert_eq!(Lang::Ko.git_action_label(action, &status()), korean);
    }
    let mut detached = status();
    detached.branch = None;
    assert_eq!(
        Lang::En.git_action_label(Action::Quick(GitAction::Push), &detached),
        "Push"
    );
    assert_eq!(
        Lang::Ko.git_action_label(Action::Quick(GitAction::Push), &detached),
        "푸시"
    );
}

#[test]
fn every_discard_effect_explains_the_loss_before_the_exact_file_is_confirmed() {
    let root = WorkspaceRoot::Project {
        project: "6f1c2a3e-0000-4000-8000-000000000001".parse().unwrap(),
    };
    for (index, worktree, english, korean) in [
        (
            FileState::Modified,
            FileState::Modified,
            "Staged and unstaged changes to this file will be lost. Git will restore the last commit. This cannot be undone.",
            "이 파일에서 스테이지한 변경과 스테이지하지 않은 변경이 사라지고 마지막 커밋으로 돌아갑니다. 되돌릴 수 없습니다.",
        ),
        (
            FileState::Added,
            FileState::Unmodified,
            "This new file will be deleted, including all of its contents. This cannot be undone.",
            "이 새 파일과 그 안의 모든 내용이 삭제됩니다. 되돌릴 수 없습니다.",
        ),
        (
            FileState::Untracked,
            FileState::Untracked,
            "This new file will be deleted, including all of its contents. This cannot be undone.",
            "이 새 파일과 그 안의 모든 내용이 삭제됩니다. 되돌릴 수 없습니다.",
        ),
    ] {
        let decision = DiscardDecision::of(
            root,
            &StatusEntry {
                path: "src/한글.rs".parse().unwrap(),
                old_path: None,
                index,
                worktree,
            },
        );
        assert_eq!(Lang::En.git_discard_copy(&decision), english);
        assert_eq!(Lang::Ko.git_discard_copy(&decision), korean);
    }
}
