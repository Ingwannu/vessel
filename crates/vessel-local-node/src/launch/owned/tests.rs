//! Release refusal must return authority to the controller rather than an error.

use super::*;
use std::io::{Read as _, Write as _};
use tokio::io::AsyncWriteExt as _;
use vessel_proc::{CommandSpec, spawn};

const SAFETY_NET: Duration = Duration::from_secs(30);
const READY: &str = "owned fixture ready\n";
const FIXTURE_ENV: &str = "VESSEL_LOCAL_NODE_KEEP_REFUSAL_FIXTURE";

#[test]
fn bound_child_fixture() {
    if std::env::var_os(FIXTURE_ENV).is_none() {
        return;
    }
    let mut stdout = std::io::stdout().lock();
    stdout.write_all(READY.as_bytes()).unwrap();
    stdout.flush().unwrap();
    let mut stdin = std::io::stdin().lock();
    let mut byte = [0_u8; 1];
    while stdin.read_exact(&mut byte).is_ok() {
        assert_eq!(byte, [b'?']);
        stdout.write_all(b"!").unwrap();
        stdout.flush().unwrap();
    }
}

#[tokio::test]
async fn refused_keep_preserves_the_live_owner_until_explicit_drop() {
    tokio::time::timeout(SAFETY_NET, async {
        let command = CommandSpec::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "launch::owned::tests::bound_child_fixture",
                "--nocapture",
            ])
            .env(FIXTURE_ENV, "1");
        let Spawned {
            child,
            mut stdin,
            stdout,
            stderr,
        } = spawn(&command).unwrap();
        let authority = child.lifetime();
        let owned = Owned {
            child,
            logs: LogReader::collect(stderr),
        };
        let mut output = BufReader::new(stdout);
        loop {
            let mut line = String::new();
            assert_ne!(output.read_line(&mut line).await.unwrap(), 0);
            if line.ends_with(READY) {
                break;
            }
        }
        let retained = match owned.keep() {
            Ok(_kept) => panic!("ordinary Bound child must refuse release"),
            Err(owned) => owned,
        };
        assert!(authority.is_live());
        stdin.write_all(b"?").await.unwrap();
        let mut reply = [0_u8; 1];
        let _read = output.read_exact(&mut reply).await.unwrap();
        assert_eq!(reply, [b'!']);
        drop(retained);
        authority.ended().await;
        assert!(!authority.is_live());
        let mut rest = Vec::new();
        let _eof = output.read_to_end(&mut rest).await.unwrap();
        assert!(rest.is_empty());
    })
    .await
    .unwrap();
}
