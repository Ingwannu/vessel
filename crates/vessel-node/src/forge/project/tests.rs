//! Real registered projects and FakeForge, plus driven completion order.
//! The actor's own load jobs produce both pages; tests hold completions
//! at the owner boundary instead of replacing the provider or cache rule.
mod fixture;
mod lifetime;
mod ordering;

use super::*;
use fixture::Fixture;
use vessel_forge::{Forge, Priority, fake::Failure};
use vessel_proto::forge::{ForgeKind, RepoRef};

fn repo(name: &str) -> RepoRef {
    RepoRef {
        kind: ForgeKind::Github,
        host: "github.com".parse().unwrap(),
        path: format!("fixture/{name}").parse().unwrap(),
    }
}

#[tokio::test]
async fn a_project_needs_no_session_pushed_head_or_nondefault_branch() {
    let fixture = Fixture::new().await;
    for checkout in ["base", "detached", "unborn"] {
        let project = fixture.project(checkout, checkout != "unborn").await;
        if checkout == "detached" {
            fixture.git(&project.root, &["switch", "--detach", "-q"]);
        }
        let page = fixture.read(project.id, Mode::Cached).await.unwrap();
        assert_eq!(page.repo(), &repo(checkout));
        assert!(page.pulls().is_empty());
    }
    assert_eq!(fixture.fake.calls("list_open"), 3);
    assert_eq!(fixture.fake.calls("find"), 0);
    assert_eq!(fixture.fake.calls("read"), 0);
    fixture.finish().await;
}

#[tokio::test]
async fn a_cache_hit_keeps_read_at_and_refresh_failure_does_not_become_empty() {
    let fixture = Fixture::new().await;
    let project = fixture.project("app", true).await;
    let first = fixture.read(project.id, Mode::Cached).await.unwrap();
    let same = fixture.read(project.id, Mode::Cached).await.unwrap();
    assert!(Arc::ptr_eq(&first, &same));
    assert_eq!(same.read_at(), first.read_at());
    assert_eq!(fixture.fake.calls("list_open"), 1);
    fixture
        .fake
        .fail_repository(repo("app"), Some(Failure::NoAccess));
    assert!(matches!(
        &*fixture.read(project.id, Mode::Refresh).await.unwrap_err(),
        Error::Provider {
            source: vessel_forge::ForgeError::Refused { .. },
            ..
        }
    ));
    assert!(fixture.read(project.id, Mode::Cached).await.is_err());
    assert_eq!(fixture.fake.calls("list_open"), 3);
    fixture.fake.fail_repository(repo("app"), None);
    assert!(fixture.read(project.id, Mode::Cached).await.is_ok());
    fixture.finish().await;
}

#[tokio::test]
async fn root_and_remote_are_checked_again_even_when_a_page_is_fresh() {
    let fixture = Fixture::new().await;
    let project = fixture.project("app", true).await;
    let first = fixture.read(project.id, Mode::Cached).await.unwrap();
    let cursor = vessel_proto::forge::ProjectPullCursor {
        repo: first.repo().clone(),
        after: "fixture:1".parse().unwrap(),
    };
    fixture.git(
        &project.root,
        &[
            "remote",
            "set-url",
            "origin",
            "https://github.com/fixture/other.git",
        ],
    );
    let calls = fixture.fake.calls("list_open");
    assert!(matches!(
        &*fixture
            .client
            .read(project.id, Some(cursor), Mode::Cached)
            .await
            .unwrap_err(),
        Error::CursorRepository
    ));
    assert_eq!(fixture.fake.calls("list_open"), calls);
    assert_eq!(
        fixture.read(project.id, Mode::Cached).await.unwrap().repo(),
        &repo("other")
    );
    std::fs::rename(&project.root, project.root.with_extension("moved")).unwrap();
    assert!(matches!(
        &*fixture.read(project.id, Mode::Cached).await.unwrap_err(),
        Error::Workspace(_)
    ));
    assert_eq!(fixture.fake.calls("list_open"), calls + 1);
    fixture.finish().await;
}

