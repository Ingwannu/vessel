//! A separate launcher exits while the parent probes the retained exact child.
//! The TCP control fixture works on Windows too: Linux results do not stand in
//! for Windows execution, but the same case can qualify job close behavior there.

#![cfg(test)]

use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt as _, AsyncReadExt as _, AsyncWriteExt as _, BufReader};
use vessel_proc::{CommandSpec, EnvValue, Spawned, spawn, spawn_releasable};

const SAFETY_NET: Duration = Duration::from_secs(30);

#[test]
fn release_launcher_fixture() {
    let Some(mode) = std::env::var_os("VESSEL_PROC_LAUNCHER_FIXTURE") else {
        return;
    };
    let address = std::env::var_os("VESSEL_PROC_CONTROL_ADDRESS").unwrap();
    let binary = std::env::var_os("VESSEL_PROC_TEST_FAKE").unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async move {
        let mut command = CommandSpec::new(binary);
        command.env.extend([
            ("FAKE_LOCAL_CONTROL_ADDRESS".into(), EnvValue::Set(address)),
            (
                "FAKE_LOCAL_LIFETIME_FIXTURE".into(),
                EnvValue::Set("1".into()),
            ),
        ]);
        let Spawned { child, stdout, .. } = if mode == "bound" {
            spawn(&command).unwrap()
        } else {
            spawn_releasable(&command).unwrap()
        };
        let mut ready = String::new();
        let _line = BufReader::new(stdout).read_line(&mut ready).await.unwrap();
        assert_eq!(ready, "lifetime ready\n");
        if mode == "detach" {
            let authority = child.lifetime();
            drop(child.detach().unwrap());
            authority.ended().await;
            assert!(!authority.is_live());
        } else {
            drop(child);
        }
    });
    // Runtime and process exit close every launcher-owned handle. A detachable
    // Windows job must survive this without leaking a handle in the launcher.
}

async fn check(mode: &str, survives: bool) {
    vessel_testkit::within(SAFETY_NET, async {
        let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
            .await
            .unwrap();
        let address = listener.local_addr().unwrap();
        let mut launcher = tokio::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "release_launcher_fixture", "--nocapture"])
            .env("VESSEL_PROC_LAUNCHER_FIXTURE", mode)
            .env("VESSEL_PROC_CONTROL_ADDRESS", address.to_string())
            .env(
                "VESSEL_PROC_TEST_FAKE",
                vessel_testkit::Fake::LocalNode.path().unwrap(),
            )
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let (control, _peer) = listener.accept().await.unwrap();
        let mut control = BufReader::new(control);
        let mut started = String::new();
        let _line = control.read_line(&mut started).await.unwrap();
        assert_eq!(started, "started\n");
        assert!(launcher.wait().await.unwrap().success());
        if survives {
            control.get_mut().write_all(b"?").await.unwrap();
            let mut alive = [0_u8; 1];
            let _probe = control.read_exact(&mut alive).await.unwrap();
            assert_eq!(alive, [b'!']);
            control.get_mut().write_all(b"x").await.unwrap();
        }
        let mut rest = Vec::new();
        let _eof = control.read_to_end(&mut rest).await.unwrap();
        assert!(rest.is_empty());
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn detached_child_survives_actual_launcher_exit() {
    check("detach", true).await;
}

#[tokio::test]
async fn ordinary_child_still_dies_when_the_launcher_drops_it() {
    check("bound", false).await;
}

#[tokio::test]
async fn releasable_owned_child_still_dies_before_launcher_exit() {
    check("owned", false).await;
}
