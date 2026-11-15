use crate::Lang;
use crate::workspace::git::{Success, UpstreamChange};
use vessel_proto::workspace::{CommitInfo, StashList};

fn receipt(
    outcome: Success,
    intent: crate::workspace::git::WriteIntent,
) -> crate::workspace::git::SuccessReceipt {
    use crate::workspace::git::{RequestKeys, Writing, Written};
    let key = RequestKeys::default().issue().unwrap();
    let mut writing = Writing::default();
    assert!(writing.begin_action(key, intent));
    assert_eq!(
        writing.finish(key, Ok(Written::Said(outcome.clone()))),
        Some(Ok(Written::Said(outcome)))
    );
    writing.success().unwrap().clone()
}

#[test]
fn successful_outcomes_name_their_facts_in_complete_notices() {
    let old: vessel_proto::workspace::CommitId =
        "1111111111111111111111111111111111111111".parse().unwrap();
    let new: vessel_proto::workspace::CommitId =
        "2222222222222222222222222222222222222222".parse().unwrap();
    let cases = [
        (
            Success::Committed(CommitInfo {
                id: old.clone(),
                parents: vec![],
                author: "Fixture Author".to_owned(),
                time: "2026-11-12T09:00:00Z".parse().unwrap(),
                subject: "Render the files".to_owned(),
            }),
            "Created commit: Render the files",
            "커밋을 만들었습니다: Render the files",
        ),
        (
            Success::Fetched,
            "Fetched the remote's latest changes.",
            "원격 저장소의 최신 변경을 가져왔습니다.",
        ),
        (
            Success::Pushed {
                remote: "origin".parse().unwrap(),
                branch: "feature".parse().unwrap(),
                upstream: UpstreamChange::Kept,
            },
            "Pushed feature to origin.",
            "origin의 feature 브랜치에 푸시했습니다.",
        ),
        (
            Success::Pushed {
                remote: "origin".parse().unwrap(),
                branch: "feature".parse().unwrap(),
                upstream: UpstreamChange::Set,
            },
            "Pushed feature to origin. The upstream is now set.",
            "origin의 feature 브랜치에 푸시하고 원격 추적 브랜치를 설정했습니다.",
        ),
        (
            Success::Pulled { from: old, to: new },
            "Pulled the latest changes.",
            "최신 변경을 풀했습니다.",
        ),
        (
            Success::UpToDate,
            "The branch is already up to date.",
            "이미 최신 상태인 브랜치입니다.",
        ),
        (
            Success::Switched("feature".parse().unwrap()),
            "Switched to feature.",
            "feature 브랜치로 전환했습니다.",
        ),
        (
            Success::Renamed("feature".parse().unwrap()),
            "Renamed the branch to feature.",
            "브랜치 이름을 변경했습니다. 새 이름: feature.",
        ),
        (
            Success::Stashed(StashList {
                entries: vec![],
                more: false,
            }),
            "Git completed the request.",
            "Git 요청을 완료했습니다.",
        ),
    ];
    for (success, english, korean) in cases {
        assert_eq!(success.alert_variant(), crate::alert::Variant::Success);
        assert_eq!(
            Lang::En
                .git_success(&receipt(
                    success.clone(),
                    crate::workspace::git::WriteIntent::Unspecified
                ))
                .message,
            english
        );
        assert_eq!(
            Lang::Ko
                .git_success(&receipt(
                    success.clone(),
                    crate::workspace::git::WriteIntent::Unspecified
                ))
                .message,
            korean
        );
    }
}

#[test]
fn dropping_a_stash_has_a_whole_warning_separate_from_its_code_identity() {
    assert_eq!(
        Lang::En.git_drop_copy(),
        "This saved entry will be deleted without applying it. This cannot be undone."
    );
    assert_eq!(
        Lang::Ko.git_drop_copy(),
        "이 보관 항목을 적용하지 않고 삭제합니다. 되돌릴 수 없습니다."
    );
}

