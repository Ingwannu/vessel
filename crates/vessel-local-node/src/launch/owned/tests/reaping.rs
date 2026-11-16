//! A fresh ignored-SIGCHLD embedding must not invalidate retained PID evidence.

use super::*;
use std::process::{Command as StdCommand, Stdio};
use std::time::Instant;
use tokio::io::AsyncReadExt as _;

const FIXTURE: &str = "VESSEL_IGNORED_CHILD_REAPING_FIXTURE";
const VERIFIED: &str = "managed-child-reaping-verified";
const PROBE_DEADLINE: Duration = Duration::from_secs(5);
const PROBE_PAUSE: Duration = Duration::from_millis(10);
const MAX_FIXTURE_BYTES: u64 = 8 * 1024;
const ECHILD: i32 = 10;

fn short_child(root: &std::path::Path) -> std::process::Child {
    StdCommand::new("/bin/sh")
        .args(["-c", "exit 0"])
        .env_clear()
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap()
}

fn observe(process: u32, wanted: Option<&str>) {
    let path = format!("/proc/{process}/stat");
    let started = Instant::now();
    loop {
        match std::fs::read_to_string(&path) {
            Ok(stat) if wanted.is_some() => {
                let (_, fields) = stat.rsplit_once(')').unwrap();
                if fields.split_whitespace().next() == wanted {
                    return;
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound && wanted.is_none() => {
                return;
            }
            Ok(_) => {}
            Err(error) => panic!("known child observation failed: {error}"),
        }
        assert!(
            started.elapsed() < PROBE_DEADLINE,
            "known child did not reach expected kernel state"
        );
        std::thread::sleep(PROBE_PAUSE);
    }
}

#[test]
fn inherited_ignored_sigchld_fixture() {
    if std::env::var_os(FIXTURE).is_none() {
        return;
    }
    let root = tempfile::tempdir().unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        // No explicit child-signal registration yet: prove the inherited ignore
        // really auto-reaps, rather than trusting the shell command's spelling.
        let mut baseline = short_child(root.path());
        observe(baseline.id(), None);
        assert_eq!(
            baseline.try_wait().unwrap_err().raw_os_error(),
            Some(ECHILD)
        );

        let _retained_registration = register_reaping().unwrap();
        let mut retained = short_child(root.path());
        observe(retained.id(), Some("Z"));
        assert!(retained.wait().unwrap().success());
        observe(retained.id(), None);
        println!("{VERIFIED}");
    });
}

#[tokio::test]
async fn ignored_sigchld_is_replaced_before_pid_based_ownership_evidence() {
    tokio::time::timeout(SAFETY_NET, async {
        let root = tempfile::tempdir().unwrap();
        let command = CommandSpec::new("/bin/sh")
            .args(["-c", "trap '' CHLD; exec \"$@\"", "managed-reaping-fixture"])
            .arg(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "launch::owned::tests::reaping::inherited_ignored_sigchld_fixture",
                "--nocapture",
                "--test-threads=1",
            ])
            .env_clear()
            .env(FIXTURE, "1")
            .cwd(root.path());
        let Spawned {
            mut child,
            stdin,
            stdout,
            mut stderr,
        } = spawn(&command).unwrap();
        drop(stdin);
        let output = tokio::spawn(async move {
            let mut bytes = Vec::new();
            let _read = stdout
                .take(MAX_FIXTURE_BYTES)
                .read_to_end(&mut bytes)
                .await
                .unwrap();
            bytes
        });
        let errors = tokio::spawn(async move {
            let _drained = tokio::io::copy(&mut stderr, &mut tokio::io::sink())
                .await
                .unwrap();
        });
        let exit = child.wait_checked().await.unwrap();
        let output = String::from_utf8(output.await.unwrap()).unwrap();
        errors.await.unwrap();
        assert_eq!(exit.code(), Some(0));
        assert!(
            output.contains(VERIFIED),
            "the exact fixture assertions did not run"
        );
    })
    .await
    .unwrap();
}
