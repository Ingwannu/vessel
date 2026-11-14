//! Kill a real node process after its active turn is durably acknowledged.
use super::{fixture, gated::Gate, policy, running, started, text};
use crate::support::{agent_of, client, read_all, read_until, step, within};
use std::io::Write as _;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt as _, AsyncWriteExt as _, BufReader};
use vessel_client::{NodeClient, NodeUrl};
use vessel_harness::{HarnessEnv, Lookup};
use vessel_node::{Config, DataDir, Node};
use vessel_proto::{AgentEvent, Delivery};
const CHILD: &str = "VESSEL_RECOVERY_KILL_CHILD";
const NAME: &str = "restart::crash::a_killed_node_resumes_only_its_committed_active_turn";

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
    within(async {
        let f = fixture();
        let mut command = tokio::process::Command::new(std::env::current_exe().unwrap());
        let _configured = command
            .args(["--exact", NAME, "--nocapture", "--test-threads=1"])
            .env(CHILD, "1")
            .env("VESSEL_RECOVERY_TEST_DATA", f.data_path())
            .env("VESSEL_RECOVERY_TEST_HOME", f.user_home())
            .env("VESSEL_RECOVERY_TEST_BIN", f.bin())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .kill_on_drop(true);
        let mut process = command.spawn().unwrap();
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
        let owner = NodeClient::connect(&url, &f.token(), client())
            .await
            .unwrap();
        policy(&owner, true, Some("loaded")).await;
        let id = running(&owner, &f).await;
        assert_eq!(
            owner
                .prompt(id, text("wait"), Delivery::Start)
                .await
                .unwrap(),
            vessel_proto::Delivered::Started
        );
        let original = owner.session(id).await.unwrap();
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
        let gate = Gate::install(&f).await;
        let node = f.start().await;
        let owner = f.connect(&node).await;
        let (loaded, mut held) = gate.arrive().await;
        assert_eq!(loaded, original.harness_session.as_ref().unwrap().as_str());
        let mut stream = owner.follow(id, original.next_seq).await.unwrap();
        let mut seen = Vec::new();
        held.write_all(&[1]).await.unwrap();
        drop(held);
        read_until(&mut stream, &mut seen, |record| started(record, "loaded")).await;
        read_until(&mut stream, &mut seen, |record| {
            matches!(agent_of(record), Some(AgentEvent::TurnEnded { .. }))
        })
        .await;
        assert_eq!(
            owner.session(id).await.unwrap().harness_session,
            original.harness_session
        );
        assert_eq!(
            read_all(&owner, id)
                .await
                .iter()
                .filter(|record| started(record, "loaded"))
                .count(),
            1
        );
        node.stop().await.unwrap();
    })
    .await;
}
