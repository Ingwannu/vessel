//! The fixture supplies causal barriers; these helpers never signal observed PIDs.

use std::net::SocketAddr;
use std::path::Path;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt as _, AsyncReadExt as _};
use vessel_local_node::{Launch, NodeHome};
use vessel_proc::{Child, CommandSpec, ProcessId};
use vessel_proto::{ClientInfo, NodeId, local_launch::Ready};

use super::control;

pub(super) const SAFETY_NET: Duration = Duration::from_secs(60);
const OBSERVE_PAUSE: Duration = Duration::from_millis(10);

type Socket = tokio::net::TcpStream;
pub(super) type Fixture =
    vessel_rpc::Framed<tokio::io::ReadHalf<Socket>, tokio::io::WriteHalf<Socket>>;

// Kept separate from the framing type so every connection still uses the same
// bounded codec, without exporting any fixture API from the controller crate.
pub(super) async fn accept(listener: &tokio::net::TcpListener) -> Fixture {
    let (stream, _) = listener.accept().await.unwrap();
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

pub(super) fn ordinary_recipe(root: &Path) -> Launch {
    launch(root, Path::new(env!("CARGO_BIN_EXE_vesseld")))
}

pub(super) fn recipe(root: &Path, control: SocketAddr) -> Launch {
    std::fs::create_dir_all(root.join("user-home")).unwrap();
    std::fs::create_dir_all(root.join("programs")).unwrap();
    launch(
        root,
        Path::new(env!("CARGO_BIN_EXE_vessel-local-node-fixture")),
    )
    .environment(|command| {
        command
            .env(control::CONTROL_ENV, control.to_string())
            .env(control::HOME_ENV, root.join("user-home"))
            .env(control::PROGRAMS_ENV, root.join("programs"))
    })
    .unwrap()
}

pub(super) async fn replacement(root: &Path) -> Child {
    let command = environment(
        CommandSpec::new(env!("CARGO_BIN_EXE_vesseld"))
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
    let _read = stdout.read_line(&mut line).await.unwrap();
    let _ready = Ready::decode(&line).unwrap();
    child
}

pub(super) async fn client(root: &Path, node: NodeId) -> vessel_client::NodeClient {
    let directory = vessel_store::DataDir::new(root.join("node"));
    let token = directory.read_token().unwrap();
    let credential = vessel_client::LocalCredential::Token(token.expose().parse().unwrap());
    let client = vessel_client::NodeClient::connect_local(
        &directory.local_endpoint(node),
        &credential,
        info(),
    )
    .await
    .unwrap();
    assert_eq!(client.node().id, node);
    client
}

pub(super) async fn zombie(process: ProcessId) {
    let path = format!("/proc/{process}/stat");
    loop {
        let stat = std::fs::read_to_string(&path).unwrap();
        let (_, fields) = stat.rsplit_once(')').unwrap();
        if fields.split_whitespace().next() == Some("Z") {
            return;
        }
        tokio::time::sleep(OBSERVE_PAUSE).await;
    }
}

pub(super) async fn reaped(process: ProcessId) {
    let path = format!("/proc/{process}");
    while std::fs::exists(&path).unwrap() {
        tokio::time::sleep(OBSERVE_PAUSE).await;
    }
}
