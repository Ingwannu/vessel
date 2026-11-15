use super::header::GitHeaderText;
use crate::{
    Lang,
    workspace::git::{DistanceName, HeadActivity, fixtures},
};
use vessel_proto::workspace::{Distance, GitOperation};

#[test]
fn the_name_is_once_in_the_head_while_complete_facts_stay_in_its_tooltip() {
    let mut status = fixtures::status(vec![]);
    status.head = Some("1111111111111111111111111111111111111111".parse().unwrap());
    status.upstream = Some(Distance {
        name: "origin/feature".parse().unwrap(),
        ahead: 0,
        behind: 0,
    });
    status.base = Some(Distance {
        name: "feature".parse().unwrap(),
        ahead: 0,
        behind: 0,
    });
    assert_eq!(Lang::En.git_header(&status, HeadActivity::Idle), GitHeaderText {
        name: "feature".to_owned(), state: "Up to date".to_owned(), base: None, commit: None,
        tooltip: "On branch feature.\nUp to date with origin/feature.\nUp to date with feature.\n0 files changed; 0 files staged.".to_owned(),
    });
    assert_eq!(Lang::Ko.git_header(&status, HeadActivity::Idle), GitHeaderText {
        name: "feature".to_owned(), state: "최신 상태".to_owned(), base: None, commit: None,
        tooltip: "feature 브랜치입니다.\norigin/feature 기준으로 최신 상태입니다.\nfeature 기준으로 최신 상태입니다.\n파일 0개가 바뀌었고 0개가 스테이지되어 있습니다.".to_owned(),
    });
}

#[test]
fn short_distances_are_words_with_whole_compact_labels_in_both_languages() {
    for (ahead, behind, implicit_en, implicit_ko, explicit_en, explicit_ko) in [
        (
            0,
            0,
            "Up to date",
            "최신 상태",
            "Up to date with origin/main",
            "origin/main 기준 최신 상태",
        ),
        (
            1,
            0,
            "1 ahead",
            "1개 앞섬",
            "1 ahead of origin/main",
            "origin/main 기준 1개 앞섬",
        ),
        (
            0,
            1,
            "1 behind",
            "1개 뒤처짐",
            "1 behind origin/main",
            "origin/main 기준 1개 뒤처짐",
        ),
        (
            2,
            1,
            "2 ahead, 1 behind",
            "2개 앞섬, 1개 뒤처짐",
            "2 ahead, 1 behind origin/main",
            "origin/main 기준 2개 앞섬, 1개 뒤처짐",
        ),
    ] {
        let distance = Distance {
            name: "origin/main".parse().unwrap(),
            ahead,
            behind,
        };
        assert_eq!(
            Lang::En.git_head_distance(&distance, DistanceName::ImplicitUpstream),
            implicit_en
        );
        assert_eq!(
            Lang::Ko.git_head_distance(&distance, DistanceName::ImplicitUpstream),
            implicit_ko
        );
        assert_eq!(
            Lang::En.git_head_distance(&distance, DistanceName::Explicit),
            explicit_en
        );
        assert_eq!(
            Lang::Ko.git_head_distance(&distance, DistanceName::Explicit),
            explicit_ko
        );
    }
}

#[test]
fn operations_have_short_words_separate_from_their_complete_next_steps() {
    for (operation, en, ko) in [
        (GitOperation::Merge, "Merge in progress", "병합 중"),
        (GitOperation::Rebase, "Rebase in progress", "리베이스 중"),
        (
            GitOperation::CherryPick,
            "Cherry-pick in progress",
            "체리픽 중",
        ),
        (GitOperation::Revert, "Revert in progress", "되돌리기 중"),
        (GitOperation::Bisect, "Bisect in progress", "이분 탐색 중"),
    ] {
        assert_eq!(Lang::En.git_head_operation(operation), en);
        assert_eq!(Lang::Ko.git_head_operation(operation), ko);
    }
}

