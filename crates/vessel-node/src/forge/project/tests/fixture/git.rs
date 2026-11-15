//! Owned fixture git: a deadline kills and reaps the child instead of
//! blocking an async worker in `std::process::Command::output`. Kill-on-drop
//! also covers cancellation by the enclosing whole-case safety clock.
use super::super::deadline::{PHASE, step};
use std::{ffi::OsString, io, path::Path, process::Output, process::Stdio};
use tokio::{
    io::AsyncReadExt,
    process::{Child, Command},
};

pub(super) async fn run(dir: &Path, args: &[&str], env: &[(String, OsString)]) {
    let mut child = Command::new("git")
        .current_dir(dir)
        .args(args)
        .envs(env.iter().map(|(key, value)| (key, value)))
        .kill_on_drop(true)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let output = bounded_output(&mut child)
        .await
        .unwrap_or_else(|elapsed| panic!("project fixture phase git output: {elapsed}"));
    assert!(
        output.status.success(),
        "git failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

async fn bounded_output(child: &mut Child) -> Result<Output, tokio::time::error::Elapsed> {
    match tokio::time::timeout(PHASE, output(child)).await {
        Ok(output) => Ok(output.unwrap()),
        Err(elapsed) => {
            child.start_kill().unwrap();
            let _reaped = step("git child reap after deadline", child.wait())
                .await
                .unwrap();
            Err(elapsed)
        }
    }
}

async fn output(child: &mut Child) -> io::Result<Output> {
    let mut stdout = child.stdout.take().unwrap();
    let mut stderr = child.stderr.take().unwrap();
    let mut out = Vec::new();
    let mut err = Vec::new();
    let (status, _, _) = tokio::try_join!(
        child.wait(),
        stdout.read_to_end(&mut out),
        stderr.read_to_end(&mut err),
    )?;
    Ok(Output {
        status,
        stdout: out,
        stderr: err,
    })
}

#[tokio::test]
async fn an_owned_child_that_never_finishes_is_killed_and_reaped() {
    use super::super::deadline::{LIMIT, case};
    case(LIMIT, async {
        let mut child = Command::new(vessel_testkit::Fake::Acp.path().unwrap())
            .env_clear()
            .kill_on_drop(true)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        // wait() normally closes stdin. Retain the writer here so the fake
        // really waits for input until bounded_output must kill it.
        let input = child.stdin.take().unwrap();
        assert!(child.try_wait().unwrap().is_none());
        assert!(bounded_output(&mut child).await.is_err());
        assert!(
            child.try_wait().unwrap().is_some(),
            "the child must be reaped"
        );
        drop(input);
    })
    .await;
}
