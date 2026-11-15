//! Complete branch and count sentences in both languages.

use crate::Lang;
use vessel_proto::workspace::Distance;

#[test]
fn upstream_counts_are_complete_sentences_including_zero_singular_and_diverged() {
    for (ahead, behind, en, ko) in [
        (
            0,
            0,
            "Up to date with origin/feature.",
            "origin/feature 기준으로 최신 상태입니다.",
        ),
        (
            1,
            0,
            "1 commit ahead of origin/feature.",
            "origin/feature보다 커밋 1개 앞서 있습니다.",
        ),
        (
            0,
            1,
            "1 commit behind origin/feature.",
            "origin/feature보다 커밋 1개 뒤처져 있습니다.",
        ),
        (
            2,
            1,
            "2 commits ahead of origin/feature and 1 commit behind it.",
            "origin/feature보다 커밋 2개 앞서고 1개 뒤처져 있습니다.",
        ),
    ] {
        let distance = Distance {
            name: "origin/feature".parse().unwrap(),
            ahead,
            behind,
        };
        assert_eq!(Lang::En.git_distance(&distance), en);
        assert_eq!(Lang::Ko.git_distance(&distance), ko);
    }
}

#[test]
fn branch_sentences_distinguish_unborn_detached_and_normal_checkouts() {
    let mut status = crate::workspace::git::fixtures::status(vec![]);
    assert_eq!(
        Lang::En.git_branch_state(&status),
        "On branch feature. No commits yet."
    );
    assert_eq!(
        Lang::Ko.git_branch_state(&status),
        "feature 브랜치입니다. 아직 커밋이 없습니다."
    );
    status.head = Some("0123456789abcdef0123456789abcdef01234567".parse().unwrap());
    assert_eq!(Lang::En.git_branch_state(&status), "On branch feature.");
    assert_eq!(Lang::Ko.git_branch_state(&status), "feature 브랜치입니다.");
    status.branch = None;
    assert_eq!(
        Lang::En.git_branch_state(&status),
        "No branch is checked out. At commit 0123456."
    );
    assert_eq!(
        Lang::Ko.git_branch_state(&status),
        "체크아웃된 브랜치가 없습니다. 현재 커밋: 0123456."
    );
    status.head = None;
    assert_eq!(
        Lang::En.git_branch_state(&status),
        "No branch is checked out. No commits yet."
    );
    assert_eq!(
        Lang::Ko.git_branch_state(&status),
        "체크아웃된 브랜치가 없습니다. 아직 커밋이 없습니다."
    );
}

#[test]
fn truncated_status_counts_do_not_claim_to_be_the_total() {
    use vessel_proto::workspace::{FileState, StatusEntry};
    let file = StatusEntry {
        path: "a".parse().unwrap(),
        old_path: None,
        index: FileState::Added,
        worktree: FileState::Modified,
    };
    let mut status = crate::workspace::git::fixtures::status(vec![file]);
    assert_eq!(
        Lang::En.git_change_counts(&status),
        "1 file changed; 1 file staged."
    );
    assert_eq!(
        Lang::Ko.git_change_counts(&status),
        "파일 1개가 바뀌었고 1개가 스테이지되어 있습니다."
    );
    status.more = true;
    assert_eq!(
        Lang::En.git_change_counts(&status),
        "At least 1 file changed; 1 of the listed files staged."
    );
    assert_eq!(
        Lang::Ko.git_change_counts(&status),
        "최소 1개 파일이 바뀌었고, 표시된 파일 중 1개가 스테이지되어 있습니다."
    );
}

#[test]
fn empty_and_large_counts_keep_their_units_and_group_their_digits() {
    let mut status = crate::workspace::git::fixtures::status(vec![]);
    assert_eq!(
        Lang::En.git_change_counts(&status),
        "0 files changed; 0 files staged."
    );
    assert_eq!(
        Lang::Ko.git_change_counts(&status),
        "파일 0개가 바뀌었고 0개가 스테이지되어 있습니다."
    );
    let distance = Distance {
        name: "origin/feature".parse().unwrap(),
        ahead: u32::MAX,
        behind: 1,
    };
    assert_eq!(
        Lang::En.git_distance(&distance),
        "4,294,967,295 commits ahead of origin/feature and 1 commit behind it."
    );
    assert_eq!(
        Lang::Ko.git_distance(&distance),
        "origin/feature보다 커밋 4,294,967,295개 앞서고 1개 뒤처져 있습니다."
    );
    status.more = true;
    assert_eq!(
        Lang::En.git_change_counts(&status),
        "The change list is incomplete. No files were listed."
    );
    assert_eq!(
        Lang::Ko.git_change_counts(&status),
        "변경 목록이 완전하지 않습니다. 표시된 파일이 없습니다."
    );
}