#[test]
fn activity_incomplete_detached_and_unborn_heads_are_readable() {
    let mut status = fixtures::status(vec![]);
    for (lang, activity, expected) in [
        (Lang::En, HeadActivity::Idle, "No commits yet"),
        (Lang::Ko, HeadActivity::Idle, "아직 커밋 없음"),
        (Lang::En, HeadActivity::Reading, "Refreshing…"),
        (Lang::Ko, HeadActivity::Reading, "새로 읽는 중…"),
        (Lang::En, HeadActivity::Writing, "Working…"),
        (Lang::Ko, HeadActivity::Writing, "작업 중…"),
    ] {
        assert_eq!(lang.git_header(&status, activity).state, expected);
    }
    status.head = Some("1111111111111111111111111111111111111111".parse().unwrap());
    assert_eq!(
        Lang::En.git_header(&status, HeadActivity::Idle).state,
        "No upstream set"
    );
    assert_eq!(
        Lang::Ko.git_header(&status, HeadActivity::Idle).state,
        "추적 브랜치 연결 없음"
    );
    status.branch = None;
    let en = Lang::En.git_header(&status, HeadActivity::Idle);
    assert_eq!(en.name, "Detached HEAD");
    assert_eq!(en.state, "At commit");
    let ko = Lang::Ko.git_header(&status, HeadActivity::Idle);
    assert_eq!(ko.name, "브랜치에서 분리됨");
    assert_eq!(ko.state, "현재 커밋");
    let commit = super::header::GitHashText {
        short: "111111111111".to_owned(),
        full: "1111111111111111111111111111111111111111".parse().unwrap(),
    };
    assert_eq!(en.commit, Some(commit.clone()));
    assert_eq!(ko.commit, Some(commit));
    status.more = true;
    assert_eq!(
        Lang::En.git_header(&status, HeadActivity::Idle).state,
        "Some changes are not listed"
    );
    assert_eq!(
        Lang::Ko.git_header(&status, HeadActivity::Idle).state,
        "목록에 없는 변경 있음"
    );
    status.head = None;
    assert_eq!(
        Lang::En.git_header(&status, HeadActivity::Idle).name,
        "No branch"
    );
    assert_eq!(
        Lang::Ko.git_header(&status, HeadActivity::Idle).name,
        "브랜치 없음"
    );
}

#[test]
fn the_commit_scope_names_say_which_changes_will_be_included() {
    let en = Lang::En.git_labels();
    assert_eq!(en.commit_scope, "Changes to commit");
    assert_eq!(en.staged, "Staged changes only");
    assert_eq!(en.tracked, "All tracked changes");
    assert_eq!(en.everything, "All changes, including new files");
    let ko = Lang::Ko.git_labels();
    assert_eq!(ko.commit_scope, "커밋할 변경");
    assert_eq!(ko.staged, "스테이징한 변경만");
    assert_eq!(ko.tracked, "추적 중인 파일의 모든 변경");
    assert_eq!(ko.everything, "새 파일을 포함한 모든 변경");
}

#[test]
fn an_unborn_checkout_is_not_described_as_a_detached_commit() {
    let mut status = fixtures::status(vec![]);
    status.branch = None;
    assert_eq!(Lang::En.git_header(&status, HeadActivity::Idle), GitHeaderText {
        name: "No branch".to_owned(), state: "No commits yet".to_owned(),
        commit: None, base: None,
        tooltip: "No branch is checked out.\nThe repository has no commits yet.\n0 files changed; 0 files staged.".to_owned(),
    });
    assert_eq!(Lang::Ko.git_header(&status, HeadActivity::Idle), GitHeaderText {
        name: "브랜치 없음".to_owned(), state: "아직 커밋 없음".to_owned(),
        commit: None, base: None,
        tooltip: "체크아웃된 브랜치가 없습니다.\n저장소에 아직 커밋이 없습니다.\n파일 0개가 바뀌었고 0개가 스테이지되어 있습니다.".to_owned(),
    });
}
