//! Actual server encode of Get and legacy parking/driver mutation replies.
//! The only new policy input is the local durable file, never a staged RPC.
use futures_util::{SinkExt as _, StreamExt as _};
use std::future::Future;
use std::time::Duration;
use tokio_tungstenite::tungstenite::{Message, client::IntoClientRequest as _};
use vessel_harness::{HarnessEnv, Lookup};
use vessel_proto::restart::{RestartRecovery, SetRestartRecovery};
use vessel_proto::settings::{GetNodeSettings, SetAgentDrivers, SetNodeSettings};
use vessel_proto::{
    Call, ClientFrame, ClientInfo, Hello, Outcome, ProtocolRange, ProtocolVersion, Reply,
    Request as _, RequestId, ServerFrame,
};
use vessel_store::{DataDir, SavedNodeSettings};

const SAVED_PROMPT: &str = "  synthetic saved continuation.\n";

type Socket =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

#[tokio::test]
async fn legacy_get_and_set_replies_hide_a_nondefault_saved_policy_on_the_wire() {
    within(async {
        let directory = tempfile::tempdir().unwrap();
        let data = DataDir::new(directory.path().join("data"));
        data.prepare().unwrap();
        let saved = SetRestartRecovery {
            enabled: Some(true),
            prompt: Some(SAVED_PROMPT.parse().unwrap()),
        };
        data.write_settings(
            &SavedNodeSettings::default().with_public_settings(SetNodeSettings {
                restart_recovery: Some(saved.clone()),
                ..SetNodeSettings::default()
            }),
        )
        .unwrap();
        let mut config =
            crate::Config::new(data.clone(), HarnessEnv::new(directory.path().join("home")));
        config.bind = Some("127.0.0.1:0".parse().unwrap());
        config.harness_lookup = Lookup::new(Vec::new());
        let node = step(
            "node starts from full saved record",
            crate::Node::start(config),
        )
        .await
        .unwrap();
        let first = GetNodeSettings::default()
            .into_call()
            .since()
            .expect("ordinary settings reads remain assigned");
        for version in first.get()..=vessel_proto::PROTOCOL_VERSION.get() {
            let version = ProtocolVersion::new(version);
            let mut socket = raw_owner(&data, &node, version).await;
            for (id, call) in ordinary_calls() {
                let required = call
                    .since()
                    .expect("ordinary settings calls remain assigned");
                if required > version {
                    continue;
                }
                assert_hidden_reply(&mut socket, id, call).await;
            }
            step("old socket closes", socket.close(None)).await.unwrap();
            assert_eq!(
                data.read_settings()
                    .unwrap()
                    .unwrap()
                    .public_settings()
                    .restart_recovery,
                Some(saved.clone())
            );
        }
        step("node stops", node.stop()).await.unwrap();
    })
    .await;
}

fn ordinary_calls() -> [(RequestId, Call); 3] {
    [
        (RequestId::new(1), GetNodeSettings::default().into_call()),
        (
            RequestId::new(2),
            SetNodeSettings {
                park_after: Some(vessel_proto::parking::ParkAfter::Never),
                ..SetNodeSettings::default()
            }
            .into_call(),
        ),
        (
            RequestId::new(3),
            SetNodeSettings {
                park_after: None,
                drivers: Some(SetAgentDrivers::default()),
                ..SetNodeSettings::default()
            }
            .into_call(),
        ),
    ]
}

async fn assert_hidden_reply(socket: &mut Socket, id: RequestId, call: Call) {
    let request = ClientFrame::Request { id, call };
    step(
        "legacy settings request",
        socket.send(Message::text(serde_json::to_string(&request).unwrap())),
    )
    .await
    .unwrap();
    let raw = step("legacy settings response", socket.next())
        .await
        .unwrap()
        .unwrap()
        .into_text()
        .unwrap();
    // Inspect bytes before decoding: defaulted response fields alone cannot
    // prove that an old peer was never sent the new field or saved prompt.
    assert!(!raw.contains("restart_recovery"));
    assert!(!raw.contains("synthetic saved continuation"));
    let ServerFrame::Response { outcome, .. } = serde_json::from_str(&raw).unwrap() else {
        panic!("settings response expected")
    };
    let Outcome::Ok(Reply::GetNodeSettings(shown) | Reply::SetNodeSettings(shown)) = outcome else {
        panic!("successful legacy settings operation: {outcome:?}")
    };
    assert_eq!(shown.restart_recovery, RestartRecovery::default());
}

async fn raw_owner(data: &DataDir, node: &crate::Node, version: ProtocolVersion) -> Socket {
    let mut request = format!("ws://{}/rpc", node.address().unwrap())
        .into_client_request()
        .unwrap();
    let mut bearer = format!("Bearer {}", data.load_or_create_token().unwrap().expose())
        .parse::<tokio_tungstenite::tungstenite::http::HeaderValue>()
        .unwrap();
    bearer.set_sensitive(true);
    let _old = request.headers_mut().insert("authorization", bearer);
    let (mut socket, _) = step(
        "old owner connects",
        tokio_tungstenite::connect_async(request),
    )
    .await
    .unwrap();
    let hello = ClientFrame::Hello(Hello {
        protocol: ProtocolRange {
            min: version,
            max: version,
        },
        client: ClientInfo {
            name: "settings-legacy-test".to_owned(),
            version: "0".to_owned(),
        },
    });
    step(
        "old hello",
        socket.send(Message::text(serde_json::to_string(&hello).unwrap())),
    )
    .await
    .unwrap();
    let raw = step("old welcome", socket.next())
        .await
        .unwrap()
        .unwrap()
        .into_text()
        .unwrap();
    let ServerFrame::Welcome(welcome) = serde_json::from_str(&raw).unwrap() else {
        panic!("welcome expected")
    };
    assert_eq!(welcome.protocol, version);
    socket
}

const CASE_DEADLINE: Duration = Duration::from_secs(60);
const STEP_TIMEOUT: Duration = Duration::from_secs(10);

async fn within<F: Future<Output = ()> + Send>(future: F) {
    vessel_testkit::within(CASE_DEADLINE, Box::pin(future))
        .await
        .unwrap();
}
async fn step<F: Future>(name: &str, future: F) -> F::Output {
    tokio::time::timeout(STEP_TIMEOUT, future)
        .await
        .unwrap_or_else(|error| panic!("{name}: {error}"))
}

#[test]
fn canonical_complete_and_compatibility_decode_chains_hide_private_input() {
    const PRIVATE: &str = "synthetic-private-recovery-note";
    let directory = tempfile::tempdir().unwrap();
    let data = DataDir::new(directory.path());
    let text = format!(r#"{{"restart_recovery":{{"enabled":"{PRIVATE}"}}}}"#);
    std::fs::write(data.settings_path(), text).unwrap();
    for error in [
        data.read_settings().unwrap_err(),
        data.read_settings_snapshot().unwrap_err(),
    ] {
        let mut current: Option<&dyn std::error::Error> = Some(&error);
        let mut depth = 0_usize;
        while let Some(cause) = current {
            assert!(!cause.to_string().contains(PRIVATE));
            assert!(!format!("{cause:?}").contains(PRIVATE));
            depth = depth.checked_add(1).unwrap();
            current = cause.source();
        }
        assert_eq!(depth, 2, "file error and safe decoder only, no raw cause");
    }
}
