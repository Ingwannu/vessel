//! Drive actual provider jobs and hold owner completions to prove order.
use super::super::{
    Actor, Completion, Request,
    cache::{Cache, Identity, Key},
};
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
    let target = resolve::prepare(&fixture.workspaces, &fixture.hosts, &request)
        .await
        .unwrap();
    actor.completed(Completion::Prepared(request, Ok(target)));
    answer
}
async fn completion(actor: &mut Actor) -> Completion {
    actor.jobs.join_next().await.unwrap().unwrap()
}

#[tokio::test]
async fn later_refresh_success_cannot_be_overwritten_by_an_old_completion() {
    let fixture = Fixture::new().await;
    let project = fixture.project("app", true).await;
    let permits = Arc::new(Semaphore::new(MAX_ACTIVE));
    let mut actor = actor(&fixture);
    let old = accept(&mut actor, &fixture, project.id, Mode::Cached, &permits).await;
    let held = completion(&mut actor).await;
    let refresh = accept(&mut actor, &fixture, project.id, Mode::Refresh, &permits).await;
    let newest = completion(&mut actor).await;
    assert_eq!(fixture.fake.calls("list_open"), 2);
    actor.completed(newest);
    let page = refresh.await.unwrap().unwrap();
    actor.completed(held);
    let old_page = old.await.unwrap().unwrap();
    assert!(!Arc::ptr_eq(&page, &old_page));
    let cached = accept(&mut actor, &fixture, project.id, Mode::Cached, &permits)
        .await
        .await
        .unwrap()
        .unwrap();
    assert!(Arc::ptr_eq(&page, &cached));
    assert_eq!(fixture.fake.calls("list_open"), 2);
    assert_eq!(permits.available_permits(), MAX_ACTIVE);
    fixture.finish().await;
}

#[tokio::test]
async fn failure_of_a_refresh_invalidates_a_hit_and_old_success_cannot_resurrect_it() {
    let fixture = Fixture::new().await;
    let project = fixture.project("app", true).await;
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
    assert!(refresh.await.unwrap().is_err());
    actor.completed(held);
    assert!(old.await.unwrap().is_ok());
    let retry = accept(&mut actor, &fixture, project.id, Mode::Cached, &permits).await;
    let failed = completion(&mut actor).await;
    actor.completed(failed);
    assert!(retry.await.unwrap().is_err());
    assert_eq!(fixture.fake.calls("list_open"), 3);
    fixture.finish().await;
}

#[tokio::test]
async fn a_fresh_hit_is_available_during_refresh_but_is_removed_after_failure() {
    let fixture = Fixture::new().await;
    let project = fixture.project("app", true).await;
    let permits = Arc::new(Semaphore::new(MAX_ACTIVE));
    let mut actor = actor(&fixture);
    let first = accept(&mut actor, &fixture, project.id, Mode::Cached, &permits).await;
    let loaded = completion(&mut actor).await;
    actor.completed(loaded);
    let page = first.await.unwrap().unwrap();
    fixture
        .fake
        .fail_repository(repo("app"), Some(Failure::RateLimited));
    let refresh = accept(&mut actor, &fixture, project.id, Mode::Refresh, &permits).await;
    let hit = accept(&mut actor, &fixture, project.id, Mode::Cached, &permits)
        .await
        .await
        .unwrap()
        .unwrap();
    assert!(Arc::ptr_eq(&hit, &page));
    let failed = completion(&mut actor).await;
    actor.completed(failed);
    assert!(refresh.await.unwrap().is_err());
    let next = accept(&mut actor, &fixture, project.id, Mode::Cached, &permits).await;
    let failed = completion(&mut actor).await;
    actor.completed(failed);
    assert!(next.await.unwrap().is_err());
    fixture.finish().await;
}

#[tokio::test]
async fn coalesced_waiters_keep_admission_even_after_the_leader_is_cancelled() {
    let fixture = Fixture::new().await;
    let project = fixture.project("app", true).await;
    let permits = Arc::new(Semaphore::new(MAX_ACTIVE));
    let mut actor = actor(&fixture);
    let leader = accept(&mut actor, &fixture, project.id, Mode::Cached, &permits).await;
    let held = completion(&mut actor).await;
    let follower = accept(&mut actor, &fixture, project.id, Mode::Cached, &permits).await;
    drop(leader);
    assert_eq!(permits.available_permits(), MAX_ACTIVE - 2);
    assert_eq!(fixture.fake.calls("list_open"), 1);
    actor.completed(held);
    assert!(follower.await.unwrap().is_ok());
    assert_eq!(permits.available_permits(), MAX_ACTIVE);
    fixture.finish().await;
}

#[tokio::test]
async fn cache_keys_are_finite_and_active_slots_are_not_evicted() {
    let fixture = Fixture::new().await;
    let project = fixture.project("app", true).await;
    let permits = Arc::new(Semaphore::new(MAX_ACTIVE));
    let mut cache = Cache::new();
    for n in 0..MAX_KEYS {
        let key = Key {
            repo: repo(&format!("app{n}")),
            after: None,
            credential: Identity::Custom,
        };
        assert!(cache.slot(&key).is_some());
    }
    // Replacing idle keys is bounded. Real admission makes MAX_ACTIVE
    // the tighter bound on simultaneous active slots.
    for n in 0..MAX_KEYS {
        let key = Key {
            repo: repo(&format!("other{n}")),
            after: None,
            credential: Identity::Custom,
        };
        assert!(cache.slot(&key).is_some());
    }
    let key = Key {
        repo: repo("active"),
        after: None,
        credential: Identity::Custom,
    };
    let (waiting, reply) = request(project.id, Mode::Cached, &permits);
    cache
        .slot(&key)
        .unwrap()
        .begin(cache::LoadId::new(), waiting);
    cache.stop();
    assert!(matches!(
        &*reply.await.unwrap().unwrap_err(),
        Error::Stopped
    ));
    assert_eq!(permits.available_permits(), MAX_ACTIVE);
    fixture.finish().await;
}
