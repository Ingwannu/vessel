//! The test-only launcher shares real admission; the shipped binary has
//! no fake-forge argument or environment selector, even with the feature on.
#![cfg(test)]

use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use serde::Serialize;
use tokio::io::{AsyncBufReadExt as _, AsyncReadExt as _, BufReader};
use tokio::process::{Child, Command};
use vessel_client::NodeClient;
use vessel_forge::fake;
use vessel_node::DataDir;
use vessel_proto::forge::{
    ForgeAccess, ForgeKind, GetForgeStatus, PullNumber, PullRequest, RepoRef,
};
use vessel_proto::{ClientInfo, StopWhen};

#[derive(Serialize)]
struct File {
    version: u16,
    repositories: Vec<Repository>,
}
#[derive(Serialize)]
struct Repository {
    repo: RepoRef,
    pulls: Vec<Pull>,
}
#[derive(Serialize)]
struct Pull {
    pull: PullRequest,
}

fn seed(root: &Path) -> std::path::PathBuf {
    let repo = RepoRef {
        kind: ForgeKind::Github,
        host: "github.com".parse().unwrap(),
        path: "fixture/example".parse().unwrap(),
    };
    let mut read = fake::pull(
        &repo,
        PullNumber::new(1).unwrap(),
        &"fixture/branch".parse().unwrap(),
        &"a".repeat(40).parse().unwrap(),
    )
    .unwrap();
    // The reusable helper starts unspecified facts; a file must carry an
    // agreeing summary and whole result before the decoder can admit it.
    read.pull.summary.checks = read.pull.checks.state;
    let bytes = serde_json::to_vec(&File {
        version: 1,
        repositories: vec![Repository {
            repo,
            pulls: vec![Pull { pull: read.pull }],
        }],
    })
    .unwrap();
    let path = root.join("seed.json");
    std::fs::write(&path, bytes).unwrap();
    path
}

#[derive(Clone, Copy)]
enum Mode {
    Product,
    #[cfg(feature = "forge-fixture")]
    Fixture,
}

async fn launch(root: &Path, seed: &Path, mode: Mode) -> (Child, NodeClient) {
    let home = root.join("home");
    let programs = root.join("programs");
    let data = root.join("data");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&programs).unwrap();
    let socket = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = socket.local_addr().unwrap();
    drop(socket);
    let binary = match mode {
        Mode::Product => env!("CARGO_BIN_EXE_vesseld"),
        #[cfg(feature = "forge-fixture")]
        Mode::Fixture => env!("CARGO_BIN_EXE_vessel-forge-fixture"),
    };
    let mut cmd = Command::new(binary);
    let _configured = cmd
        .args(["--bind", &address.to_string(), "--data-dir"])
        .arg(&data)
        .env_clear()
        .env("HOME", &home)
        .env("USERPROFILE", &home)
        .env("PATH", &programs)
        .env("VESSEL_FORGE_SEED", seed)
        .kill_on_drop(true)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    match mode {
        Mode::Product => {
            let _configured = cmd.arg("--detached");
        }
        #[cfg(feature = "forge-fixture")]
        Mode::Fixture => {
            let _configured = cmd
                .arg("--seed")
                .arg(seed)
                .arg("--home")
                .arg(&home)
                .arg("--programs")
                .arg(&programs);
        }
    }
    let mut child = cmd.spawn().unwrap();
    let mut reader = BufReader::new(child.stdout.take().unwrap());
    let mut ready = String::new();
    let read = tokio::time::timeout(Duration::from_secs(20), reader.read_line(&mut ready))
        .await
        .unwrap()
        .unwrap();
    if read == 0 {
        let mut errors = String::new();
        if let Some(stderr) = child.stderr.take() {
            let _bytes = stderr
                .take(16_384)
                .read_to_string(&mut errors)
                .await
                .unwrap();
        }
        // Only the launcher's safe Display category, never arbitrary node
        // logs or a parser's source chain, belongs in a failed assertion.
        let diagnosis = errors
            .lines()
            .find(|line| line.contains("forge fixture refused"))
            .unwrap_or("child exited without a fixture startup diagnostic");
        panic!("child closed before readiness: {diagnosis}");
    }
    let (marker, value) = ready.trim().split_once(' ').unwrap();
    match mode {
        Mode::Product => {
            assert_eq!(marker, vessel_node::attach::READY);
            assert_eq!(value.parse::<u32>().unwrap(), child.id().unwrap());
        }
        #[cfg(feature = "forge-fixture")]
        Mode::Fixture => {
            assert_eq!(marker, "FIXTURE_READY");
            assert_eq!(value, address.to_string());
        }
    }
    let token = DataDir::new(data)
        .read_token()
        .unwrap()
        .expose()
        .parse()
        .unwrap();
    let info = ClientInfo {
        name: "fixture-launch-test".to_owned(),
        version: "0".to_owned(),
    };
    let client = NodeClient::connect(&address.to_string().parse().unwrap(), &token, info)
        .await
        .unwrap();
    (child, client)
}

