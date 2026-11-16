//! The fixture supplies causal barriers; these helpers never signal observed PIDs.

use crate::{Launch, NodeHome};
use std::net::SocketAddr;
use std::path::Path;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt as _, AsyncReadExt as _};
use vessel_proc::{Child, CommandSpec, ProcessId};
use vessel_proto::{ClientInfo, NodeId, local_launch::Ready};

use super::control;
use futures_util::{SinkExt as _, StreamExt as _};

pub(super) const SAFETY_NET: Duration = Duration::from_secs(60);
const PHASE: Duration = Duration::from_secs(5);

pub(super) async fn step<F: Future>(name: &str, operation: F) -> F::Output {
    tokio::time::timeout(PHASE, operation)
        .await
        .unwrap_or_else(|error| panic!("{name} timed out: {error}"))
}

type Socket = tokio::net::TcpStream;
pub(super) type Fixture =
    vessel_rpc::Framed<tokio::io::ReadHalf<Socket>, tokio::io::WriteHalf<Socket>>;

// Kept separate from the framing type so every connection still uses the same
// bounded codec, without exporting any fixture API from the controller crate.
pub(super) async fn accept(listener: &tokio::net::TcpListener) -> Fixture {
    let (stream, _) = step("original fixture control accept", listener.accept())
        .await
        .unwrap();
    let (reader, writer) = tokio::io::split(stream);
    Fixture::new(reader, writer, control::FRAME_LIMIT)
}

fn info() -> ClientInfo {
    ClientInfo {
        name: "managed-peer-test".to_owned(),
        version: "test".to_owned(),
    }
}

fn environment(command: CommandSpec, root: &Path) -> CommandSpec {
    command
        .env_clear()
        .env("HOME", root.join("user-home"))
        .env("PATH", root.join("programs"))
}

fn launch(root: &Path, binary: &Path) -> Launch {
    Launch::new(
        binary.to_owned(),
        NodeHome::new(root.join("node")).unwrap(),
        info(),
    )
    .unwrap()
    .environment(|command| environment(command, root))
    .unwrap()
}

pub(super) struct Paths {
    node: std::path::PathBuf,
    fixture: std::path::PathBuf,
}

impl Paths {
    pub(super) fn locate() -> Self {
        Self {
            node: vessel_testkit::vesseld().expect("real same-commit node build unavailable"),
            fixture: vessel_testkit::local_node_fixture()
                .expect("feature fixture build unavailable"),
        }
    }
}

pub(super) fn ordinary_recipe(root: &Path, paths: &Paths) -> Launch {
    launch(root, &paths.node)
}

pub(super) fn recipe(root: &Path, control: SocketAddr, paths: &Paths) -> Launch {
    std::fs::create_dir_all(root.join("user-home")).unwrap();
    std::fs::create_dir_all(root.join("programs")).unwrap();
    launch(root, &paths.fixture)
        .environment(|command| {
            command
                .env(control::CONTROL_ENV, control.to_string())
                .env(control::HOME_ENV, root.join("user-home"))
                .env(control::PROGRAMS_ENV, root.join("programs"))
        })
        .unwrap()
}

/// The sole blocking worker holds the post-Ready identity read until release.
/// Every scenario uses the real child and its real stopped-server barrier.
pub(super) struct HeldLaunch {
    pub(super) attempt: Attempt,
    pub(super) release: std::sync::mpsc::SyncSender<()>,
    pub(super) fixture: Fixture,
    pub(super) process: ProcessId,
    pub(super) expected: NodeId,
}

/// A failed assertion cancels the original startup future and its RAII owner,
/// instead of leaving a detached start task alive until its outer deadline.
pub(super) struct Pending(tokio::task::JoinHandle<Result<crate::LocalNode, crate::LocalNodeError>>);

impl Future for Pending {
    type Output = Result<Result<crate::LocalNode, crate::LocalNodeError>, tokio::task::JoinError>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        std::pin::Pin::new(&mut self.0).poll(cx)
    }
}

impl Drop for Pending {
    fn drop(&mut self) {
        self.0.abort();
    }
}

pub(super) struct Attempt {
    starting: Pending,
    settlement: tokio::sync::oneshot::Receiver<crate::launch::cleanup_witness::Settlement>,
    dropped: tokio::sync::oneshot::Receiver<()>,
}

impl Attempt {
    pub(super) async fn refusal(self) -> crate::LocalNodeError {
        let result = step(
            "original startup refusal after checked cleanup",
            self.starting,
        )
        .await
        .unwrap();
        let refusal = result.unwrap_err();
        let settlement = step("same cleanup owner exact reap witness", self.settlement)
            .await
            .unwrap();
        assert!(matches!(
            settlement,
            crate::launch::cleanup_witness::Settlement::Reaped(_)
        ));
        step("original startup owner drop witness", self.dropped)
            .await
            .unwrap();
        refusal
    }
}

