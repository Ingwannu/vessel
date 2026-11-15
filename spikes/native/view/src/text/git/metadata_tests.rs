use crate::Lang;
use vessel_proto::workspace::{GitOperation, StashEntry};

#[test]
fn every_operation_is_a_sentence_with_a_next_step_in_both_languages() {
    for (operation, english, korean) in [
        (
            GitOperation::Merge,
            "The repository is in the middle of a merge. Finish or abort it before changing branches.",
            "저장소가 병합 도중입니다. 브랜치를 바꾸기 전에 끝내거나 중단하세요.",
        ),
        (
            GitOperation::Rebase,
            "The repository is in the middle of a rebase. Finish or abort it before changing branches.",
            "저장소가 리베이스 도중입니다. 브랜치를 바꾸기 전에 끝내거나 중단하세요.",
        ),
        (
            GitOperation::CherryPick,
            "The repository is in the middle of a cherry-pick. Finish or abort it before changing branches.",
            "저장소가 체리픽 도중입니다. 브랜치를 바꾸기 전에 끝내거나 중단하세요.",
        ),
        (
            GitOperation::Revert,
            "The repository is in the middle of a revert. Finish or abort it before changing branches.",
            "저장소가 되돌리기 도중입니다. 브랜치를 바꾸기 전에 끝내거나 중단하세요.",
        ),
        (
            GitOperation::Bisect,
            "The repository is in the middle of a bisect. Finish or abort it before changing branches.",
            "저장소가 이분 탐색 도중입니다. 브랜치를 바꾸기 전에 끝내거나 중단하세요.",
        ),
    ] {
        assert_eq!(Lang::En.git_operation_line(operation), english);
        assert_eq!(Lang::Ko.git_operation_line(operation), korean);
    }
}

#[test]
fn stash_identity_and_relative_and_viewer_local_time_have_separate_places() {
    let at = "2026-11-12T09:00:00Z".parse().unwrap();
    let entry = StashEntry {
        index: 7,
        commit: "1111111111111111111111111111111111111111".parse().unwrap(),
        time: at,
        message: "Saved edits".to_owned(),
    };
    let offset = chrono::FixedOffset::east_opt(9 * 3600).unwrap();
    let now = "2026-11-12T09:01:00Z".parse().unwrap();
    assert_eq!(
        Lang::En.git_stash_meta(&entry, now, offset),
        super::metadata::GitStashText {
            reference: "stash@{7}".to_owned(),
            saved: "Saved 1 minute ago".to_owned(),
            exact: "2026-11-12 18:00:00 (UTC+9)".to_owned(),
        }
    );
    assert_eq!(
        Lang::Ko.git_stash_meta(&entry, now, offset),
        super::metadata::GitStashText {
            reference: "stash@{7}".to_owned(),
            saved: "1분 전에 보관".to_owned(),
            exact: "2026-11-12 18:00:00 (UTC+9)".to_owned(),
        }
    );
}

#[test]
fn every_stash_age_is_a_whole_sentence_in_each_language() {
    use crate::devices::time::Age;
    for (age, en, ko) in [
        (Age::Now, "Saved just now", "방금 보관"),
        (Age::Minutes(1), "Saved 1 minute ago", "1분 전에 보관"),
        (Age::Minutes(2), "Saved 2 minutes ago", "2분 전에 보관"),
        (Age::Hours(1), "Saved 1 hour ago", "1시간 전에 보관"),
        (Age::Hours(2), "Saved 2 hours ago", "2시간 전에 보관"),
        (Age::Days(1), "Saved 1 day ago", "1일 전에 보관"),
        (Age::Days(2), "Saved 2 days ago", "2일 전에 보관"),
        (
            Age::Date("2026-11-12".parse().unwrap()),
            "Saved on Nov 12",
            "11월 12일에 보관",
        ),
    ] {
        assert_eq!(Lang::En.git_stash_saved(age), en);
        assert_eq!(Lang::Ko.git_stash_saved(age), ko);
    }
}

#[test]
fn the_default_branch_note_is_the_references_short_label() {
    assert_eq!(
        Lang::En.git_labels().commit_default_branch,
        "Default branch"
    );
    assert_eq!(Lang::Ko.git_labels().commit_default_branch, "기본 브랜치");
}

#[test]
fn primary_and_menu_commit_labels_have_separate_roles() {
    use crate::workspace::{
        git::{Action, fixtures},
        git_quick::GitAction,
    };
    let status = fixtures::status(vec![]);
    let action = Action::Quick(GitAction::Commit);
    assert_eq!(Lang::En.git_primary_label(action, &status), "Commit");
    assert_eq!(Lang::En.git_action_label(action, &status), "Commit…");
    assert_eq!(Lang::Ko.git_primary_label(action, &status), "커밋");
    assert_eq!(Lang::Ko.git_action_label(action, &status), "커밋…");
    let action = Action::Quick(GitAction::Push);
    assert_eq!(
        Lang::En.git_primary_label(action, &status),
        "Push and set upstream"
    );
    assert_eq!(
        Lang::Ko.git_primary_label(action, &status),
        "푸시하고 원격 추적 설정"
    );
}
