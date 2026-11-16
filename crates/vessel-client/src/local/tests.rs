use super::*;
use futures_util::StreamExt as _;
use std::time::Duration;

const DEADLINE: Duration = Duration::from_secs(5);
const NO_REDIAL: Duration = Duration::from_millis(50);

#[tokio::test]
async fn open_and_peer_query_emit_zero_bytes_and_auth_uses_the_original_handle() {
    tokio::time::timeout(DEADLINE, async {
        let root = tempfile::tempdir().unwrap();
        let endpoint = root.path().join("socket");
        let first = tokio::net::UnixListener::bind(&endpoint).unwrap();
        let opened = NodeClient::open_local(&endpoint).await.unwrap();
        let (original, _) = first.accept().await.unwrap();
        assert_eq!(
            opened.peer_process().unwrap().unwrap().get(),
            std::process::id()
        );
        assert_eq!(
            original.try_read(&mut [0; 1]).unwrap_err().kind(),
            io::ErrorKind::WouldBlock
        );
        // Namespace replacement cannot redirect this already-opened handle.
        std::fs::remove_file(&endpoint).unwrap();
        let replacement = tokio::net::UnixListener::bind(&endpoint).unwrap();
        let authenticating = tokio::spawn(async move {
            let credential = LocalCredential::Token("public-test-credential".parse().unwrap());
            opened
                .authenticate(
                    &credential,
                    ClientInfo {
                        name: "peer-test".to_owned(),
                        version: "test".to_owned(),
                    },
                )
                .await
        });
        let (reader, writer) = tokio::io::split(original);
        let mut frames = Framed::new(reader, writer, MAX_FRAME);
        assert_eq!(
            frames.next().await.unwrap().unwrap(),
            "public-test-credential"
        );
        drop(frames);
        assert!(authenticating.await.unwrap().is_err());
        assert!(
            tokio::time::timeout(NO_REDIAL, replacement.accept())
                .await
                .is_err()
        );
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn private_open_errors_do_not_retain_the_endpoint_name() {
    let root = tempfile::tempdir().unwrap();
    let private = root.path().join("never-print-private-pipe-name");
    let error = NodeClient::open_local(&private).await.unwrap_err();
    assert!(!format!("{error:?}").contains("never-print-private-pipe-name"));
    assert!(matches!(error, LocalConnectError::Open(_)));
}