#[test]
fn a_created_commit_and_both_pull_endpoints_remain_exact_separate_code_values() {
    use crate::{GitHashText, GitSuccessCommits, GitSuccessText};
    let first: vessel_proto::workspace::CommitId =
        "0123456789abcdef0123456789abcdef01234567".parse().unwrap();
    let next: vessel_proto::workspace::CommitId =
        "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789"
            .parse()
            .unwrap();
    let created = Success::Committed(CommitInfo {
        id: first.clone(),
        parents: vec![],
        author: "Author".into(),
        time: "2026-11-13T00:00:00Z".parse().unwrap(),
        subject: "Keep the facts".into(),
    });
    let first_hash = GitHashText {
        short: "0123456789ab".into(),
        full: first.clone(),
    };
    for (lang, message) in [
        (Lang::En, "Created commit: Keep the facts"),
        (Lang::Ko, "커밋을 만들었습니다: Keep the facts"),
    ] {
        assert_eq!(
            lang.git_success(&receipt(
                created.clone(),
                crate::workspace::git::WriteIntent::Unspecified
            )),
            GitSuccessText {
                message: message.into(),
                commits: GitSuccessCommits::Created(first_hash.clone()),
            }
        );
    }
    let pulled = Success::Pulled {
        from: first,
        to: next.clone(),
    };
    for (lang, message) in [
        (Lang::En, "Pulled the latest changes."),
        (Lang::Ko, "최신 변경을 풀했습니다."),
    ] {
        assert_eq!(
            lang.git_success(&receipt(
                pulled.clone(),
                crate::workspace::git::WriteIntent::Unspecified
            )),
            GitSuccessText {
                message: message.into(),
                commits: GitSuccessCommits::Pulled {
                    from: first_hash.clone(),
                    to: GitHashText {
                        short: "abcdef012345".into(),
                        full: next.clone()
                    },
                },
            }
        );
        assert_eq!(
            lang.git_success(&receipt(
                Success::Fetched,
                crate::workspace::git::WriteIntent::Unspecified,
            ))
            .commits,
            GitSuccessCommits::None
        );
    }
}

#[test]
fn stash_success_names_the_completed_action_without_guessing_from_the_new_list() {
    use crate::workspace::git::{StashChoice, StashScope, WriteIntent};
    use crate::{GitHashText, GitSuccessCommits, GitSuccessText};
    let entry = vessel_proto::workspace::StashEntry {
        index: 7,
        message: "Saved edits".into(),
        commit: "0123456789abcdef0123456789abcdef01234567".parse().unwrap(),
        time: "2026-11-13T00:00:00Z".parse().unwrap(),
    };
    let identity = GitSuccessCommits::Stash {
        reference: "stash@{7}".into(),
        commit: GitHashText {
            short: "0123456789ab".into(),
            full: entry.commit.clone(),
        },
    };
    let unchanged = StashList {
        entries: vec![entry.clone()],
        more: false,
    };
    let empty = StashList {
        entries: vec![],
        more: false,
    };
    for (intent, list, en, ko, commits) in [
        (
            WriteIntent::SaveStash(StashScope::Tracked),
            unchanged.clone(),
            "Saved changes to tracked files.",
            "추적 중인 파일의 변경을 임시 보관했습니다.",
            GitSuccessCommits::None,
        ),
        (
            WriteIntent::SaveStash(StashScope::WithUntracked),
            unchanged.clone(),
            "Saved changes, including new files.",
            "새 파일을 포함한 변경을 임시 보관했습니다.",
            GitSuccessCommits::None,
        ),
        (
            WriteIntent::Stash {
                choice: StashChoice::Apply,
                entry: entry.clone(),
            },
            unchanged,
            "Applied the saved changes. The saved entry was kept.",
            "보관한 변경을 적용하고 보관 항목은 남겼습니다.",
            identity.clone(),
        ),
        (
            WriteIntent::Stash {
                choice: StashChoice::Pop,
                entry: entry.clone(),
            },
            empty.clone(),
            "Applied the saved changes and removed the saved entry.",
            "보관한 변경을 적용하고 보관 항목을 삭제했습니다.",
            identity.clone(),
        ),
        (
            WriteIntent::Stash {
                choice: StashChoice::Drop,
                entry,
            },
            empty,
            "Deleted the saved entry without applying it.",
            "보관 항목을 적용하지 않고 삭제했습니다.",
            identity,
        ),
    ] {
        let success = Success::Stashed(list);
        for (lang, message) in [(Lang::En, en), (Lang::Ko, ko)] {
            assert_eq!(
                lang.git_success(&receipt(success.clone(), intent.clone())),
                GitSuccessText {
                    message: message.into(),
                    commits: commits.clone(),
                }
            );
        }
    }
}
