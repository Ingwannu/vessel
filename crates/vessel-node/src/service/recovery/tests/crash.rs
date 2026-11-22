//! Kill a real node process after its active turn is durably acknowledged.
use super::{CASE_DEADLINE, Fixture, done};
use crate::{Config, DataDir, Node};
use std::io::Write as _;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt as _, AsyncWriteExt as _, BufReader, Lines};
use tokio::process::{Child, ChildStdout};
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
    // Only the test's private stdin pipe asks for an observation; all agent
    // inputs still travel through the ordinary authenticated node client.
    let request = tokio::task::spawn_blocking(|| {
        serde_json::from_reader::<_, CommitRequest>(std::io::stdin().lock())
    })
    .await
    .unwrap()
    .unwrap();
    let info = super::committed(&node, request.session, request.through).await;
    writeln!(
        std::io::stdout().lock(),
        "RECOVERY_CHILD_COMMITTED {}",
        serde_json::to_string(&info).unwrap()
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
                .write_settings(
                    &vessel_store::SavedNodeSettings::default().with_public_settings(
                        SetNodeSettings {
                            restart_recovery: Some(vessel_proto::restart::SetRestartRecovery {
                                enabled: Some(true),
                                prompt: Some("loaded".parse().unwrap()),
                            }),
                            ..SetNodeSettings::default()
                        },
                    ),
                )
                .unwrap();
            let mut process = child_command(&f).spawn().unwrap();
            let mut lines = BufReader::new(process.stdout.take().unwrap()).lines();
            let url: NodeUrl = child_message(&mut lines, "RECOVERY_CHILD_READY ").await;
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
            let (session, through) = f.start_turn(&owner).await;
            let original = commit_in_child(&mut process, &mut lines, session, through).await;
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

#[derive(serde::Serialize, serde::Deserialize)]
struct CommitRequest {
    session: vessel_proto::SessionId,
    through: EventSeq,
}
struct Committed(vessel_proto::SessionInfo);
impl std::str::FromStr for Committed {
    type Err = serde_json::Error;
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(text).map(Self)
    }
}
async fn child_message<T>(lines: &mut Lines<BufReader<ChildStdout>>, prefix: &str) -> T
where
    T: std::str::FromStr,
    T::Err: std::fmt::Debug,
{
    loop {
        let line = step(prefix, lines.next_line())
            .await
            .unwrap()
            .expect("the test node exited before its observation");
        if let Some((_, value)) = line.split_once(prefix) {
            return value.parse().unwrap();
        }
    }
}
async fn commit_in_child(
    process: &mut Child,
    lines: &mut Lines<BufReader<ChildStdout>>,
    session: vessel_proto::SessionId,
    through: EventSeq,
) -> vessel_proto::SessionInfo {
    let request = serde_json::to_vec(&CommitRequest { session, through }).unwrap();
    let mut stdin = process.stdin.take().unwrap();
    step("request the real store commit", stdin.write_all(&request))
        .await
        .unwrap();
    drop(stdin);
    let Committed(info) = child_message(lines, "RECOVERY_CHILD_COMMITTED ").await;
    assert_eq!(info.id, session);
    assert!(info.next_seq >= through);
    info
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
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .kill_on_drop(true);
    command
}
