//! Actual node owner plus GitHub on loopback, including observed refusals
//! and reversed in-flight completion under channel barriers.
mod stand;
use super::*;
use axum::http::StatusCode;
use stand::{EMPTY, Reply, Stand};
use vessel_forge::{Forge, Priority};

const LIMIT: Duration = Duration::from_secs(10);
#[tokio::test]
async fn a_refusal_observed_by_the_real_provider_prevents_a_node_cache_hit() {
    vessel_testkit::within(
        LIMIT,
        Box::pin(async {
            let fixture = Fixture::new().await;
            let project = fixture.project("app", Initial::Committed).await;
            let stand = Stand::start(&fixture).await;
            let client = stand.client(&fixture);
            stand.queue(Reply::immediate(StatusCode::OK, EMPTY));
            let first = step(
                "project client reply",
                client.read(project.id, None, Mode::Cached),
            )
            .await
            .unwrap();
            let before = stand
                .credentials
                .cached_identity(&repo("app").host)
                .unwrap();
            stand.queue(Reply::immediate(StatusCode::UNAUTHORIZED, "{}"));
            assert!(
                step(
                    "loopback provider reply",
                    stand.github.list_open(&repo("app"), None, Priority::Person)
                )
                .await
                .is_err()
            );
            assert_ne!(
                stand
                    .credentials
                    .cached_identity(&repo("app").host)
                    .unwrap(),
                before
            );
            stand.queue(Reply::immediate(StatusCode::OK, EMPTY));
            let next = step(
                "project client reply",
                client.read(project.id, None, Mode::Cached),
            )
            .await
            .unwrap();
            assert!(!Arc::ptr_eq(&first, &next));
            assert_eq!(stand.calls(), 3);
            fixture.finish().await;
        }),
    )
    .await
    .expect("the loopback project-read test must finish");
}

#[tokio::test]
async fn a_refusal_during_a_held_read_prevents_publication_and_the_next_read_recovers() {
    vessel_testkit::within(
        LIMIT,
        Box::pin(async {
            let fixture = Fixture::new().await;
            let project = fixture.project("app", Initial::Committed).await;
            let stand = Stand::start(&fixture).await;
            let client = stand.client(&fixture);
            let (held, started, release) = Reply::held();
            stand.queue(held);
            let caller = Arc::clone(&client);
            let loading = tokio::spawn(async move {
                step(
                    "project client reply",
                    caller.read(project.id, None, Mode::Cached),
                )
                .await
            });
            step("HTTP barrier reached", started).await.unwrap();
            stand.queue(Reply::immediate(StatusCode::UNAUTHORIZED, "{}"));
            assert!(
                step(
                    "loopback provider reply",
                    stand.github.list_open(&repo("app"), None, Priority::Person)
                )
                .await
                .is_err()
            );
            release.send(()).unwrap();
            assert!(matches!(
                &*step("owner reply", loading).await.unwrap().unwrap_err(),
                Error::CredentialChanged
            ));
            stand.queue(Reply::immediate(StatusCode::OK, EMPTY));
            assert!(
                step(
                    "project client reply",
                    client.read(project.id, None, Mode::Cached)
                )
                .await
                .is_ok()
            );
            assert_eq!(stand.calls(), 3);
            fixture.finish().await;
        }),
    )
    .await
    .expect("the loopback project-read test must finish");
}

#[tokio::test]
async fn a_newer_loopback_refresh_wins_when_the_old_http_read_finishes_last() {
    vessel_testkit::within(
        LIMIT,
        Box::pin(async {
            let fixture = Fixture::new().await;
            let project = fixture.project("app", Initial::Committed).await;
            let stand = Stand::start(&fixture).await;
            let client = stand.client(&fixture);
            let (old, old_started, old_release) = Reply::held();
            stand.queue(old);
            let caller = Arc::clone(&client);
            let old_call = tokio::spawn(async move {
                step(
                    "project client reply",
                    caller.read(project.id, None, Mode::Cached),
                )
                .await
            });
            step("HTTP barrier reached", old_started).await.unwrap();
            let (new, new_started, new_release) = Reply::held();
            stand.queue(new);
            let caller = Arc::clone(&client);
            let refresh = tokio::spawn(async move {
                step(
                    "project client reply",
                    caller.read(project.id, None, Mode::Refresh),
                )
                .await
            });
            step("HTTP barrier reached", new_started).await.unwrap();
            new_release.send(()).unwrap();
            let current = step("owner reply", refresh).await.unwrap().unwrap();
            old_release.send(()).unwrap();
            let older = step("owner reply", old_call).await.unwrap().unwrap();
            assert!(!Arc::ptr_eq(&current, &older));
            let hit = step(
                "project client reply",
                client.read(project.id, None, Mode::Cached),
            )
            .await
            .unwrap();
            assert!(Arc::ptr_eq(&hit, &current));
            assert_eq!(stand.calls(), 2);
            fixture.finish().await;
        }),
    )
    .await
    .expect("the loopback project-read test must finish");
}

