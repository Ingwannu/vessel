//! Repository paging through the actual fake provider's shared state.

use chrono::DateTime;
use vessel_proto::forge::{ForgeKind, RefusedReason};
use vessel_proto::workspace::CommitId;

use super::*;
use crate::fake::{Failure, pull};
use crate::provider::{Forge, Priority};

fn repo(path: &str) -> RepoRef {
    RepoRef {
        kind: ForgeKind::Github,
        host: "forge.example.invalid".parse().unwrap(),
        path: path.parse().unwrap(),
    }
}

fn insert(fake: &FakeForge, repo: &RepoRef, number: usize, state: PullState) {
    let number = PullNumber::new(u64::try_from(number).unwrap()).unwrap();
    let branch = format!("topic-{number}").parse().unwrap();
    let head: CommitId = "0123456789abcdef0123456789abcdef01234567".parse().unwrap();
    let mut read = pull(repo, number, &branch, &head).unwrap();
    read.pull.summary.updated_at = DateTime::<Utc>::UNIX_EPOCH;
    read.pull.summary.state = state;
    read.pull.checks.state = read.pull.summary.checks;
    fake.put(branch, read);
}

#[tokio::test]
async fn open_repository_pages_are_independent_of_branch_discovery_and_other_repositories() {
    const EXTRA: usize = 5;
    let fake = FakeForge::new();
    let owner = repo("fixture/project");
    let other = repo("other/repository");
    let count = OpenPullPage::MAX_PULLS.checked_add(EXTRA).unwrap();
    for number in 1..=count {
        insert(&fake, &owner, number, PullState::Open);
    }
    insert(
        &fake,
        &owner,
        count.checked_add(1).unwrap(),
        PullState::Closed,
    );
    insert(
        &fake,
        &owner,
        count.checked_add(2).unwrap(),
        PullState::Merged,
    );
    insert(&fake, &other, 1, PullState::Open);
    let first = fake
        .list_open(&owner, None, Priority::Person)
        .await
        .unwrap();
    assert_eq!(first.repo(), &owner);
    let numbers: Vec<_> = first.pulls().iter().map(|pull| pull.number.get()).collect();
    let expected: Vec<_> = (EXTRA.checked_add(1).unwrap()..=count)
        .rev()
        .map(|number| u64::try_from(number).unwrap())
        .collect();
    assert_eq!(numbers, expected);
    let next = first.continuation().unwrap();
    assert_eq!(next.repo, owner);
    let second = fake
        .list_open(&owner, Some(&next.after), Priority::Person)
        .await
        .unwrap();
    assert_eq!(
        second
            .pulls()
            .iter()
            .map(|pull| pull.number.get())
            .collect::<Vec<_>>(),
        (1..=EXTRA)
            .rev()
            .map(|number| u64::try_from(number).unwrap())
            .collect::<Vec<_>>()
    );
    assert_eq!(second.continuation(), None);
    assert_eq!(fake.calls("find"), 0);
    assert_eq!(fake.calls("read"), 0);
    assert_eq!(fake.calls("list_open"), 2);
}

#[tokio::test]
async fn an_empty_repository_is_success_but_explicit_provider_failures_are_not_empty_pages() {
    let fake = FakeForge::new();
    let owner = repo("fixture/project");
    let empty = fake
        .list_open(&owner, None, Priority::Person)
        .await
        .unwrap();
    assert!(empty.pulls().is_empty());
    assert_eq!(empty.continuation(), None);
    fake.fail_repository(owner.clone(), Some(Failure::NoAccess));
    assert!(matches!(
        fake.list_open(&owner, None, Priority::Person).await,
        Err(ForgeError::Refused {
            reason: RefusedReason::NoAccess
        })
    ));
    fake.fail_repository(owner.clone(), Some(Failure::RateLimited));
    assert!(matches!(
        fake.list_open(&owner, None, Priority::Person).await,
        Err(ForgeError::RateLimited { .. })
    ));
    fake.fail_repository(owner.clone(), Some(Failure::SignedOut));
    assert!(matches!(
        fake.list_open(&owner, None, Priority::Person).await,
        Err(ForgeError::SignedOut { .. })
    ));
    fake.fail_repository(owner.clone(), Some(Failure::Down));
    assert!(matches!(
        fake.list_open(&owner, None, Priority::Person).await,
        Err(ForgeError::Down)
    ));
}

#[tokio::test]
async fn malformed_or_gone_cursors_refuse_without_echoing_them() {
    let fake = FakeForge::new();
    let owner = repo("fixture/project");
    for text in [
        "opaque-private-metadata",
        "fixture:private-metadata",
        "fixture:0",
        "fixture:1",
    ] {
        let cursor: PullCursor = text.parse().unwrap();
        let error = fake
            .list_open(&owner, Some(&cursor), Priority::Person)
            .await
            .unwrap_err();
        assert!(!format!("{error:?}").contains(text));
        assert!(!error.to_string().contains(text));
    }
}
