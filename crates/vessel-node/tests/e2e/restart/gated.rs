//! A real provider load is held while authority/policy/ownership changes.
use super::{fixture, policy, running, started, text, write_profile};
use crate::support::{agent_of, client, read_all, read_until, step, within};
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::{TcpListener, TcpStream};
use vessel_client::Connector as _;
use vessel_client::secure::{RouteTimeouts, pair_over};
use vessel_proto::{Scope, SessionStatus};

pub(super) struct Gate {
    listener: TcpListener,
}
impl Gate {
    pub(super) async fn install(f: &crate::support::Fixture) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        write_profile(f, Some(listener.local_addr().unwrap()));
        Self { listener }
    }
    pub(super) async fn arrive(&self) -> (String, TcpStream) {
        let (mut stream, _) = step(
            "provider reaches session/load barrier",
            self.listener.accept(),
        )
        .await
        .unwrap();
        let length = step("load id length", stream.read_u32()).await.unwrap();
        assert!(length <= 4096);
        let mut bytes = vec![0; usize::try_from(length).unwrap()];
        assert_eq!(
            step("load id", stream.read_exact(&mut bytes))
                .await
                .unwrap(),
            bytes.len()
        );
        (String::from_utf8(bytes).unwrap(), stream)
    }
}
#[derive(Clone, Copy)]
enum Change {
    Revoke,
    Reduce,
    Off,
    Stop,
}
async fn blocked(change: Change) {
    within(async {
        let f = fixture();
        let node = f.start().await;
        let owner = f.connect(&node).await;
        policy(&owner, true, Some("loaded")).await;
        let (device, creator) = creator(&node, &owner).await;

        let id = running(&device, &f).await;
        assert_eq!(
            device
                .prompt(id, text("wait"), vessel_proto::Delivery::Start)
                .await
                .unwrap(),
            vessel_proto::Delivered::Started
        );
        let original = device.session(id).await.unwrap();
        node.stop().await.unwrap();
        drop(device);
        let gate = Gate::install(&f).await;
        let node = f.start().await;
        let owner = f.connect(&node).await;
        let (loading, mut held) = gate.arrive().await;
        assert_eq!(loading, original.harness_session.as_ref().unwrap().as_str());
        match change {
            Change::Revoke => owner.revoke_device(creator).await.unwrap(),
            Change::Reduce => {
                // No product scope-edit RPC exists. Change the isolated
                // stored grant while load is held, before releasing ready;
                // the subsequent fresh lookup must observe its real bytes.
                let data = f.data_dir();
                let device = creator;
                tokio::task::spawn_blocking(move || {
                    let db = rusqlite::Connection::open(data.database_path()).unwrap();
                    assert_eq!(
                        db.execute(
                            "UPDATE devices SET scopes='[\"read\"]' WHERE id=?1",
                            [device.to_string()]
                        )
                        .unwrap(),
                        1
                    );
                })
                .await
                .unwrap();
            }
            Change::Off => policy(&owner, false, None).await,
            Change::Stop => owner.kill(id).await.unwrap(),
        }
        // Stop/Off may already have killed the held process: a closed
        // barrier is expected and never used to infer a prompt success.
        drop(held.write_all(&[1]).await);
        drop(held);
        let mut stream = owner.follow(id, original.next_seq).await.unwrap();
        let mut seen = Vec::new();
        read_until(&mut stream, &mut seen, |record| {
            matches!(&record.event, vessel_proto::SessionEvent::Status { status } if !status.is_live())
        }).await;
        assert!(
            !read_all(&owner, id)
                .await
                .iter()
                .any(|record| started(record, "loaded"))
        );
        let records = read_all(&owner, id).await;
        assert!(records.iter().any(|record| {
            matches!(agent_of(record), Some(vessel_proto::AgentEvent::HarnessLog { line })
                if line.contains("startup recovery"))
        }));
        // The finished boot owner cannot kill a later manual run.
        write_profile(&f, None);
        let profiles = owner.reload_profiles().await.unwrap();
        assert!(!profiles.is_empty());
        let resumed = owner.resume_session(id).await.unwrap();
        assert_eq!(resumed.status, SessionStatus::Running);
        assert_eq!(
            owner
                .prompt(id, text("wait"), vessel_proto::Delivery::Start)
                .await
                .unwrap(),
            vessel_proto::Delivered::Started
        );
        assert_eq!(
            owner.session(id).await.unwrap().status,
            SessionStatus::Running
        );
        node.stop().await.unwrap();
    })
    .await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn revocation_during_real_load_prevents_the_next_prompt() {
    blocked(Change::Revoke).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scope_reduction_during_real_load_prevents_the_next_prompt() {
    blocked(Change::Reduce).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn off_during_real_load_prevents_the_next_prompt() {
    blocked(Change::Off).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn person_stop_during_real_load_prevents_the_next_prompt_and_late_cleanup() {
    blocked(Change::Stop).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn off_after_actual_prompt_admission_keeps_that_turn_running() {
    within(async {
        let f = fixture();
        let node = f.start().await;
        let owner = f.connect(&node).await;
        policy(&owner, true, Some("wait")).await;
        let id = running(&owner, &f).await;
        assert_eq!(
            owner
                .prompt(id, text("wait"), vessel_proto::Delivery::Start)
                .await
                .unwrap(),
            vessel_proto::Delivered::Started
        );
        let before = owner.session(id).await.unwrap().next_seq;
        node.stop().await.unwrap();
        let gate = Gate::install(&f).await;
        let node = f.start().await;
        let owner = f.connect(&node).await;
        let (_, mut held) = gate.arrive().await;
        let mut stream = owner.follow(id, before).await.unwrap();
        let mut seen = Vec::new();
        held.write_all(&[1]).await.unwrap();
        drop(held);
        read_until(&mut stream, &mut seen, |record| started(record, "wait")).await;
        policy(&owner, false, None).await;
        assert_eq!(
            owner.session(id).await.unwrap().status,
            SessionStatus::Running
        );
        assert!(
            owner
                .prompt(id, text("not admitted"), vessel_proto::Delivery::Start)
                .await
                .is_err()
        );
        node.stop().await.unwrap();
        let node = f.start().await;
        let owner = f.connect(&node).await;
        assert_eq!(
            read_all(&owner, id)
                .await
                .iter()
                .filter(|record| started(record, "wait"))
                .count(),
            2
        );
        node.stop().await.unwrap();
    })
    .await;
}

async fn creator(
    node: &vessel_node::Node,
    owner: &vessel_client::NodeClient,
) -> (vessel_client::NodeClient, vessel_proto::DeviceId) {
    let mut offer = owner
        .invite_with_scopes(None, 600, vec![Scope::Read, Scope::Operate])
        .await
        .unwrap()
        .link
        .decode()
        .unwrap();
    let route = crate::secure::direct_route(node);
    offer.direct = vec![route.clone()];
    let (paired, _) = pair_over(
        &crate::secure::dialer(),
        &offer,
        "recovery creator",
        RouteTimeouts::default(),
    )
    .await
    .unwrap();
    let connector = crate::secure::device_via(crate::secure::dialer(), &paired, &[&route], &[]);
    let device = connector.connect(client()).await.unwrap();
    let visible = device
        .call(vessel_proto::settings::GetNodeSettings {
            include_restart_recovery: true,
        })
        .await
        .unwrap();
    assert!(visible.restart_recovery.enabled);
    let refused = device
        .call(vessel_proto::settings::SetNodeSettings {
            restart_recovery: Some(vessel_proto::restart::SetRestartRecovery {
                enabled: Some(false),
                prompt: None,
            }),
            ..vessel_proto::settings::SetNodeSettings::default()
        })
        .await;
    assert_eq!(
        crate::agents::code(refused),
        vessel_proto::ErrorCode::Forbidden
    );

    (device, paired.credential.device)
}
