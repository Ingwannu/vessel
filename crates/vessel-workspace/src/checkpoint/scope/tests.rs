//! Real Git and isolated policy inputs exercise the whole admitted capture scope.
use super::*;
use crate::testing::Sandbox;
use std::{fs, os::unix::fs::PermissionsExt as _, path::PathBuf};
fn archive(sandbox: &Sandbox, name: &str) -> Archive {
    let path = sandbox.path().join(name);
    fs::create_dir(&path).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
    Archive::open(path).unwrap()
}
async fn captured(
    git: Git,
    root: PathBuf,
    archive: Archive,
    defaults: Defaults,
) -> Result<ObservedSnapshot, Error> {
    let runtime = tokio::runtime::Handle::current();
    tokio::task::spawn_blocking(move || {
        capture(
            &runtime,
            &git,
            &root,
            &archive,
            &defaults,
            &CancellationToken::new(),
        )
    })
    .await
    .unwrap()
}
#[tokio::test]
async fn checkpoint_global_and_repository_ignore_policies_never_expand_capture_coverage() {
    let sandbox = Sandbox::new();
    let dir = sandbox.repo("project");
    let git = isolated_git(&sandbox, None).await;
    let global = sandbox.path().join("global-ignore");
    fs::write(&global, b"global-output\n").unwrap();
    sandbox.run(
        &dir,
        &["config", "core.excludesFile", global.to_str().unwrap()],
    );
    fs::write(dir.join(".git/info/exclude"), b"admin-output\n").unwrap();
    fs::write(dir.join(".gitignore"), b"local-output\n").unwrap();
    for name in ["global-output", "admin-output", "local-output"] {
        fs::write(dir.join(name), b"excluded").unwrap();
    }
    let defaults = Defaults::new(sandbox.path().join("unused-default")).unwrap();
    let first = captured(
        git.clone(),
        dir.clone(),
        archive(&sandbox, "first"),
        defaults.clone(),
    )
    .await
    .unwrap();
    for name in ["global-output", "admin-output", "local-output"] {
        assert!(
            !first
                .image()
                .entries()
                .iter()
                .any(|entry| entry.path.relative().as_str() == name)
        );
    }
    // Even with identical visible names and contents, changed policy bytes stale
    // a future preview. Ignored contents are not copied to the private archive.
    fs::write(&global, b"global-output\n# policy change\n").unwrap();
    let second = captured(git, dir, archive(&sandbox, "second"), defaults)
        .await
        .unwrap();
    assert_eq!(first.image(), second.image());
    assert_ne!(first.evidence().ignore(), second.evidence().ignore());
}
#[tokio::test]
async fn checkpoint_default_global_policy_is_explicit_and_nested_gitignore_is_included() {
    let sandbox = Sandbox::new();
    let dir = sandbox.repo("project");
    let git = isolated_git(&sandbox, None).await;
    fs::create_dir(dir.join("nested")).unwrap();
    fs::write(dir.join("nested/visible"), b"file").unwrap();
    fs::write(dir.join("nested/.gitignore"), b"hidden\n").unwrap();
    fs::write(dir.join("nested/hidden"), b"excluded").unwrap();
    let global = sandbox.path().join("xdg/git/ignore");
    fs::write(&global, b"# explicitly absent config\n").unwrap();
    let defaults = Defaults::new(global.clone()).unwrap();
    let first = captured(
        git.clone(),
        dir.clone(),
        archive(&sandbox, "first"),
        defaults.clone(),
    )
    .await
    .unwrap();
    fs::write(&global, b"# changed default policy\n").unwrap();
    let second = captured(git, dir, archive(&sandbox, "second"), defaults)
        .await
        .unwrap();
    assert_eq!(first.image(), second.image());
    assert_ne!(first.evidence().ignore(), second.evidence().ignore());
    assert!(
        !first
            .image()
            .entries()
            .iter()
            .any(|entry| entry.path.relative().as_str() == "nested/hidden")
    );
}
#[tokio::test]
async fn checkpoint_a_file_created_after_reading_is_refused_by_the_second_enumeration() {
    let sandbox = Sandbox::new();
    let dir = sandbox.repo("project");
    let wrapper = sandbox.path().join("git-wrapper");
    Sandbox::executable(
        &wrapper,
        r#"#!/bin/sh
for arg in "$@"; do
    if [ "$arg" = "ls-files" ]; then
        if [ -e "$0.count" ]; then printf 'late file' > newly-created; fi
        : > "$0.count"
    fi
done
exec git "$@"
"#,
    );
    let git = isolated_git(&sandbox, Some(wrapper)).await;
    let defaults = Defaults::new(sandbox.path().join("xdg/git/ignore")).unwrap();
    assert!(matches!(
        captured(git, dir, archive(&sandbox, "first"), defaults).await,
        Err(Error::Refused(CheckpointError::CaptureUnstable))
    ));
}

async fn isolated_git(sandbox: &Sandbox, program: Option<PathBuf>) -> Git {
    fs::create_dir_all(sandbox.path().join("xdg/git")).unwrap();
    fs::create_dir_all(sandbox.path().join("home")).unwrap();
    sandbox
        .git_through_env(
            program.unwrap_or_else(|| which::which("git").unwrap()),
            vec![
                (
                    "HOME".to_owned(),
                    sandbox.path().join("home").into_os_string(),
                ),
                (
                    "XDG_CONFIG_HOME".to_owned(),
                    sandbox.path().join("xdg").into_os_string(),
                ),
            ],
        )
        .await
}
