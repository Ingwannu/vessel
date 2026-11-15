//! Component limits and credential associations use the same owned cache.
use super::*;
use crate::forge::project::{Admission, MAX_ACTIVE};
use tokio::sync::{Semaphore, oneshot};
use vessel_forge::{Credentials, EnvTokens, Forge, Priority, Token, fake::FakeForge};
use vessel_proto::{
    ProjectId,
    forge::{ForgeKind, RepoRef},
};

fn key(name: &str, credential: Identity) -> Key {
    Key {
        repo: RepoRef {
            kind: ForgeKind::Github,
            host: "github.com".parse().unwrap(),
            path: format!("fixture/{name}").parse().unwrap(),
        },
        after: None,
        credential,
    }
}
fn waiter(permit: Admission) -> (Request, oneshot::Receiver<Outcome>) {
    let (reply, answer) = oneshot::channel();
    (
        Request {
            project: ProjectId::random(),
            cursor: None,
            mode: Mode::Cached,
            reply,
            keep: permit,
        },
        answer,
    )
}
#[tokio::test]
async fn idle_keys_are_capped_and_an_active_key_survives_eviction() {
    let mut cache = Cache::new();
    let permits = Arc::new(Semaphore::new(MAX_ACTIVE));
    let active = key("active", Identity::Custom);
    let keep = Arc::new(Arc::clone(&permits).try_acquire_owned().unwrap());
    let (request, answer) = waiter(keep);
    cache.slot(&active).unwrap().begin(LoadId::new(), request);
    for number in 0..MAX_KEYS * 2 {
        assert!(
            cache
                .slot(&key(&format!("idle{number}"), Identity::Custom))
                .is_some()
        );
        assert!(cache.slots.len() <= MAX_KEYS);
        assert!(
            cache
                .slots
                .iter()
                .any(|slot| slot.key == active && !slot.flights.is_empty())
        );
    }
    assert_eq!(cache.slots.len(), MAX_KEYS);
    cache.stop();
    assert!(matches!(
        &*answer.await.unwrap().unwrap_err(),
        Error::Stopped
    ));
    assert_eq!(permits.available_permits(), MAX_ACTIVE);
}
#[tokio::test]
async fn a_known_refusal_cannot_reuse_a_page_under_the_current_credential_identity() {
    let credentials = Credentials::new(
        EnvTokens {
            github: Token::new("fixture-configured"),
            enterprise: None,
        },
        None,
    );
    let host = "github.com".parse().unwrap();
    let first_identity = credentials.identity(&host).await.unwrap();
    let first = key("app", Identity::Credential(first_identity.clone()));
    let fake = FakeForge::new();
    let page = Arc::new(
        fake.list_open(&first.repo, None, Priority::Person)
            .await
            .unwrap(),
    );
    let permits = Arc::new(Semaphore::new(MAX_ACTIVE));
    let keep = Arc::new(Arc::clone(&permits).try_acquire_owned().unwrap());
    let (request, answer) = waiter(keep);
    let mut cache = Cache::new();
    let load = LoadId::new();
    cache.slot(&first).unwrap().begin(load.clone(), request);
    cache.finish(&first, &load, &Ok(Arc::clone(&page)), Instant::now());
    assert!(Arc::ptr_eq(&answer.await.unwrap().unwrap(), &page));
    assert!(cache.slot(&first).unwrap().reuse(Mode::Cached).is_some());
    credentials.refused(&host);
    let selected = credentials.identity(&host).await.unwrap();
    assert_ne!(first_identity, selected);
    let current = key("app", Identity::Credential(selected));
    assert!(cache.slot(&current).unwrap().reuse(Mode::Cached).is_none());
    assert_eq!(cache.slots.len(), 2);
    assert!(
        cache.slot(&first).unwrap().reuse(Mode::Cached).is_some(),
        "old allocation stays alive, but current selection never asks for its page"
    );
}