#[tokio::test]
async fn a_prepared_completion_cannot_start_work_after_its_credential_was_refused() {
    vessel_testkit::within(
        LIMIT,
        Box::pin(async {
            let fixture = Fixture::new().await;
            let project = fixture.project("app", Initial::Committed).await;
            let stand = Stand::start(&fixture).await;
            let permits = Arc::new(Semaphore::new(MAX_ACTIVE));
            let (reply, answer) = oneshot::channel();
            let request = Request {
                project: project.id,
                cursor: None,
                mode: Mode::Cached,
                reply,
                keep: Arc::new(Arc::clone(&permits).try_acquire_owned().unwrap()),
            };
            let target = step(
                "prepare project target",
                resolve::prepare(&fixture.workspaces, &stand.hosts, &request),
            )
            .await
            .unwrap();
            stand.queue(Reply::immediate(StatusCode::UNAUTHORIZED, "{}"));
            assert!(
                step(
                    "loopback provider reply",
                    stand.github.list_open(&repo("app"), None, Priority::Person)
                )
                .await
                .is_err()
            );
            let (_sender, received) = mpsc::channel(MAX_ACTIVE);
            let mut owner = Actor {
                workspaces: Arc::clone(&fixture.workspaces),
                hosts: Arc::clone(&stand.hosts),
                received,
                jobs: JoinSet::new(),
                cache: Cache::new(),
            };
            owner.completed(Completion::Prepared(request, Ok(target)));
            assert!(matches!(
                &*step("owner reply", answer).await.unwrap().unwrap_err(),
                Error::CredentialChanged
            ));
            assert!(owner.jobs.is_empty());
            assert_eq!(stand.calls(), 1);
            assert_eq!(permits.available_permits(), MAX_ACTIVE);
            drop(owner);
            fixture.finish().await;
        }),
    )
    .await
    .expect("the loopback project-read test must finish");
}

#[tokio::test]
async fn a_queued_success_cannot_publish_after_its_credential_was_refused() {
    vessel_testkit::within(
        LIMIT,
        Box::pin(async {
            let fixture = Fixture::new().await;
            let project = fixture.project("app", Initial::Committed).await;
            let stand = Stand::start(&fixture).await;
            let permits = Arc::new(Semaphore::new(MAX_ACTIVE));
            let (reply, answer) = oneshot::channel();
            let request = Request {
                project: project.id,
                cursor: None,
                mode: Mode::Cached,
                reply,
                keep: Arc::new(Arc::clone(&permits).try_acquire_owned().unwrap()),
            };
            let target = step(
                "prepare project target",
                resolve::prepare(&fixture.workspaces, &stand.hosts, &request),
            )
            .await
            .unwrap();
            let (_sender, received) = mpsc::channel(MAX_ACTIVE);
            let mut owner = Actor {
                workspaces: Arc::clone(&fixture.workspaces),
                hosts: Arc::clone(&stand.hosts),
                received,
                jobs: JoinSet::new(),
                cache: Cache::new(),
            };
            stand.queue(Reply::immediate(StatusCode::OK, EMPTY));
            owner.completed(Completion::Prepared(request, Ok(target)));
            let completed = step("collect owner completion", owner.jobs.join_next())
                .await
                .unwrap()
                .unwrap();
            assert_eq!(permits.available_permits(), MAX_ACTIVE - 1);
            stand.queue(Reply::immediate(StatusCode::UNAUTHORIZED, "{}"));
            assert!(
                step(
                    "loopback provider reply",
                    stand.github.list_open(&repo("app"), None, Priority::Person)
                )
                .await
                .is_err()
            );
            owner.completed(completed);
            assert!(matches!(
                &*step("owner reply", answer).await.unwrap().unwrap_err(),
                Error::CredentialChanged
            ));
            assert_eq!(stand.calls(), 2);
            assert_eq!(permits.available_permits(), MAX_ACTIVE);
            drop(owner);
            fixture.finish().await;
        }),
    )
    .await
    .expect("a queued success must be revalidated");
}
