//! Fake-agent attention travels through a real node, encrypted RPC pairing,
//! `SQLite` and a real HTTP receiver. Only the final HTTP origin is injected.
use super::crypto_tests::subscriber;
use crate::{Config, DataDir, Node};
use axum::{
    Router,
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::post,
};
use tokio::{net::TcpListener, sync::mpsc};
use tokio_util::sync::CancellationToken;
use vessel_client::Connector as _;
use vessel_client::{
    BearerToken, NodeClient, NodeUrl, SecureTarget,
    secure::{RouteTimeouts, SecureConnector, WsDialer, pair_over},
};
use vessel_harness::{
    HarnessEnv, Lookup,
    fakes::{Role, install},
};
use vessel_proto::inbox::{InboxRead, ListInbox};
use vessel_proto::push::{PushActivity, PushFocus, ReportPushPresence};
use vessel_proto::{
    AgentEvent, ClientInfo, ContentBlock, EventSeq, HarnessKind, InviteDevice, Scope, SessionEvent,
    SessionInput, SessionMode, SessionSpec,
};
use vessel_relay::TlsConfig;
use vessel_testkit::Fake;

async fn step<T>(name: &'static str, future: impl Future<Output = T>) -> T {
    tokio::time::timeout(std::time::Duration::from_secs(10), Box::pin(future))
        .await
        .unwrap_or_else(|_| panic!("push step timed out: {name}"))
}

