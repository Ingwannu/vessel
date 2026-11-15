//! `vessel shell`, `vessel script` and `vessel ports` (protocol 11, ADR
//! 0031) against an in-process node whose terminals run `/bin/sh`. Unix
//! only (the shell's commands).

use std::path::Path;
use std::process::Command;

use vessel_harness::Lookup;
use vessel_node::Shell;

use crate::support::{Fixture, within};

/// A node with `/bin/sh` for terminals and the machine's git.
pub(super) async fn start() -> Fixture {
    let git = which::which("git").expect("the tests need git on PATH");
    let git_folder = git.parent().unwrap().to_path_buf();
    Fixture::start_with(|config, _| {
        config.terminals.shell = Some(Shell::plain("/bin/sh"));
        let dirs = config.harness_lookup.dirs().to_vec();
        config.harness_lookup = Lookup::new(dirs.into_iter().chain([git_folder]));
    })
    .await
}

/// A repository with one commit and a script in its `vessel.toml`.
fn repository(dir: &Path) {
    let git = |args: &[&str]| {
        let status = Command::new("git")
            .args([
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.invalid",
            ])
            .args([
                "-c",
                "commit.gpgsign=false",
                "-c",
                "init.defaultBranch=main",
            ])
            .args(args)
            .current_dir(dir)
            .status()
            .unwrap();
        assert!(status.success(), "git {args:?}");
    };
    git(&["init", "-q"]);
    std::fs::write(
        dir.join("vessel.toml"),
        "[scripts.hello]\ncommand = 'echo hello-$VESSEL_SCRIPT'\n",
    )
    .unwrap();
    git(&["add", "vessel.toml"]);
    git(&["commit", "-q", "-m", "first"]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_shell_is_opened_listed_typed_into_and_closed() {
    within(async {
        let fixture = start().await;
        let session = fixture.run_detached().await;

        let opened = fixture
            .vessel(
                &[
                    "shell",
                    "--session",
                    &session,
                    "--title",
                    "build",
                    "--detached",
                ],
                b"",
            )
            .await;
        assert!(opened.ok, "{}", opened.stderr);
        let terminal = opened.stdout.split_whitespace().next().unwrap().to_owned();
        assert!(opened.stdout.contains("shell"), "{}", opened.stdout);

        let listed = fixture
            .vessel(&["shell", "list", "--session", &session], b"")
            .await;
        assert!(listed.ok, "{}", listed.stderr);
        assert!(listed.stdout.contains(&terminal), "{}", listed.stdout);
        assert!(listed.stdout.contains("\"build\""), "{}", listed.stdout);

        // Attached with piped input: the keys go in, and the shell's exit
        // ends the attach.
        let attached = fixture
            .vessel(
                &["shell", "attach", terminal.get(..8).unwrap()],
                b"echo cli-$((2*21))\nexit 3\n",
            )
            .await;
        assert!(attached.ok, "{}", attached.stderr);
        assert!(attached.stdout.contains("cli-42"), "{}", attached.stdout);
        assert!(
            attached.stderr.contains("[vessel] terminal exited"),
            "{}",
            attached.stderr
        );

        let closed = fixture.vessel(&["shell", "close", &terminal], b"").await;
        assert!(closed.ok, "{}", closed.stderr);
        let gone = fixture.vessel(&["shell", "attach", &terminal], b"").await;
        assert!(!gone.ok);
        assert!(gone.stderr.contains("no terminal"), "{}", gone.stderr);

        let ports = fixture.vessel(&["ports"], b"").await;
        assert!(ports.ok, "{}", ports.stderr);
        fixture.stop().await;
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_script_runs_once_its_set_is_approved() {
    within(async {
        let fixture = start().await;
        let folder = tempfile::tempdir().unwrap();
        let root = std::fs::canonicalize(folder.path()).unwrap().join("app");
        std::fs::create_dir(&root).unwrap();
        repository(&root);
        let added = fixture
            .vessel(&["project", "add", root.to_str().unwrap()], b"")
            .await;
        assert!(added.ok, "{}", added.stderr);

        let listed = fixture
            .vessel(&["script", "list", "--project", "app"], b"")
            .await;
        assert!(listed.ok, "{}", listed.stderr);
        assert!(
            listed
                .stdout
                .contains("script hello: echo hello-$VESSEL_SCRIPT"),
            "{}",
            listed.stdout
        );
        assert!(listed.stdout.contains("approved: no"), "{}", listed.stdout);

        let refused = fixture
            .vessel(&["script", "run", "hello", "--project", "app"], b"")
            .await;
        assert!(!refused.ok);
        assert!(
            refused.stderr.contains("not approved"),
            "{}",
            refused.stderr
        );

        let digest = listed
            .stdout
            .lines()
            .find_map(|line| line.strip_prefix("digest: "))
            .unwrap()
            .to_owned();
        let approved = fixture
            .vessel(&["project", "approve", "app", &digest], b"")
            .await;
        assert!(approved.ok, "{}", approved.stderr);

        let run = fixture
            .vessel(&["script", "run", "hello", "--project", "app"], b"")
            .await;
        assert!(run.ok, "{}", run.stderr);
        assert!(run.stdout.contains("hello-hello"), "{}", run.stdout);
        assert!(
            run.stderr.contains("[vessel] terminal exited"),
            "{}",
            run.stderr
        );
        fixture.stop().await;
    })
    .await;
}

#[cfg(target_os = "linux")]
mod interrupts;
