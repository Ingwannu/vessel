//! Fresh real paired-device grants after the provider's actual load barrier.
use super::*;
use vessel_client::secure::{RouteTimeouts, SecureConnector, WsDialer, pair_over};
use vessel_client::{Connector as _, SecureTarget};
use vessel_proto::{DeviceId, Scope};

#[derive(Clone, Copy)]
enum Invalidate {
    Revoke,
    Reduce,
    Off,
    Stop,
}
pub(super) async fn pair(node: &Node, owner: &NodeClient) -> (DeviceId, NodeClient) {
    let mut offer = owner
        .invite_with_scopes(None, 600, vec![Scope::Read, Scope::Operate])
        .await
        .unwrap()
        .link
        .decode()
        .unwrap();
    offer.direct = vec![format!("ws://{}", node.address().unwrap()).parse().unwrap()];
    let dialer =
        WsDialer::new(vessel_relay::TlsConfig::with_roots(rustls::RootCertStore::empty()).unwrap());
    let (paired, _) = pair_over(
        &dialer,
        &offer,
        "recovery creator",
        RouteTimeouts::default(),
    )
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
    let client = connector
        .connect(ClientInfo {
            name: "recovery-device".into(),
            version: "0".into(),
        })
        .await
        .unwrap();
    (id, client)
}
async fn invalidated(change: Invalidate) {
    vessel_testkit::within(
        CASE_DEADLINE,
        Box::pin(async {
            let f = Fixture::new();
            let node = f.start().await;
            let owner = f.client(&node).await;
            set(&node, true).await;
            let (id, device) = pair(&node, &owner).await;
            let original = f.active(&node, &device).await;
            node.stop().await.unwrap();
            drop(device);
            let gate = LoadGate::install(&f).await;
            let node = f.start().await;
            let owner = f.client(&node).await;
            let (loaded, mut load) = gate.arrive().await;
            assert_eq!(loaded, original.harness_session.as_ref().unwrap().as_str());
            match change {
                Invalidate::Revoke => owner.revoke_device(id).await.unwrap(),
                Invalidate::Reduce => {
                    // There is no scope-edit RPC. This isolated stored-grant edit
                    // precedes Ready; it does not claim to share the internal fence.
                    let path = f.data.database_path();
                    tokio::task::spawn_blocking(move || {
                        let db = rusqlite::Connection::open(path).unwrap();
                        assert_eq!(
                            db.execute(
                                "UPDATE devices SET scopes='[\"read\"]' WHERE id=?1",
                                [id.to_string()]
                            )
                            .unwrap(),
                            1
                        );
                    })
                    .await
                    .unwrap();
                }
                Invalidate::Off => set(&node, false).await,
                Invalidate::Stop => owner.kill(original.id).await.unwrap(),
            }
            drop(load.write_all(&[1]).await);
            drop(load);
            done(&node).await;
            let records = owner
                .read(original.id, EventSeq::ZERO, EVENT_PAGE)
                .await
                .unwrap()
                .records;
            assert!(!records.iter().any(restart::continuation));
            assert!(!owner.session(original.id).await.unwrap().status.is_live());
            // A completed boot owner cannot later stop a manual takeover.
            f.profile(None);
            assert!(!owner.reload_profiles().await.unwrap().is_empty());
            assert_eq!(
                owner.resume_session(original.id).await.unwrap().status,
                SessionStatus::Running
            );
            assert_eq!(
                owner
                    .prompt(
                        original.id,
                        vec![ContentBlock::Text {
                            text: "wait".into()
                        }],
                        Delivery::Start
                    )
                    .await
                    .unwrap(),
                vessel_proto::Delivered::Started
            );
            assert_eq!(
                owner.session(original.id).await.unwrap().status,
                SessionStatus::Running
            );
            node.stop().await.unwrap();
        }),
    )
    .await
    .unwrap();
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn revoked_during_load_cannot_prompt_or_clean_up_a_manual_takeover() {
    invalidated(Invalidate::Revoke).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn reduced_during_load_cannot_prompt_or_clean_up_a_manual_takeover() {
    invalidated(Invalidate::Reduce).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn off_during_load_cannot_prompt_or_clean_up_a_manual_takeover() {
    invalidated(Invalidate::Off).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn stopped_during_load_cannot_prompt_or_clean_up_a_manual_takeover() {
    invalidated(Invalidate::Stop).await;
}
