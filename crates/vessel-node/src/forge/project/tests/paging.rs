//! End-to-end repository pagination through the staged actor and real fake.
use super::*;
use vessel_proto::forge::{PullNumber, PullState};

pub(super) fn put(fixture: &Fixture, repository: &RepoRef, number: usize, state: PullState) {
    let number = PullNumber::new(u64::try_from(number).unwrap()).unwrap();
    let branch = format!("topic-{number}").parse().unwrap();
    let head = "0123456789abcdef0123456789abcdef01234567".parse().unwrap();
    let mut read = vessel_forge::fake::pull(repository, number, &branch, &head).unwrap();
    read.pull.summary.state = state;
    read.pull.summary.updated_at = chrono::DateTime::<chrono::Utc>::UNIX_EPOCH;
    fixture.fake.put(branch, read);
}

#[tokio::test]
async fn pages_have_real_continuations_and_do_not_include_closed_or_foreign_pulls() {
    let fixture = Fixture::new().await;
    let project = fixture.project("app", Initial::Committed).await;
    let count = OpenPullPage::MAX_PULLS + 1;
    for number in 1..=count {
        put(&fixture, &repo("app"), number, PullState::Open);
    }
    put(&fixture, &repo("app"), count + 1, PullState::Closed);
    put(&fixture, &repo("other"), 1, PullState::Open);
    let first = fixture.read(project.id, Mode::Cached).await.unwrap();
    assert_eq!(first.pulls().len(), OpenPullPage::MAX_PULLS);
    let continuation = first.continuation().unwrap();
    let final_page = fixture
        .client
        .read(project.id, Some(continuation), Mode::Cached)
        .await
        .unwrap();
    assert_eq!(final_page.pulls().len(), 1);
    assert_eq!(final_page.pulls()[0].number, PullNumber::new(1).unwrap());
    assert!(final_page.continuation().is_none());
    assert_eq!(fixture.fake.calls("list_open"), 2);
    assert_eq!(fixture.fake.calls("find"), 0);
    assert_eq!(fixture.fake.calls("read"), 0);
    fixture.finish().await;
}

#[tokio::test]
async fn a_configured_branch_remote_wins_and_a_detached_checkout_uses_origin() {
    let fixture = Fixture::new().await;
    let project = fixture.project("app", Initial::Committed).await;
    fixture.git(
        &project.root,
        &[
            "remote",
            "add",
            "fork",
            "https://github.com/fixture/fork.git",
        ],
    );
    fixture.git(&project.root, &["config", "branch.main.remote", "fork"]);
    assert_eq!(
        fixture.read(project.id, Mode::Cached).await.unwrap().repo(),
        &repo("fork")
    );
    fixture.git(&project.root, &["switch", "--detach", "-q"]);
    assert_eq!(
        fixture.read(project.id, Mode::Cached).await.unwrap().repo(),
        &repo("app")
    );
    fixture.finish().await;
}
