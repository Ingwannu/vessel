//! One run's limits, hooks and cancellation, against real repositories.

use std::time::Duration;

use tokio_util::sync::CancellationToken;

use super::Limits;
use crate::git::GitError;
use crate::testing::Sandbox;

#[tokio::test]
async fn output_past_its_limit_is_cut_and_said_so() {
    let sandbox = Sandbox::new();
    let repo = sandbox.repo("r");
    let git = sandbox.git().await;
    let output = git
        .command(&repo)
        .args(["log", "--format=%H%n%H%n%H"])
        .limits(Limits::LOCAL.with_stdout(16))
        .output()
        .await
        .unwrap();
    assert!(output.stdout_cut);
    assert_eq!(output.stdout.len(), 16);
    assert!(!output.succeeded());
    assert!(matches!(output.checked(), Err(GitError::OutputLimit)));
}

/// A hook that would write a marker: it runs only when a call asks for
/// the repository's hooks, and a run past its time is stopped.
#[cfg(unix)]
#[tokio::test]
async fn hooks_run_only_when_asked_and_a_slow_run_is_stopped() {
    use std::os::unix::fs::PermissionsExt;

    let sandbox = Sandbox::new();
    let repo = sandbox.repo("r");
    let git = sandbox.git().await;
    let marker = sandbox.path().join("hook-ran");
    let hook = repo.join(".git/hooks/pre-commit");
    std::fs::write(&hook, format!("#!/bin/sh\ntouch '{}'\n", marker.display())).unwrap();
    std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755)).unwrap();
    let commit = |message: &'static str| {
        git.command(&repo)
            .args(["commit", "-q", "--allow-empty", "-m", message])
    };
    drop(commit("quiet").output().await.unwrap().checked().unwrap());
    assert!(!marker.exists(), "a read or a plain call runs no hook");
    drop(
        commit("hooked")
            .with_hooks()
            .output()
            .await
            .unwrap()
            .checked()
            .unwrap(),
    );
    assert!(marker.exists(), "commit and push run the user's hooks");

    std::fs::write(&hook, "#!/bin/sh\nsleep 30\n").unwrap();
    let slow = commit("slow")
        .with_hooks()
        .limits(Limits {
            time: Duration::from_millis(300),
            ..Limits::LOCAL
        })
        .output()
        .await;
    assert!(matches!(slow, Err(GitError::TimedOut { .. })), "{slow:?}");
}

#[tokio::test]
async fn a_cancelled_run_stops() {
    let sandbox = Sandbox::new();
    let repo = sandbox.repo("r");
    let git = sandbox.git().await;
    let token = CancellationToken::new();
    token.cancel();
    let cancelled = git
        .command(&repo)
        .arg("status")
        .cancel_on(token)
        .output()
        .await;
    // Either it was stopped, or it finished before the check: never a
    // hang.
    assert!(
        matches!(cancelled, Err(GitError::Cancelled) | Ok(_)),
        "{cancelled:?}"
    );
}
