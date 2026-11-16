//! Native metadata requests preserve explicit intent and precede preparatory work.
use crate::support::{Fixture, within};
use vessel_proto::lifecycle_version;
#[cfg(unix)]
use vessel_proto::{ProjectId, SessionId};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn native_discovery_falls_back_only_for_its_implicit_default() {
    within(async {
        let fixture = Fixture::start().await;
        let _ordinary = fixture.run_detached().await;
        let default = fixture.vessel(&["sessions"], b"").await;
        assert!(default.ok, "{}", default.stderr);
        assert!(default.stdout.contains("claude-code"));
        for args in [
            vec!["sessions", "--background"],
            vec!["sessions", "--label", "purpose=absent"],
        ] {
            let result = fixture.vessel(&args, b"").await;
            if lifecycle_version::SINCE.is_none() {
                assert!(!result.ok);
                assert!(result.stdout.is_empty());
                assert!(result.stderr.contains("not assigned"), "{}", result.stderr);
            } else {
                assert!(result.ok, "{}", result.stderr);
                assert_eq!(
                    result.stdout.contains("claude-code"),
                    args.contains(&"--background")
                );
            }
        }
        fixture.stop().await;
    })
    .await;
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn creation_metadata_admission_precedes_real_worktree_preparation() {
    within(async {
        use vessel_proto::workspace::{GetProjectCommands, InitProject, ProjectCommands};
        let fixture = crate::terminals::start().await;
        let owner = fixture.owner().await;
        let root = tempfile::tempdir().unwrap();
        let project = owner
            .call(InitProject {
                path: root.path().join("repo"),
                name: None,
            })
            .await
            .unwrap();
        commit(&project.root).await;
        let commands = owner
            .call(GetProjectCommands {
                project: project.id,
                base: None,
            })
            .await
            .unwrap();
        assert!(
            matches!(commands, ProjectCommands::None),
            "fixture has no commands to approve"
        );
        let project_id = project.id.to_string();
        worktree_control(&fixture, &owner, project.id).await;
        for flags in [
            vec!["--listing", "shown"],
            vec!["--listing", "background"],
            vec!["--label", "purpose=guard"],
        ] {
            let prior_sessions = owner.sessions().await.unwrap().len();
            let prior_worktrees = worktrees(&owner, project.id).await;
            let mut args = vec![
                "run",
                "claude",
                "--structured",
                "--detach",
                "--project",
                &project_id,
                "--new-worktree",
            ];
            args.extend(flags.iter().copied());
            let result = fixture.vessel(&args, b"").await;
            if lifecycle_version::SINCE.is_none() {
                assert!(!result.ok);
                assert!(result.stdout.is_empty());
                assert!(result.stderr.contains("not assigned"), "{}", result.stderr);
                assert!(!result.stderr.contains("[vessel] setup:"));
                assert_eq!(worktrees(&owner, project.id).await, prior_worktrees);
                assert_eq!(owner.sessions().await.unwrap().len(), prior_sessions);
            } else {
                assert!(result.ok, "{}", result.stderr);
                let id: SessionId = result.stdout.trim().parse().unwrap();
                let applied = owner.session(id).await.unwrap();
                if flags.contains(&"background") {
                    assert_eq!(applied.listing, vessel_proto::SessionListing::Background);
                } else if flags.contains(&"shown") {
                    assert_eq!(applied.listing, vessel_proto::SessionListing::Shown);
                } else {
                    assert!(
                        applied
                            .spec
                            .labels
                            .iter()
                            .any(|(key, value)| key.as_str() == "purpose"
                                && value.as_str() == "guard")
                    );
                }
                assert_eq!(
                    worktrees(&owner, project.id).await.len(),
                    prior_worktrees.len().checked_add(1).unwrap()
                );
            }
        }
        fixture.stop().await;
    })
    .await;
}
#[cfg(unix)]
async fn worktree_control(
    fixture: &Fixture,
    owner: &vessel_client::NodeClient,
    project: ProjectId,
) {
    use vessel_proto::workspace::SessionPlace;
    let initial = worktrees(owner, project).await;
    let project_id = project.to_string();
    let control = fixture
        .vessel(
            &[
                "run",
                "claude",
                "--structured",
                "--detach",
                "--project",
                &project_id,
                "--new-worktree",
            ],
            b"",
        )
        .await;
    assert!(control.ok, "{}", control.stderr);
    let control_id: SessionId = control.stdout.trim().parse().unwrap();
    let after = worktrees(owner, project).await;
    assert_eq!(after.len(), initial.len().checked_add(1).unwrap());
    let created = after
        .iter()
        .filter(|id| !initial.contains(id))
        .copied()
        .collect::<Vec<_>>();
    assert_eq!(created.len(), 1, "control must really create one worktree");
    let session = owner.session(control_id).await.unwrap();
    match session.spec.place.as_deref() {
        Some(SessionPlace::Worktree { worktree }) => assert!(created.contains(worktree)),
        Some(SessionPlace::Project { .. }) | None => panic!("control did not use its new worktree"),
    }
}

#[cfg(unix)]
async fn commit(path: &std::path::Path) {
    let git = which::which("git").unwrap();
    let no_hooks = path.join("fixture-no-hooks");
    std::fs::create_dir(&no_hooks).unwrap();
    // InitProject already recorded its default branch. Keep that branch so
    // worktree creation tests admission, rather than an external branch rename.
    let args = [
        "-c",
        "user.name=Fixture",
        "-c",
        "user.email=fixture@example.invalid",
        "-c",
        "commit.gpgsign=false",
        "commit",
        "--allow-empty",
        "-q",
        "-m",
        "fixture",
    ];
    let result = tokio::process::Command::new(&git)
        .arg("-c")
        .arg(format!("core.hooksPath={}", no_hooks.display()))
        .args(args)
        .current_dir(path)
        .env_clear()
        .env("HOME", path)
        .env("XDG_CONFIG_HOME", path)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .await
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}
#[cfg(unix)]
async fn worktrees(
    client: &vessel_client::NodeClient,
    project: ProjectId,
) -> Vec<vessel_proto::WorktreeId> {
    let response = client
        .call(vessel_proto::workspace::ListWorktrees {
            project: Some(project),
        })
        .await
        .unwrap();
    assert!(!response.more);
    let mut ids = response
        .worktrees
        .into_iter()
        .map(|row| row.id)
        .collect::<Vec<_>>();
    ids.sort_unstable();
    ids
}
