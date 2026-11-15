//! Seed tests call the actual `FakeForge` trait operations, not a mirror.
use super::*;
use crate::{Forge as _, ForgeError, Fresh, Merged, Opened, Opening, Priority};
use vessel_proto::forge::{
    CheckRun, CheckState, ChecksState, PullNumber, PullState, RefusedReason,
};

fn repo() -> RepoRef {
    RepoRef {
        kind: ForgeKind::Github,
        host: "forge.example.invalid".parse().unwrap(),
        path: "fixture/project".parse().unwrap(),
    }
}
fn document() -> Raw {
    let repo = repo();
    let branch = "feature/example".parse().unwrap();
    let head = "a".repeat(40).parse().unwrap();
    let mut read =
        super::super::pull(&repo, PullNumber::new(207).unwrap(), &branch, &head).unwrap();
    read.pull.summary.title = "변경 사항과 실패한 검사".parse().unwrap();
    read.pull.checks.state = ChecksState::Failing;
    read.pull.summary.checks = ChecksState::Failing;
    read.pull.checks.failed = 1;
    read.pull.checks.failing = vec![CheckRun {
        name: "fixture check".parse().unwrap(),
        state: CheckState::Failed,
        url: None,
    }];
    Raw {
        version: Version,
        repositories: vec![Repository {
            repo,
            failure: None,
            pulls: vec![Pull {
                pull: read.pull,
                merge: Merge::Refused,
            }],
        }],
    }
}
fn decode(raw: &Raw) -> Seed {
    Seed::decode(&serde_json::to_vec(raw).unwrap()).unwrap()
}

#[tokio::test]
async fn seeded_facts_conditionals_and_refused_merge_use_real_fake_state() {
    let raw = document();
    let expected = raw.repositories[0].pulls[0].pull.clone();
    let forge = decode(&raw).into_forge();
    let repo = repo();
    let read = forge
        .read(&repo, expected.summary.number, Priority::Person)
        .await
        .unwrap();
    assert_eq!(read.pull.summary, expected.summary);
    assert_eq!(read.pull.checks, expected.checks);
    assert_eq!(read.pull.reviews, expected.reviews);
    let found = forge
        .find(&repo, &expected.head_branch, Priority::Background)
        .await
        .unwrap();
    assert!(
        matches!(found, Fresh::Changed(entries) if entries.len()==1 && entries[0].number==expected.summary.number)
    );
    assert_eq!(
        forge
            .find(&repo, &expected.head_branch, Priority::Background)
            .await
            .unwrap(),
        Fresh::Same
    );
    assert!(matches!(
        forge
            .find(&repo, &expected.head_branch, Priority::Person)
            .await
            .unwrap(),
        Fresh::Changed(_)
    ));
    assert_eq!(
        forge
            .merge(
                &repo,
                expected.summary.number,
                MergeMethod::Squash,
                &expected.summary.head_commit
            )
            .await
            .unwrap(),
        Merged::Rejected("the repository refused this merge".to_owned())
    );
    assert_eq!(
        forge.get(&repo, expected.summary.number).unwrap().pull,
        expected
    );
    let wrong = "b".repeat(40).parse().unwrap();
    assert_eq!(
        forge
            .merge(&repo, expected.summary.number, MergeMethod::Squash, &wrong)
            .await
            .unwrap(),
        Merged::HeadMoved
    );
    assert_eq!(
        forge
            .get(&repo, expected.summary.number)
            .unwrap()
            .pull
            .summary
            .state,
        PullState::Open
    );
}

#[tokio::test]
async fn failures_are_per_repository_and_never_fall_through_to_http() {
    for failure in [
        Failure::NoAccess,
        Failure::RateLimited,
        Failure::Down,
        Failure::SignedOut,
    ] {
        let mut raw = document();
        raw.repositories[0].failure = Some(failure);
        let mut healthy = document().repositories.remove(0);
        healthy.repo.path = "fixture/healthy".parse().unwrap();
        healthy.pulls[0].pull.summary.repo = healthy.repo.clone();
        let healthy_repo = healthy.repo.clone();
        let healthy_number = healthy.pulls[0].pull.summary.number;
        raw.repositories.push(healthy);
        let expected = raw.repositories[0].pulls[0].pull.clone();
        let forge = decode(&raw).into_forge();
        assert!(forge.whoami(&repo().host).await.unwrap().is_some());
        assert!(
            forge
                .read(&healthy_repo, healthy_number, Priority::Person)
                .await
                .is_ok()
        );
        let error = forge
            .read(&repo(), expected.summary.number, Priority::Person)
            .await
            .unwrap_err();
        match (failure, error) {
            (
                Failure::NoAccess,
                ForgeError::Refused {
                    reason: RefusedReason::NoAccess,
                },
            )
            | (Failure::RateLimited, ForgeError::RateLimited { .. })
            | (Failure::Down, ForgeError::Down)
            | (Failure::SignedOut, ForgeError::SignedOut { .. }) => {}
            pair => panic!("wrong failure {pair:?}"),
        }
        let unknown = RepoRef {
            path: "fixture/unknown".parse().unwrap(),
            ..repo()
        };
        assert!(matches!(
            forge
                .read(&unknown, expected.summary.number, Priority::Person)
                .await,
            Err(ForgeError::NotFound)
        ));
    }
}

