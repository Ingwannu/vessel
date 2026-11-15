//! Drive actual provider jobs and hold owner completions to prove order.
use super::super::{Actor, Completion, Request, cache::Cache};
use super::*;
use tokio::sync::{Semaphore, mpsc, oneshot};
use tokio::task::JoinSet;

fn actor(fixture: &Fixture) -> Actor {
    let (_sender, received) = mpsc::channel(MAX_ACTIVE);
    Actor {
        workspaces: Arc::clone(&fixture.workspaces),
        hosts: Arc::clone(&fixture.hosts),
        received,
        jobs: JoinSet::new(),
        cache: Cache::new(),
    }
}
fn request(
    project: ProjectId,
    mode: Mode,
    permits: &Arc<Semaphore>,
) -> (Request, oneshot::Receiver<Outcome>) {
    let keep = Arc::new(Arc::clone(permits).try_acquire_owned().unwrap());
    let (reply, answer) = oneshot::channel();
    (
        Request {
            project,
            cursor: None,
            mode,
            reply,
            keep,
        },
        answer,
    )
}
async fn accept(
    actor: &mut Actor,
    fixture: &Fixture,
    project: ProjectId,
    mode: Mode,
    permits: &Arc<Semaphore>,
) -> oneshot::Receiver<Outcome> {
    let (request, answer) = request(project, mode, permits);
    let target = step(
        "prepare project target",
        resolve::prepare(&fixture.workspaces, &fixture.hosts, &request),
    )
    .await
    .unwrap();
    actor.completed(Completion::Prepared(request, Ok(target)));
    answer
}
async fn completion(actor: &mut Actor) -> Completion {
    step("collect owner completion", actor.jobs.join_next())
        .await
        .unwrap()
        .unwrap()
}

#[tokio::test]
async fn later_refresh_success_cannot_be_overwritten_by_an_old_completion() {
    case(LIMIT, async {
        let fixture = Fixture::new().await;
        let project = fixture.project("app", Initial::Committed).await;
        let permits = Arc::new(Semaphore::new(MAX_ACTIVE));
        let mut actor = actor(&fixture);
        paging::put(
            &fixture,
            &repo("app"),
            1,
            vessel_proto::forge::PullState::Open,
        );
        let old = accept(&mut actor, &fixture, project.id, Mode::Cached, &permits).await;
        let held = completion(&mut actor).await;
        paging::put(
            &fixture,
            &repo("app"),
            2,
            vessel_proto::forge::PullState::Open,
        );
        let refresh = accept(&mut actor, &fixture, project.id, Mode::Refresh, &permits).await;
        let newest = completion(&mut actor).await;
        assert_eq!(fixture.fake.calls("list_open"), 2);
        actor.completed(newest);
        let page = step("owner reply", refresh).await.unwrap().unwrap();
        actor.completed(held);
        let old_page = step("owner reply", old).await.unwrap().unwrap();
        assert!(!Arc::ptr_eq(&page, &old_page));
        assert_eq!(old_page.pulls().len(), 1);
        assert_eq!(page.pulls().len(), 2);
        let cached = step(
            "cached owner reply",
            accept(&mut actor, &fixture, project.id, Mode::Cached, &permits).await,
        )
        .await
        .unwrap()
        .unwrap();
        assert!(Arc::ptr_eq(&page, &cached));
        assert_eq!(fixture.fake.calls("list_open"), 2);
        assert_eq!(permits.available_permits(), MAX_ACTIVE);
        fixture.finish().await;
    })
    .await;
}

#[tokio::test]
async fn failure_of_a_refresh_invalidates_a_hit_and_old_success_cannot_resurrect_it() {
    case(LIMIT, async {
        let fixture = Fixture::new().await;
        let project = fixture.project("app", Initial::Committed).await;
        let permits = Arc::new(Semaphore::new(MAX_ACTIVE));
        let mut actor = actor(&fixture);
        let old = accept(&mut actor, &fixture, project.id, Mode::Cached, &permits).await;
        let held = completion(&mut actor).await;
        fixture
            .fake
            .fail_repository(repo("app"), Some(Failure::NoAccess));
        let refresh = accept(&mut actor, &fixture, project.id, Mode::Refresh, &permits).await;
        let failed = completion(&mut actor).await;
        actor.completed(failed);
        assert!(step("owner reply", refresh).await.unwrap().is_err());
        actor.completed(held);
        assert!(step("owner reply", old).await.unwrap().is_ok());
        let retry = accept(&mut actor, &fixture, project.id, Mode::Cached, &permits).await;
        let failed = completion(&mut actor).await;
        actor.completed(failed);
        assert!(step("owner reply", retry).await.unwrap().is_err());
        assert_eq!(fixture.fake.calls("list_open"), 3);
        fixture.finish().await;
    })
    .await;
}

#[tokio::test]
async fn a_fresh_hit_is_available_during_refresh_but_is_removed_after_failure() {
    case(LIMIT, async {
        let fixture = Fixture::new().await;
        let project = fixture.project("app", Initial::Committed).await;
        let permits = Arc::new(Semaphore::new(MAX_ACTIVE));
        let mut actor = actor(&fixture);
        let first = accept(&mut actor, &fixture, project.id, Mode::Cached, &permits).await;
        let loaded = completion(&mut actor).await;
        actor.completed(loaded);
        let page = step("owner reply", first).await.unwrap().unwrap();
        fixture
            .fake
            .fail_repository(repo("app"), Some(Failure::RateLimited));
        let refresh = accept(&mut actor, &fixture, project.id, Mode::Refresh, &permits).await;
        let hit = step(
            "cached owner reply",
            accept(&mut actor, &fixture, project.id, Mode::Cached, &permits).await,
        )
        .await
        .unwrap()
        .unwrap();
        assert!(Arc::ptr_eq(&hit, &page));
        let failed = completion(&mut actor).await;
        actor.completed(failed);
        assert!(step("owner reply", refresh).await.unwrap().is_err());
        let next = accept(&mut actor, &fixture, project.id, Mode::Cached, &permits).await;
        let failed = completion(&mut actor).await;
        actor.completed(failed);
        assert!(step("owner reply", next).await.unwrap().is_err());
        fixture.finish().await;
    })
    .await;
}

#[tokio::test]
async fn coalesced_waiters_keep_admission_even_after_the_leader_is_cancelled() {
    case(LIMIT, async {
        let fixture = Fixture::new().await;
        let project = fixture.project("app", Initial::Committed).await;
        let permits = Arc::new(Semaphore::new(MAX_ACTIVE));
        let mut actor = actor(&fixture);
        let leader = accept(&mut actor, &fixture, project.id, Mode::Cached, &permits).await;
        let held = completion(&mut actor).await;
        let follower = accept(&mut actor, &fixture, project.id, Mode::Cached, &permits).await;
        drop(leader);
        assert_eq!(permits.available_permits(), MAX_ACTIVE - 2);
        assert_eq!(fixture.fake.calls("list_open"), 1);
        actor.completed(held);
        assert!(step("owner reply", follower).await.unwrap().is_ok());
        assert_eq!(permits.available_permits(), MAX_ACTIVE);
        fixture.finish().await;
    })
    .await;
}