#[cfg(unix)]
#[tokio::test]
async fn replacing_a_registered_root_by_a_symlink_cannot_hit_the_cache() {
    let fixture = Fixture::new().await;
    let project = fixture.project("app", true).await;
    assert!(fixture.read(project.id, Mode::Cached).await.is_ok());
    let moved = project.root.with_extension("moved");
    std::fs::rename(&project.root, &moved).unwrap();
    std::os::unix::fs::symlink(&moved, &project.root).unwrap();
    assert!(matches!(
        &*fixture.read(project.id, Mode::Cached).await.unwrap_err(),
        Error::Workspace(_)
    ));
    assert_eq!(fixture.fake.calls("list_open"), 1);
    fixture.finish().await;
}

#[tokio::test]
async fn expired_pages_are_loaded_again_with_driven_time() {
    let fixture = Fixture::new().await;
    let project = fixture.project("app", true).await;
    let first = fixture.read(project.id, Mode::Cached).await.unwrap();
    tokio::time::pause();
    tokio::time::advance(FRESH).await;
    tokio::time::resume();
    let next = fixture.read(project.id, Mode::Cached).await.unwrap();
    assert!(!Arc::ptr_eq(&first, &next));
    assert_eq!(fixture.fake.calls("list_open"), 2);
    fixture.finish().await;
}

#[tokio::test]
async fn provider_failures_stay_distinct_and_the_next_call_can_recover() {
    let fixture = Fixture::new().await;
    let project = fixture.project("app", true).await;
    for failure in [
        Failure::Down,
        Failure::RateLimited,
        Failure::SignedOut,
        Failure::NoAccess,
    ] {
        fixture.fake.fail_repository(repo("app"), Some(failure));
        let error = fixture.read(project.id, Mode::Refresh).await.unwrap_err();
        let Error::Provider { access, .. } = &*error else {
            panic!("provider error: {error:?}");
        };
        assert!(matches!(
            (failure, access),
            (Failure::Down, ForgeAccess::Unreachable { .. })
                | (Failure::RateLimited, ForgeAccess::RateLimited { .. })
                | (Failure::SignedOut, ForgeAccess::SignedOut { .. })
                | (Failure::NoAccess, ForgeAccess::Refused { .. })
        ));
        fixture.fake.fail_repository(repo("app"), None);
        assert!(
            fixture
                .read(project.id, Mode::Cached)
                .await
                .unwrap()
                .pulls()
                .is_empty()
        );
    }
    fixture.finish().await;
}

#[tokio::test]
async fn missing_remote_plain_folder_and_missing_sign_in_do_not_read_a_forge() {
    let fixture = Fixture::new().await;
    let project = fixture.project("app", true).await;
    fixture.git(&project.root, &["remote", "remove", "origin"]);
    assert!(matches!(
        &*fixture.read(project.id, Mode::Cached).await.unwrap_err(),
        Error::Access(ForgeAccess::NoRemote)
    ));
    let plain = fixture.temp.path().join("plain");
    std::fs::create_dir(&plain).unwrap();
    let plain = fixture.workspaces.add_project(plain, None).await.unwrap();
    assert!(matches!(
        &*fixture.read(plain.id, Mode::Cached).await.unwrap_err(),
        Error::Workspace(WorkspaceError::NotGit)
    ));
    assert_eq!(fixture.fake.calls("list_open"), 0);
    fixture.git(
        &project.root,
        &[
            "remote",
            "add",
            "origin",
            "https://github.com/fixture/app.git",
        ],
    );
    let hosts = Arc::new(Hosts::new(
        super::super::ForgeSetup::default(),
        fixture.temp.path().to_path_buf(),
    ));
    let client = ProjectPulls::start(
        Arc::clone(&fixture.workspaces),
        hosts,
        &fixture.tasks,
        fixture.stop.clone(),
    );
    assert!(matches!(
        &*client
            .read(project.id, None, Mode::Cached)
            .await
            .unwrap_err(),
        Error::Access(ForgeAccess::SignedOut { .. })
    ));
    fixture.finish().await;
}
