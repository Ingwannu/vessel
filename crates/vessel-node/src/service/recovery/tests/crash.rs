//! Kill a real node process after its active turn is durably acknowledged.
use super::{CASE_DEADLINE, Fixture, done};
use crate::{Config, DataDir, Node};
use std::io::Write as _;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt as _, BufReader};
use vessel_client::{NodeClient, NodeUrl};
use vessel_harness::{HarnessEnv, Lookup};
use vessel_proto::{AgentEvent, EventSeq, SessionEvent};

const CHILD: &str = "VESSEL_RECOVERY_KILL_CHILD";
const NAME: &str =
    "service::recovery::tests::crash::a_killed_node_resumes_only_its_committed_active_turn";

async fn child() {
    let path = |name| PathBuf::from(std::env::var_os(name).unwrap());
    let mut config = Config::new(
        DataDir::new(path("VESSEL_RECOVERY_TEST_DATA")),
        HarnessEnv::new(path("VESSEL_RECOVERY_TEST_HOME")),
    );
    config.bind = Some("127.0.0.1:0".parse().unwrap());
    config.harness_lookup = Lookup::new([path("VESSEL_RECOVERY_TEST_BIN")]);
    let node = Node::start(config).await.unwrap();
    writeln!(
        std::io::stdout().lock(),
        "RECOVERY_CHILD_READY {}",
        node.address().unwrap()
    )
    .unwrap();
    std::future::pending::<()>().await;
}
/// Own only the fake provider children observed under this test's node.
/// A start identity and executable check prevent cleanup of a reused PID.
struct Providers(Vec<(u32, String, PathBuf)>);
fn identity(pid: u32) -> Option<String> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    stat.rsplit_once(") ")?
        .1
        .split_whitespace()
        .nth(19)
        .map(str::to_owned)
}
impl Providers {
    fn capture(parent: u32) -> Self {
        let expected = vessel_testkit::Fake::Acp
            .path()
            .unwrap()
            .canonicalize()
            .unwrap();
        let mut found = Vec::new();
        for task in std::fs::read_dir(format!("/proc/{parent}/task")).unwrap() {
            let task = task.unwrap();
            let Ok(children) = std::fs::read_to_string(task.path().join("children")) else {
                continue;
            };
            for pid in children
                .split_whitespace()
                .filter_map(|pid| pid.parse::<u32>().ok())
            {
                let Ok(executable) = std::fs::read_link(format!("/proc/{pid}/exe")) else {
                    continue;
                };
                if executable == expected
                    && let Some(started) = identity(pid)
                {
                    found.push((pid, started, executable));
                }
            }
        }
        assert_eq!(
            found.len(),
            1,
            "one active fake provider belongs to this test node"
        );
        Self(found)
    }
}
impl Drop for Providers {
    fn drop(&mut self) {
        for (pid, started, executable) in &self.0 {
            if identity(*pid).as_ref() == Some(started)
                && std::fs::read_link(format!("/proc/{pid}/exe")).ok().as_ref() == Some(executable)
            {
                let status = std::process::Command::new("kill")
                    .args(["-KILL", "--"])
                    .arg(pid.to_string())
                    .status()
                    .unwrap();
                // It may have exited from stdin EOF between verification and
                // signal. No other process can have the captured start id.
                if !status.success() {
                    assert!(identity(*pid).as_ref() != Some(started));
                }
            }
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_killed_node_resumes_only_its_committed_active_turn() {
    if std::env::var_os(CHILD).is_some() {
        child().await;
        return;
    }
    vessel_testkit::within(
        CASE_DEADLINE,
        Box::pin(async {
            let f = Fixture::new();
            f.data.prepare().unwrap();
            f.data
                .write_settings_record(&vessel_store::SavedNodeSettings {
                    restart_recovery: Some(vessel_proto::restart::SetRestartRecovery {
                        enabled: Some(true),
                        prompt: Some("loaded".parse().unwrap()),
                    }),
                    ..vessel_store::SavedNodeSettings::default()
                })
                .unwrap();
            let mut process = child_command(&f).spawn().unwrap();
            let mut lines = BufReader::new(process.stdout.take().unwrap()).lines();
            let url = loop {
                let line = step("child node becomes ready", lines.next_line())
                    .await
                    .unwrap()
                    .expect("the child exited before ready");
                if let Some((_, address)) = line.split_once("RECOVERY_CHILD_READY ") {
                    break address.parse::<NodeUrl>().unwrap();
                }
            };
            let token = f.data.read_token().unwrap().expose().parse().unwrap();
            let owner = NodeClient::connect(
                &url,
                &token,
                vessel_proto::ClientInfo {
                    name: "recovery-child-test".into(),
                    version: "0".into(),
                },
            )
            .await
            .unwrap();
            let original = f.active(&owner).await;
            let id = original.id;
            let providers = Providers::capture(process.id().unwrap());
            process.start_kill().unwrap();
            assert!(
                !step(
                    "the node is killed without graceful shutdown",
                    process.wait()
                )
                .await
                .unwrap()
                .success()
            );
            drop(providers);
            drop(owner);
            drop(lines);
            let node = f.start().await;
            let owner = f.client(&node).await;
            done(&node).await;
            let mut stream = owner.follow(id, original.next_seq).await.unwrap();
            loop {
                let record = stream.next().await.unwrap().unwrap();
                if matches!(
                    record.event,
                    SessionEvent::Agent {
                        event: AgentEvent::TurnEnded { .. }
                    }
                ) {
                    break;
                }
            }
            assert_eq!(
                owner.session(id).await.unwrap().harness_session,
                original.harness_session
            );
            assert_eq!(
                owner
                    .read(id, EventSeq::ZERO, super::EVENT_PAGE)
                    .await
                    .unwrap()
                    .records
                    .iter()
                    .filter(|record| super::restart::continuation(record))
                    .count(),
                1
            );
            node.stop().await.unwrap();
        }),
    )
    .await
    .unwrap();
}

async fn step<T>(label: &str, work: impl Future<Output = T>) -> T {
    tokio::time::timeout(super::WAIT, work)
        .await
        .unwrap_or_else(|_| panic!("recovery crash step timed out: {label}"))
}

fn child_command(f: &Fixture) -> tokio::process::Command {
    let mut command = tokio::process::Command::new(std::env::current_exe().unwrap());
    let _configured = command
        .args(["--exact", NAME, "--nocapture", "--test-threads=1"])
        .env(CHILD, "1")
        .env("VESSEL_RECOVERY_TEST_DATA", f.data.root())
        .env("VESSEL_RECOVERY_TEST_HOME", &f.home)
        .env("VESSEL_RECOVERY_TEST_BIN", &f.bin)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .kill_on_drop(true);
    command
}