#[tokio::test]
async fn opening_reserves_seeded_numbers_and_capacity_without_overwrite() {
    let raw = document();
    let preserved = raw.repositories[0].pulls[0].pull.clone();
    let forge = decode(&raw).into_forge();
    let opening = Opening {
        head: "new-branch".parse().unwrap(),
        base: "main".parse().unwrap(),
        title: "new pull".parse().unwrap(),
        body: "fixture body".parse().unwrap(),
        draft: false,
    };
    assert_eq!(
        forge.open(&repo(), &opening).await.unwrap(),
        Opened::Created(PullNumber::new(208).unwrap())
    );
    assert_eq!(
        forge.get(&repo(), preserved.summary.number).unwrap().pull,
        preserved
    );
    for number in 1..=MAX_PULLS {
        let mut read = super::super::pull(
            &repo(),
            PullNumber::new(u64::try_from(number).unwrap()).unwrap(),
            &opening.head,
            &preserved.summary.head_commit,
        )
        .unwrap();
        read.pull.summary.state = PullState::Closed;
        forge.put(opening.head.clone(), read);
    }
    assert_eq!(forge.open(&repo(), &opening).await.unwrap(), Opened::Exists);
    let other = Opening {
        head: "another-new-branch".parse().unwrap(),
        ..opening
    };
    assert_eq!(
        forge.open(&repo(), &other).await.unwrap(),
        Opened::Rejected("the pull request limit has been reached".to_owned())
    );
    assert_eq!(
        forge.get(&repo(), preserved.summary.number).unwrap().pull,
        preserved
    );
}

#[test]
fn whole_file_rules_and_diagnostics_do_not_expose_values() {
    let secret = "PRIVATE_FIXTURE_SENTINEL";
    let invalid = format!("{{\"{secret}\":true}}");
    let error = Seed::decode(invalid.as_bytes()).unwrap_err();
    assert!(!error.to_string().contains(secret));
    assert!(std::error::Error::source(&error).is_some());
    let bytes = vec![b' '; usize::try_from(MAX_BYTES + 1).unwrap()];
    assert!(matches!(Seed::decode(&bytes), Err(SeedError::TooLarge)));
    let mut raw = document();
    raw.repositories.push(document().repositories.remove(0));
    assert!(matches!(
        Seed::decode(&serde_json::to_vec(&raw).unwrap()),
        Err(SeedError::Rule(SeedRule::Repository))
    ));
    let mut raw = document();
    let duplicate = raw.repositories[0].pulls[0].pull.clone();
    raw.repositories[0].pulls.push(Pull {
        pull: duplicate,
        merge: Merge::Normal,
    });
    assert!(matches!(
        Seed::decode(&serde_json::to_vec(&raw).unwrap()),
        Err(SeedError::Rule(SeedRule::Pull))
    ));
    let mut raw = document();
    raw.repositories[0].pulls[0].pull.summary.checks = ChecksState::Passing;
    assert!(matches!(
        Seed::decode(&serde_json::to_vec(&raw).unwrap()),
        Err(SeedError::Rule(SeedRule::Checks))
    ));
    let empty_v2 = b"{\"version\":2,\"repositories\":[]}";
    assert!(matches!(Seed::decode(empty_v2), Err(SeedError::Format(_))));
}

#[test]
fn file_loading_rejects_unsafe_kind_and_oversize_before_decoding() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("seed.json");
    std::fs::write(&path, serde_json::to_vec(&document()).unwrap()).unwrap();
    assert!(Seed::load(&path).is_ok());
    assert!(matches!(
        Seed::load(temp.path()),
        Err(SeedError::NotRegular)
    ));
    let file = std::fs::File::create(&path).unwrap();
    file.set_len(MAX_BYTES + 1).unwrap();
    assert!(matches!(Seed::load(&path), Err(SeedError::TooLarge)));
    #[cfg(unix)]
    {
        let link = temp.path().join("link.json");
        std::os::unix::fs::symlink(&path, &link).unwrap();
        assert!(matches!(Seed::load(&link), Err(SeedError::NotRegular)));
    }
}

