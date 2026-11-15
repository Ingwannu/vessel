//! Scope/submit share the node's add --update rule; missing paths in a
//! bounded reply remain unknown rather than evidence that a scope is empty.
use super::*;
use crate::{Lang, workspace::git_quick::GitAction};
use vessel_proto::workspace::{CommitStage, FileState, StatusEntry};

fn file(path: &str, index: FileState, worktree: FileState) -> StatusEntry {
    StatusEntry {
        path: path.parse().unwrap(),
        old_path: None,
        index,
        worktree,
    }
}

#[test]
fn complete_and_partial_scope_eligibility_is_identical_in_choices_and_submit() {
    let new = file("new.rs", FileState::Unmodified, FileState::Untracked);
    let working = file("tracked.rs", FileState::Unmodified, FileState::Modified);
    let staged = file("staged.rs", FileState::Added, FileState::Unmodified);
    for (files, more, counts, enabled) in [
        (vec![], false, [0, 0, 0], [false, false, false]),
        (vec![new.clone()], false, [0, 0, 1], [false, false, true]),
        (
            vec![file(
                "new-pair.rs",
                FileState::Untracked,
                FileState::Untracked,
            )],
            false,
            [0, 0, 1],
            [false, false, true],
        ),
        (vec![working.clone()], false, [0, 1, 1], [false, true, true]),
        (vec![staged.clone()], false, [1, 1, 1], [true, true, true]),
        (
            vec![new.clone(), working, staged],
            false,
            [1, 2, 3],
            [true, true, true],
        ),
        (vec![new], true, [0, 0, 1], [true, true, true]),
        (vec![], true, [0, 0, 0], [true, true, true]),
    ] {
        let mut status = fixtures::status(files);
        status.more = more;
        let form = CommitForm::of(&status);
        for stage in [
            CommitStage::Staged,
            CommitStage::Tracked,
            CommitStage::Everything,
        ] {
            assert_eq!(
                commit_files(&status, stage).count(),
                form.eligible(stage).get()
            );
        }
        assert_eq!(
            form.coverage,
            if more {
                StatusCoverage::Partial
            } else {
                StatusCoverage::Complete
            }
        );
        let read = Reading::Ready(status);
        for (((stage, count), enabled), choice) in [
            CommitStage::Staged,
            CommitStage::Tracked,
            CommitStage::Everything,
        ]
        .into_iter()
        .zip(counts)
        .zip(enabled)
        .zip(commit_scopes(&read, &Writing::default()))
        {
            assert_eq!(form.eligible(stage).get(), count);
            assert_eq!(form.has_changes(stage), enabled);
            assert_eq!(choice, CommitScope { stage, enabled });
            assert_eq!(
                can_commit(&read, &Writing::default(), "Fixture commit", stage),
                enabled
            );
        }
    }
}

#[test]
fn complete_untracked_only_gates_do_not_change_the_three_whole_scope_labels() {
    let read = Reading::Ready(fixtures::status(vec![file(
        "new.rs",
        FileState::Unmodified,
        FileState::Untracked,
    )]));
    for (lang, expected) in [
        (
            Lang::En,
            [
                "Staged changes only",
                "All tracked changes",
                "All changes, including new files",
            ],
        ),
        (
            Lang::Ko,
            [
                "스테이징한 변경만",
                "추적 중인 파일의 모든 변경",
                "새 파일을 포함한 모든 변경",
            ],
        ),
    ] {
        let scopes = commit_scopes(&read, &Writing::default());
        for (scope, label) in scopes.into_iter().zip(expected) {
            assert_eq!(lang.git_commit_scope(scope.stage), label);
        }
        assert_eq!(scopes.map(|scope| scope.enabled), [false, false, true]);
    }
}

#[test]
fn failed_and_missing_reads_do_not_claim_refresh_and_failed_reads_do_not_poll() {
    let mut keys = RequestKeys::default();
    let mut loading = Reading::Empty;
    loading.start(keys.issue().unwrap());
    for (read, reason, polling, en, ko) in [
        (
            Reading::Empty,
            Unavailable::StatusMissing,
            true,
            "Repository status has not been read yet. Read it before continuing.",
            "저장소 상태를 아직 읽지 않았습니다. 상태를 읽은 뒤 계속하세요.",
        ),
        (
            Reading::Failed(crate::workspace::problem::Problem::Older),
            Unavailable::StatusFailed,
            false,
            "Repository status could not be read. Refresh it before continuing.",
            "저장소 상태를 읽지 못했습니다. 상태를 새로 읽은 뒤 계속하세요.",
        ),
        (
            loading,
            Unavailable::Reading,
            false,
            "Repository status is being refreshed. Wait for it before continuing.",
            "저장소 상태를 새로 읽고 있습니다. 끝난 뒤 계속하세요.",
        ),
    ] {
        assert_eq!(
            availability(Action::Quick(GitAction::Commit), &read, &Writing::default()),
            Some(reason)
        );
        assert_eq!(
            should_refresh(&read, &Writing::default(), PanelVisibility::Shown),
            polling
        );
        assert_eq!(Lang::En.git_unavailable(reason), en);
        assert_eq!(Lang::Ko.git_unavailable(reason), ko);
        assert!(!can_commit(
            &read,
            &Writing::default(),
            "Commit",
            CommitStage::Everything
        ));
        assert_eq!(
            commit_scopes(&read, &Writing::default()).map(|scope| scope.enabled),
            [false; 3]
        );
    }
}
