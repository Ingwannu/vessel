//! Real Windows API probes; wine execution is only an approximation of Windows.

#![cfg(windows)]

use futures_util::SinkExt as _;
use std::time::Duration;
use tokio::io::AsyncWriteExt as _;
use tokio::net::windows::named_pipe::{ClientOptions, NamedPipeClient, ServerOptions};
use vessel_node::{Config, DataDir, DesktopBootstrapError, Node, NodeError};
use vessel_proto::local_launch::{BootstrapChallenge, BootstrapPipe, CHALLENGE_BYTES};

const SAFETY_NET: Duration = Duration::from_secs(40);
const NO_CONNECTION: Duration = Duration::from_millis(100);

fn entropy() -> [u8; CHALLENGE_BYTES] {
    let mut bytes = [0; CHALLENGE_BYTES];
    getrandom::fill(&mut bytes).unwrap();
    bytes
}

fn config(root: &std::path::Path, pipe: BootstrapPipe) -> Config {
    let home = root.join("user-home");
    let programs = root.join("programs");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&programs).unwrap();
    let mut config = Config::new(
        DataDir::new(root.join("node")),
        vessel_harness::HarnessEnv::new(home),
    );
    config.harness_lookup = vessel_harness::Lookup::new([programs]);
    config.bind = None;
    config.desktop_bootstrap = Some(pipe);
    config
}

fn info() -> vessel_proto::ClientInfo {
    vessel_proto::ClientInfo {
        name: "private-pipe-test".to_owned(),
        version: "test".to_owned(),
    }
}

async fn echo(node: &mut Node, stream: &mut NamedPipeClient) {
    // Challenge generation belongs AFTER open, never to the name seed.
    let challenge = BootstrapChallenge::from_bytes(entropy());
    stream
        .write_all(challenge.encode().as_bytes())
        .await
        .unwrap();
    let ready = node.desktop_bootstrap_ready().await.unwrap().unwrap();
    assert_eq!(ready.node(), node.info().id);
    assert_eq!(ready.challenge().as_bytes(), challenge.as_bytes());
}

async fn authenticate(stream: NamedPipeClient, directory: &DataDir) -> vessel_client::NodeClient {
    let (reader, writer) = tokio::io::split(stream);
    let mut frames = vessel_rpc::Framed::new(reader, writer, vessel_rpc::MAX_FRAME);
    // This test presents a real isolated token only after the private echo.
    frames
        .send(directory.read_token().unwrap().expose().to_owned())
        .await
        .unwrap();
    vessel_client::NodeClient::over(frames, info(), &vessel_rpc::ClientConfig::default())
        .await
        .unwrap()
}

#[tokio::test]
async fn precreated_instance_refuses_start_without_including_its_name_in_the_error() {
    vessel_testkit::within(SAFETY_NET, async {
        let root = tempfile::tempdir().unwrap();
        let pipe = BootstrapPipe::from_bytes(entropy());
        let _squatter = ServerOptions::new()
            .first_pipe_instance(true)
            .max_instances(1)
            .reject_remote_clients(true)
            .create(pipe.path())
            .unwrap();
        let error = Node::start(config(root.path(), pipe)).await.unwrap_err();
        assert!(matches!(
            &error,
            NodeError::DesktopBootstrap(DesktopBootstrapError::Io(_))
        ));
        assert!(!format!("{error:?}").contains(pipe.path().to_str().unwrap()));
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn second_instance_is_refused_and_the_original_handle_serves_auth_and_rpc() {
    vessel_testkit::within(SAFETY_NET, async {
        let root = tempfile::tempdir().unwrap();
        let pipe = BootstrapPipe::from_bytes(entropy());
        let config = config(root.path(), pipe);
        let directory = config.data_dir.clone();
        let mut node = Node::start(config).await.unwrap();
        assert!(
            ServerOptions::new()
                .max_instances(1)
                .create(pipe.path())
                .is_err()
        );
        let mut stream = ClientOptions::new().open(pipe.path()).unwrap();
        echo(&mut node, &mut stream).await;
        // Even after connection, there is no recreated waiting instance.
        assert!(
            ServerOptions::new()
                .max_instances(1)
                .create(pipe.path())
                .is_err()
        );
        let client = authenticate(stream, &directory).await;
        assert_eq!(client.node().id, node.info().id);
        assert_eq!(
            client.node_info().await.unwrap().state,
            vessel_proto::NodeState::Serving
        );
        client
            .stop_node(vessel_proto::StopWhen::WhenIdle)
            .await
            .unwrap();
        node.stop().await.unwrap();
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn canonical_input_cannot_settle_the_private_challenge() {
    vessel_testkit::within(SAFETY_NET, async {
        let root = tempfile::tempdir().unwrap();
        let pipe = BootstrapPipe::from_bytes(entropy());
        let mut node = Node::start(config(root.path(), pipe)).await.unwrap();
        let mut canonical = ClientOptions::new().open(node.local_endpoint()).unwrap();
        canonical
            .write_all(
                BootstrapChallenge::from_bytes(entropy())
                    .encode()
                    .as_bytes(),
            )
            .await
            .unwrap();
        drop(canonical);
        let mut pending = Box::pin(node.desktop_bootstrap_ready());
        assert!(
            tokio::time::timeout(NO_CONNECTION, &mut pending)
                .await
                .is_err()
        );
        let mut private = ClientOptions::new().open(pipe.path()).unwrap();
        let challenge = BootstrapChallenge::from_bytes(entropy());
        private
            .write_all(challenge.encode().as_bytes())
            .await
            .unwrap();
        let ready = pending.await.unwrap().unwrap();
        assert_eq!(ready.challenge().as_bytes(), challenge.as_bytes());
        drop(private);
        node.stop().await.unwrap();
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn a_post_proof_namespace_replacement_cannot_receive_the_old_handles_bytes() {
    vessel_testkit::within(SAFETY_NET, async {
        let root = tempfile::tempdir().unwrap();
        let pipe = BootstrapPipe::from_bytes(entropy());
        let mut node = Node::start(config(root.path(), pipe)).await.unwrap();
        let mut original = ClientOptions::new().open(pipe.path()).unwrap();
        echo(&mut node, &mut original).await;
        node.stop().await.unwrap();
        let squatter = ServerOptions::new()
            .first_pipe_instance(true)
            .max_instances(1)
            .reject_remote_clients(true)
            .create(pipe.path())
            .unwrap();
        // This is a public sentinel, not a token. Reopening would let the new
        // server connect; writing the old retained handle must not do that.
        let _broken = original.write_all(b"old-handle-only").await;
        assert!(
            tokio::time::timeout(NO_CONNECTION, squatter.connect())
                .await
                .is_err()
        );
    })
    .await
    .unwrap();
}