#[tokio::test]
async fn fixture_caches_and_merge_history_remain_bounded() {
    let raw = document();
    let pull = raw.repositories[0].pulls[0].pull.clone();
    let forge = decode(&raw).into_forge();
    for i in 0..MAX_PULLS.saturating_add(5) {
        let branch = format!("missing-{i}").parse().unwrap();
        assert_eq!(
            forge
                .find(&repo(), &branch, Priority::Background)
                .await
                .unwrap(),
            Fresh::Changed(vec![])
        );
        assert!(
            forge
                .changed(
                    &repo(),
                    PullNumber::new(u64::try_from(i).unwrap() + 1000).unwrap(),
                    &pull.summary.head_commit,
                    Priority::Background
                )
                .await
                .unwrap()
        );
        assert!(matches!(
            forge
                .merge(
                    &repo(),
                    pull.summary.number,
                    MergeMethod::Merge,
                    &pull.summary.head_commit
                )
                .await
                .unwrap(),
            Merged::Rejected(_)
        ));
    }
    assert!(forge.state().found.is_empty());
    assert!(forge.state().seen.is_empty());
    assert_eq!(forge.merges().len(), MAX_PULLS);
    assert_eq!(forge.get(&repo(), pull.summary.number).unwrap().pull, pull);
}

#[tokio::test]
async fn a_seeded_highest_number_cannot_overflow_or_overwrite() {
    let mut raw = document();
    raw.repositories[0].pulls[0].pull.summary.number = PullNumber::new(PullNumber::MAX).unwrap();
    let old = raw.repositories[0].pulls[0].pull.clone();
    let forge = decode(&raw).into_forge();
    let opening = Opening {
        head: "new-branch".parse().unwrap(),
        base: "main".parse().unwrap(),
        title: "new".parse().unwrap(),
        body: "fixture".parse().unwrap(),
        draft: false,
    };
    assert!(matches!(
        forge.open(&repo(), &opening).await,
        Err(ForgeError::Unexpected {
            what: "a new pull request number"
        })
    ));
    assert_eq!(forge.get(&repo(), old.summary.number).unwrap().pull, old);
}

#[test]
fn complete_seed_bounds_reject_excess_without_partial_loading() {
    let mut raw = document();
    raw.repositories.clear();
    for n in 0..MAX_REPOSITORIES.saturating_add(1) {
        let mut entry = document().repositories.remove(0);
        entry.repo.path = format!("fixture/repo-{n}").parse().unwrap();
        entry.pulls.clear();
        raw.repositories.push(entry);
    }
    assert!(matches!(
        Seed::decode(&serde_json::to_vec(&raw).unwrap()),
        Err(SeedError::Rule(SeedRule::Count))
    ));
    let mut raw = document();
    raw.repositories[0].pulls.clear();
    for n in 1..=MAX_PULLS.saturating_add(1) {
        let mut item = document().repositories.remove(0).pulls.remove(0);
        item.pull.summary.number = PullNumber::new(u64::try_from(n).unwrap()).unwrap();
        raw.repositories[0].pulls.push(item);
    }
    assert!(matches!(
        Seed::decode(&serde_json::to_vec(&raw).unwrap()),
        Err(SeedError::Rule(SeedRule::Count))
    ));
    let mut raw = document();
    let run = raw.repositories[0].pulls[0].pull.checks.failing[0].clone();
    raw.repositories[0].pulls[0].pull.checks.failing = vec![run; Checks::MAX_FAILING + 1];
    assert!(matches!(
        Seed::decode(&serde_json::to_vec(&raw).unwrap()),
        Err(SeedError::Rule(SeedRule::Checks))
    ));
    let mut raw = document();
    raw.repositories[0].pulls[0].pull.merge_methods = vec![MergeMethod::Merge, MergeMethod::Merge];
    assert!(matches!(
        Seed::decode(&serde_json::to_vec(&raw).unwrap()),
        Err(SeedError::Rule(SeedRule::Methods))
    ));
    let mut raw = document();
    raw.repositories[0].pulls[0].pull.summary.repo.path = "fixture/other".parse().unwrap();
    assert!(matches!(
        Seed::decode(&serde_json::to_vec(&raw).unwrap()),
        Err(SeedError::Rule(SeedRule::Pull))
    ));
}

mod concurrency;