pub(super) async fn held_launch(root: &Path, paths: &Paths) -> HeldLaunch {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let launch = recipe(root, listener.local_addr().unwrap(), paths);
    let (release, held) = std::sync::mpsc::sync_channel(1);
    let (entered, waiting) = tokio::sync::oneshot::channel();
    let (settled, settlement) = tokio::sync::oneshot::channel();
    let (drop_notice, dropped) = tokio::sync::oneshot::channel();
    let starting = Pending(tokio::spawn(crate::launch::cleanup_witness::observe(
        settled,
        drop_notice,
        async move {
            crate::LocalNode::start_observed(launch, move |_logs| {
                // Reserve before Ready, so no timing guess or controller hook is
                // needed to install the replacement before identity/authentication.
                drop(tokio::task::spawn_blocking(move || {
                    let _delivered = entered.send(());
                    let _released = held.recv_timeout(SAFETY_NET);
                }));
            })
            .await
        },
    )));
    step("original startup file worker entered", waiting)
        .await
        .unwrap();
    let mut fixture = accept(&listener).await;
    let frame = step("original fixture ready notice", fixture.next())
        .await
        .unwrap()
        .unwrap();
    let (process, expected) = match serde_json::from_str(&frame).unwrap() {
        control::Notice::Ready { process, node } => (process, node),
        control::Notice::Stopped { .. } => panic!("stop preceded readiness"),
    };
    fixture
        .send(serde_json::to_string(&control::Command::Stop).unwrap())
        .await
        .unwrap();
    let stopped = step("original fixture stopped notice", fixture.next())
        .await
        .unwrap()
        .unwrap();
    assert!(
        matches!(serde_json::from_str(&stopped).unwrap(), control::Notice::Stopped { process: found } if process == found)
    );
    HeldLaunch {
        attempt: Attempt {
            starting,
            settlement,
            dropped,
        },
        release,
        fixture,
        process,
        expected,
    }
}

pub(super) async fn replacement(root: &Path, paths: &Paths) -> Child {
    let command = environment(
        CommandSpec::new(&paths.node)
            .args(["--desktop-managed", "--bind", "none", "--data-dir"])
            .arg(root.join("node")),
        root,
    );
    let vessel_proc::Spawned {
        child,
        stdin,
        stdout,
        mut stderr,
    } = vessel_proc::spawn(&command).unwrap();
    drop(stdin);
    // Drain only the known fixture child's diagnostics; never print credentials.
    drop(tokio::spawn(async move {
        let _drained = tokio::io::copy(&mut stderr, &mut tokio::io::sink()).await;
    }));
    let limit = u64::try_from(vessel_proto::local_launch::MAX_READY_BYTES + 1).unwrap();
    let mut stdout = tokio::io::BufReader::new(stdout.take(limit));
    let mut line = String::new();
    let _read = step("real successor readiness", stdout.read_line(&mut line))
        .await
        .unwrap();
    let _ready = Ready::decode(&line).unwrap();
    child
}

pub(super) async fn client(root: &Path, node: NodeId) -> vessel_client::NodeClient {
    let home = NodeHome::new(root.join("node")).unwrap();
    let identity = crate::node_files::fixture_identity(&home).unwrap().unwrap();
    assert_eq!(identity.node, node);
    let client = vessel_client::NodeClient::connect_local(
        identity.endpoint.path(),
        &identity.credential,
        info(),
    )
    .await
    .unwrap();
    assert_eq!(client.node().id, node);
    client
}

pub(super) async fn zombie(process: ProcessId, exited: &mut tokio::signal::unix::Signal) {
    let path = format!("/proc/{process}/stat");
    step(
        "original fixture exit notification and zombie snapshot",
        async {
            loop {
                let stat = std::fs::read_to_string(&path).unwrap();
                let (_, fields) = stat.rsplit_once(')').unwrap();
                if fields.split_whitespace().next() == Some("Z") {
                    return;
                }
                // Other parallel child exits are possible; only an actual signal
                // permits another snapshot of this exact known original child.
                exited.recv().await.expect("child signal listener ended");
            }
        },
    )
    .await;
}

pub(super) fn reaped(process: ProcessId) {
    // A non-CleanupUnconfirmed failure now returns AFTER the original owner's
    // checked wait. One absence snapshot audits that receipt; no clock orders it.
    assert!(!std::fs::exists(format!("/proc/{process}")).unwrap());
}
