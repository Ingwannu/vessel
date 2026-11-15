//! Actual `NodeClient` round trips with finite setup/event/completion phases.
use super::*;
use std::future::Future;
use tokio::task::AbortHandle;
use vessel_app_view::agent_status::pages::MachineKey;
use vessel_app_view::models::page::Listed;
use vessel_proto::{Method, ProtocolVersion, RequestId};
mod peer;
use peer::{Peer, Respond};

const PHASE: Duration = Duration::from_secs(5);
const OUTER: Duration = READ_TIMEOUT.saturating_add(PHASE.saturating_mul(6));
async fn limited<T>(name: &str, budget: Duration, future: impl Future<Output = T>) -> T {
    tokio::time::timeout(budget, future)
        .await
        .unwrap_or_else(|_| panic!("{name} exceeded {budget:?}"))
}
async fn phase<T>(name: &str, future: impl Future<Output = T>) -> T {
    limited(name, PHASE, future).await
}
async fn safety<T>(future: impl Future<Output = T>) -> T {
    limited("model test safety", OUTER, future).await
}
struct Abort(AbortHandle);
impl Drop for Abort {
    fn drop(&mut self) {
        self.0.abort();
    }
}
fn ask() -> Ask {
    Ask {
        machine: MachineKey::from_book("fixture"),
        ticket: RequestId::new(17),
    }
}
#[tokio::test]
async fn one_connection_keeps_routes_profiles_and_added_agent_labels_in_node_order() {
    safety(async {
        let peer = Peer::new(vessel_proto::PROTOCOL_VERSION, Respond::Lists).await;
        let snapshot = phase("model snapshot", read(&peer.client)).await.unwrap();
        assert_eq!(snapshot.node, peer.client.node().id);
        let Listed::Available(routes) = snapshot.routes else {
            panic!("routes unsupported")
        };
        assert_eq!(routes.len(), 1);
        assert_eq!(routes[0].name.as_str(), "shared");
        assert_eq!(routes[0].model.as_str(), "fixture/large");
        assert_eq!(routes[0].provider.as_str(), "fixture");
        assert_eq!(routes[0].api, vessel_proto::ModelApi::OpenAiResponses);
        assert_eq!(
            routes[0].small_model.as_ref().unwrap().as_str(),
            "fixture/small"
        );
        let Listed::Available(profiles) = snapshot.profiles else {
            panic!("profiles unsupported")
        };
        assert_eq!(
            profiles.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(),
            ["work", "personal"]
        );
        assert_eq!(snapshot.agents.len(), 1);
        assert_eq!(
            vessel_app_view::agent_status::agent_label(
                &vessel_proto::HarnessRef::Added("custom".parse().unwrap()),
                &snapshot.agents
            ),
            "Custom agent"
        );
        assert_eq!(
            peer.calls(),
            [Method::ListModels, Method::ListProfiles, Method::ListAgents]
        );
    })
    .await;
}
#[tokio::test]
async fn unsupported_methods_are_not_sent_and_successful_empty_is_available() {
    safety(async {
        let before_profiles = ProtocolVersion::new(
            Method::ListProfiles
                .since()
                .unwrap()
                .get()
                .checked_sub(1)
                .unwrap(),
        );
        let before_models = ProtocolVersion::new(
            Method::ListModels
                .since()
                .unwrap()
                .get()
                .checked_sub(1)
                .unwrap(),
        );
        for (protocol, models, profiles, expected) in [
            (before_profiles, false, false, vec![]),
            (before_models, false, true, vec![Method::ListProfiles]),
            (
                Method::ListModels.since().unwrap(),
                true,
                true,
                vec![Method::ListModels, Method::ListProfiles],
            ),
            (
                vessel_proto::PROTOCOL_VERSION,
                true,
                true,
                vec![Method::ListModels, Method::ListProfiles, Method::ListAgents],
            ),
        ] {
            let peer = Peer::new(protocol, Respond::Empty).await;
            let snapshot = phase("model snapshot", read(&peer.client)).await.unwrap();
            assert_eq!(
                matches!(snapshot.routes,Listed::Available(ref rows) if rows.is_empty()),
                models
            );
            assert_eq!(
                matches!(snapshot.profiles,Listed::Available(ref rows) if rows.is_empty()),
                profiles
            );
            assert!(snapshot.agents.is_empty());
            peer.barrier().await;
            assert_eq!(
                peer.calls()
                    .iter()
                    .copied()
                    .filter(|method| *method != Method::NodeInfo)
                    .collect::<Vec<_>>(),
                expected
            );
        }
    })
    .await;
}
#[tokio::test]
async fn node_refusals_keep_the_original_wire_error() {
    safety(async {
        for code in [
            vessel_proto::ErrorCode::Forbidden,
            vessel_proto::ErrorCode::Unauthorized,
            vessel_proto::ErrorCode::Unsupported,
        ] {
            let peer = Peer::new(vessel_proto::PROTOCOL_VERSION, Respond::Refuse(code)).await;
            let ModelsError::Call(CallError::Remote(wire)) =
                phase("model refusal", read(&peer.client))
                    .await
                    .unwrap_err()
            else {
                panic!("wire error lost")
            };
            assert_eq!(
                wire,
                vessel_proto::WireError::new(code, "fixture diagnostic")
            );
            assert_eq!(peer.calls(), [Method::ListModels]);
        }
    })
    .await;
}
#[tokio::test(start_paused = true)]
async fn hub_deadline_and_capacity_settle_the_original_machine_and_ticket() {
    safety(async {
        let mut peer = Peer::new(vessel_proto::PROTOCOL_VERSION, Respond::Wait).await;
        let hub = Hub::default();
        let (send, mut receive) = mpsc::channel(MAX_READS.saturating_add(2));
        let mut pending = Vec::new();
        for index in 0..MAX_READS {
            let request = Ask {
                machine: MachineKey::from_book(format!("occupied-{index}")),
                ticket: RequestId::new(u64::try_from(index).unwrap()),
            };
            pending.push((request.machine.clone(), request.ticket));
            phase(
                "hub dispatch",
                hub.ask(Some(peer.client.clone()), request, send.clone()),
            )
            .await;
            assert_eq!(
                phase("model call observed", peer.seen.recv()).await,
                Some(Method::ListModels)
            );
        }
        phase(
            "busy dispatch",
            hub.ask(Some(peer.client.clone()), ask(), send.clone()),
        )
        .await;
        let crate::Event::Models(answer) = phase("busy event", receive.recv()).await.unwrap()
        else {
            panic!("wrong event")
        };
        assert_eq!(answer.machine, ask().machine);
        assert_eq!(answer.ticket, ask().ticket);
        assert!(matches!(answer.result, Err(ModelsError::Busy)));
        phase("offline dispatch", hub.ask(None, ask(), send)).await;
        let crate::Event::Models(answer) = phase("offline event", receive.recv()).await.unwrap()
        else {
            panic!("wrong event")
        };
        assert_eq!(answer.machine, ask().machine);
        assert_eq!(answer.ticket, ask().ticket);
        assert!(matches!(answer.result, Err(ModelsError::Offline)));
        // All short observation/event phases have completed before this.
        tokio::time::advance(READ_TIMEOUT).await;
        for _ in 0..MAX_READS {
            let crate::Event::Models(answer) =
                phase("deadline event", receive.recv()).await.unwrap()
            else {
                panic!("wrong event")
            };
            let target = (answer.machine, answer.ticket);
            let Some(index) = pending.iter().position(|expected| *expected == target) else {
                panic!("stale or duplicate deadline event")
            };
            assert_eq!(pending.remove(index), target);
            assert!(matches!(answer.result, Err(ModelsError::Timeout)));
        }
        assert!(pending.is_empty());
        peer.barrier().await;
        assert_eq!(
            peer.calls()
                .iter()
                .filter(|method| **method == Method::ListModels)
                .count(),
            MAX_READS
        );
    })
    .await;
}