async fn stop(mut child: Child, client: NodeClient) {
    client.stop_node(StopWhen::Now).await.unwrap();
    assert!(
        tokio::time::timeout(Duration::from_secs(20), child.wait())
            .await
            .unwrap()
            .unwrap()
            .success()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn product_binary_rejects_fixture_flags_and_ignores_a_seed_environment() {
    Box::pin(vessel_testkit::within(Duration::from_secs(90), async {
        let root = tempfile::tempdir().unwrap();
        let seed = seed(root.path());
        let refused = Command::new(env!("CARGO_BIN_EXE_vesseld"))
            .arg("--seed")
            .arg(&seed)
            .output()
            .await
            .unwrap();
        assert!(!refused.status.success());
        assert!(refused.stdout.is_empty());
        let (child, client) = launch(root.path(), &seed, Mode::Product).await;
        let status = client.call(GetForgeStatus {}).await.unwrap();
        assert!(
            status
                .hosts
                .iter()
                .all(|host| !matches!(host.access, ForgeAccess::Ready { .. }))
        );
        stop(child, client).await;
    }))
    .await
    .expect("fixture product test timed out");
}

#[cfg(feature = "forge-fixture")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn fixture_binary_loads_real_fake_without_gh_or_token_discovery() {
    Box::pin(vessel_testkit::within(Duration::from_secs(90),async {
        let root=tempfile::tempdir().unwrap();let seed=seed(root.path());
        let (child,client)=launch(root.path(),&seed,Mode::Fixture).await;
        let status=client.call(GetForgeStatus{}).await.unwrap();
        assert!(status.hosts.iter().any(|host|matches!(&host.access,ForgeAccess::Ready{login:Some(login)} if login.as_str()=="vessel-test")));
        stop(child,client).await;
    })).await.expect("fixture custom provider test timed out");
}

#[cfg(feature = "forge-fixture")]
#[tokio::test]
async fn malformed_seed_diagnostic_never_quotes_its_content() {
    let root = tempfile::tempdir().unwrap();
    let file = root.path().join("bad.json");
    let marker = "PRIVATE_SEED_SENTINEL";
    std::fs::write(&file, format!("{{\"{marker}\":true}}")).unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_vessel-forge-fixture"));
    let _configured = command
        .arg("--seed")
        .arg(file)
        .arg("--home")
        .arg(root.path())
        .arg("--data-dir")
        .arg(root.path().join("data"))
        .arg("--programs")
        .arg(root.path())
        .args(["--bind", "127.0.0.1:0"])
        .kill_on_drop(true);
    let output = tokio::time::timeout(Duration::from_secs(20), command.output())
        .await
        .expect("malformed seed startup did not refuse")
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(!String::from_utf8_lossy(&output.stderr).contains(marker));
}

#[cfg(feature = "forge-fixture")]
#[tokio::test]
async fn fixture_refuses_external_bind_or_advertisement_before_node_start() {
    for (bind, public) in [
        ("0.0.0.0:0", None),
        ("127.0.0.1:0", Some("ws://forge.example.invalid:9000")),
    ] {
        let root = tempfile::tempdir().unwrap();
        let file = seed(root.path());
        let data = root.path().join("must-not-start");
        let mut command = Command::new(env!("CARGO_BIN_EXE_vessel-forge-fixture"));
        let _configured = command
            .arg("--seed")
            .arg(file)
            .arg("--home")
            .arg(root.path())
            .arg("--programs")
            .arg(root.path())
            .arg("--data-dir")
            .arg(&data)
            .args(["--bind", bind])
            .kill_on_drop(true);
        if let Some(public) = public {
            let _configured = command.args(["--public", public]);
        }
        let output = tokio::time::timeout(Duration::from_secs(20), command.output())
            .await
            .expect("external fixture configuration did not refuse")
            .unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("loopback only"));
        assert!(
            !data.exists(),
            "node state was created before refusing external exposure"
        );
    }
}