struct Received {
    headers: HeaderMap,
    body: Bytes,
    reply: Option<tokio::sync::oneshot::Sender<()>>,
}
struct Receiver {
    url: url::Url,
    inbox: mpsc::Receiver<Received>,
    stop: CancellationToken,
    task: tokio::task::JoinHandle<()>,
}
impl Receiver {
    async fn start(status: StatusCode) -> Self {
        Self::with_headers(status, HeaderMap::new()).await
    }
    async fn with_headers(status: StatusCode, response_headers: HeaderMap) -> Self {
        Self::serve(status, response_headers, false).await
    }
    async fn delayed(status: StatusCode) -> Self {
        Self::serve(status, HeaderMap::new(), true).await
    }
    async fn serve(status: StatusCode, response_headers: HeaderMap, delayed: bool) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/push", listener.local_addr().unwrap())
            .parse()
            .unwrap();
        let (outbox, inbox) = mpsc::channel(16);
        let router = Router::new()
            .route(
                "/push",
                post(
                    move |State(outbox): State<mpsc::Sender<Received>>,
                          headers: HeaderMap,
                          body: Bytes| async move {
                        let response_headers = response_headers.clone();
                        let (reply, released) = if delayed {
                            let (reply, released) = tokio::sync::oneshot::channel();
                            (Some(reply), Some(released))
                        } else {
                            (None, None)
                        };
                        outbox
                            .send(Received {
                                headers,
                                body,
                                reply,
                            })
                            .await
                            .unwrap();
                        if let Some(released) = released {
                            released.await.unwrap();
                        }
                        (status, response_headers)
                    },
                ),
            )
            .with_state(outbox);
        let stop = CancellationToken::new();
        let closing = stop.clone();
        let task = tokio::spawn(async move {
            axum::serve(listener, router)
                .with_graceful_shutdown(closing.cancelled_owned())
                .await
                .unwrap();
        });
        Self {
            url,
            inbox,
            stop,
            task,
        }
    }
    async fn close(self) {
        self.stop.cancel();
        self.task.await.unwrap();
    }
}
fn client_info() -> ClientInfo {
    ClientInfo {
        name: "push-test".to_owned(),
        version: "0".to_owned(),
    }
}
struct Fixture {
    temp: tempfile::TempDir,
    node: Node,
    owner: NodeClient,
}
impl Fixture {
    async fn start(origin: url::Url) -> Self {
        Self::start_with(origin, |_| {}).await
    }
    async fn start_with(origin: url::Url, adjust: impl FnOnce(&mut Config)) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let bin = temp.path().join("bin");
        std::fs::create_dir(&bin).unwrap();
        let fake = Fake::Acp.path().unwrap();
        drop(install(&bin, HarnessKind::ClaudeCode, Role::Structured, &fake).unwrap());
        let home = temp.path().join("home");
        std::fs::create_dir(&home).unwrap();
        let data = DataDir::new(temp.path().join("data"));
        let mut config = Self::config(temp.path(), origin);
        adjust(&mut config);
        let node = Node::start(config).await.unwrap();
        let url: NodeUrl = format!("ws://{}", node.address().unwrap()).parse().unwrap();
        let token: BearerToken = data.read_token().unwrap().expose().parse().unwrap();
        let owner = NodeClient::connect(&url, &token, client_info())
            .await
            .unwrap();
        Self { temp, node, owner }
    }
    fn config(root: &std::path::Path, origin: url::Url) -> Config {
        let mut config = Config::new(
            DataDir::new(root.join("data")),
            HarnessEnv::new(root.join("home")),
        );
        config.bind = Some("127.0.0.1:0".parse().unwrap());
        config.harness_lookup = Lookup::new([root.join("bin")]);
        config.name = Some("inbox push test".to_owned());
        config.push_test_origin = Some(origin);
        // A node refuses an invitation it has no route for (N3's row 4);
        // the tests dial its real address, so the advertised one only has
        // to exist, as in the e2e's `secure::advertise_placeholder_route`.
        config.public = vec!["ws://127.0.0.1:1".parse().unwrap()];
        config
    }
    async fn pair(&self) -> (vessel_proto::DeviceId, NodeClient) {
        let mut offer = self
            .owner
            .call(InviteDevice {
                label: None,
                scopes: Some(vec![Scope::Read]),
                ttl_secs: 60,
            })
            .await
            .unwrap()
            .link
            .decode()
            .unwrap();
        offer.direct = vec![
            format!("ws://{}", self.node.address().unwrap())
                .parse()
                .unwrap(),
        ];
        let dialer = WsDialer::new(TlsConfig::with_roots(rustls::RootCertStore::empty()).unwrap());
        let (paired, _) = pair_over(&dialer, &offer, "inbox phone", RouteTimeouts::default())
            .await
            .unwrap();
        let id = paired.credential.device;
        let target = SecureTarget {
            key: paired.credential.key,
            direct: offer.direct,
            relays: Vec::new(),
        };
        let connector =
            SecureConnector::new(dialer, target, paired.node.id, paired.credential).unwrap();
        (id, connector.connect(client_info()).await.unwrap())
    }
    async fn asking(&self) -> (vessel_proto::SessionId, vessel_client::SessionStream) {
        let id = self
            .owner
            .create_session(
                SessionSpec::new(
                    HarnessKind::ClaudeCode,
                    SessionMode::Structured {
                        model: None,
                        resume: None,
                        options: Vec::new(),
                    },
                )
                .with_cwd(self.temp.path().to_path_buf()),
            )
            .await
            .unwrap()
            .id;
        let mut stream = self.owner.follow(id, EventSeq::ZERO).await.unwrap();
        self.owner
            .send(
                id,
                SessionInput::Prompt {
                    content: vec![ContentBlock::Text {
                        text: "tool".to_owned(),
                    }],
                },
            )
            .await
            .unwrap();
        loop {
            let record = step("fake agent asks for permission", stream.next())
                .await
                .unwrap()
                .unwrap();
            if matches!(
                record.event,
                SessionEvent::Agent {
                    event: AgentEvent::PermissionRequested(_)
                }
            ) {
                break;
            }
        }
        let _committed = step(
            "permission committed before coalescing",
            self.owner.read(id, EventSeq::ZERO, 1),
        )
        .await
        .unwrap();
        (id, stream)
    }
    async fn close(self) {
        self.node.stop().await.unwrap();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn disconnected_read_device_receives_only_encrypted_generic_attention_from_a_fake_agent() {
    Box::pin(vessel_testkit::within(
        std::time::Duration::from_secs(60),
        async {
            let mut receiver = Receiver::start(StatusCode::CREATED).await;
            let fixture = Fixture::start(receiver.url.clone()).await;
            let (device_id, device) = fixture.pair().await;
            let key = device.push_key().await.unwrap();
            let (private, subscription) = subscriber();
            let lease = device.subscribe_push(subscription.clone()).await.unwrap();
            assert!(lease.expires_at > chrono::Utc::now());
            assert_eq!(device.push_key().await.unwrap(), key);
            assert_eq!(
                fixture.owner.push_key().await.unwrap_err().code(),
                Some(vessel_proto::ErrorCode::Forbidden)
            );
            // A connected device's focus change alone is not artificial activity.
            device
                .report_push_presence(ReportPushPresence {
                    idle_age: None,
                    focus: PushFocus::Background,
                    activity: PushActivity::Unchanged,
                })
                .await
                .unwrap();
            drop(device);
            let (id, _stream) = fixture.asking().await;
            let message = step("encrypted HTTP request arrived", receiver.inbox.recv())
                .await
                .unwrap();
            assert_eq!(
                message.headers.get("Content-Encoding").unwrap(),
                "aes128gcm"
            );
            let decoded = web_push_native::decrypt(
                message.body.to_vec(),
                &private,
                &web_push_native::Auth::from(*subscription.auth.expose()),
            )
            .unwrap();
            assert_eq!(decoded, crate::push::crypto::PAYLOAD);
            assert_eq!(
                fixture
                    .owner
                    .inbox(ListInbox::default())
                    .await
                    .unwrap()
                    .entries[0]
                    .key
                    .session,
                id
            );
            fixture.owner.revoke_device(device_id).await.unwrap();
            let page = fixture.owner.inbox(ListInbox::default()).await.unwrap();
            fixture
                .owner
                .mark_inbox(page.entries[0].key, InboxRead::Read)
                .await
                .unwrap();
            fixture.close().await;
            receiver.close().await;
        },
    ))
    .await
    .unwrap();
}

#[tokio::test]
async fn redirects_are_not_followed_and_expired_statuses_are_distinct_from_refusals() {
    for (status, expected) in [
        (
            StatusCode::CREATED,
            crate::push::delivery::Delivered::Accepted,
        ),
        (
            StatusCode::NOT_FOUND,
            crate::push::delivery::Delivered::Gone,
        ),
        (StatusCode::GONE, crate::push::delivery::Delivered::Gone),
        (StatusCode::FOUND, crate::push::delivery::Delivered::Refused),
        (
            StatusCode::TOO_MANY_REQUESTS,
            crate::push::delivery::Delivered::Refused,
        ),
    ] {
        let mut headers = HeaderMap::new();
        let _old = headers.insert(
            axum::http::header::LOCATION,
            axum::http::HeaderValue::from_static("http://127.0.0.1:9/never-follow"),
        );
        let receiver = Receiver::with_headers(status, headers).await;
        let temp = tempfile::tempdir().unwrap();
        let dir = DataDir::new(temp.path());
        dir.prepare().unwrap();
        let crypto = crate::push::crypto::Crypto::load(&dir)
            .unwrap()
            .test_at(receiver.url.clone());
        let (_, subscription) = subscriber();
        let request = crypto.encrypt(&subscription).unwrap();
        let prepared = crypto
            .delivery
            .prepare(&subscription.endpoint, request)
            .await
            .unwrap();
        assert_eq!(prepared.send().await.unwrap(), expected);
        receiver.close().await;
    }
}
#[tokio::test]
async fn private_numeric_and_localhost_endpoints_never_reach_http_preparation() {
    let delivery = crate::push::delivery::Delivery::new().unwrap();
    for endpoint in [
        "https://127.0.0.1/push",
        "https://10.0.0.1/push",
        "https://169.254.169.254/push",
        "https://localhost/push",
    ] {
        assert!(matches!(
            delivery.validate(&endpoint.parse().unwrap()).await,
            Err(crate::push::delivery::DeliveryError::Endpoint)
        ));
    }
}

mod cancellation;

mod restart;

mod renewed_lease;

mod terminal_delivery;
