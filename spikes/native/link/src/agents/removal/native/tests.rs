//! Real client/RPC messages; these simulate typed node refusals, not node policy.
use super::*;
use crate::Event;
use crate::agents::{AgentsError, Ask, Hub, MachineKey};
use peer::{Peer, Response};
use tokio::sync::mpsc;
use vessel_proto::{Call, RequestId, WireError};

mod budget;
mod peer;
use budget::{AbortOnDrop, phase, safety};

fn target(peer: &Peer) -> Removal {
    Removal {
        node: peer.client.node().id,
        name: "example".parse().unwrap(),
    }
}

#[tokio::test]
async fn a_confirmed_command_reaches_the_hub_and_settles_its_original_page_ticket() {
    safety(async {
        use vessel_app_view::agent_status::{
            command::CommandShell,
            pages::{Administration, Inventory, Operation, Pages},
            removal::Confirmation,
        };
        let peer = Peer::new(REMOVE_PROTOCOL, Response::Ack).await;
        let machine = MachineKey::from_book("confirmed-route");
        let mut pages = Pages::<()>::default();
        let read = pages.begin(machine.clone(), Operation::Discovery).unwrap();
        assert!(pages.loaded(
            &machine,
            read,
            Inventory {
                node: peer.client.node().id,
                command_shell: CommandShell::Unix,
                harnesses: Vec::new(),
                agents: vec![serde_json::from_str(
                    r#"{"agent":{"added":"example"},"detail":{"kind":"added","added":{"program":"fixture-agent","source":{"from":"command"}}}}"#,
                ).unwrap()],
                readiness: Vec::new(),
                administration: Administration::Supported,
            },
        ));
        let confirmed = Confirmation::new(
            &pages,
            machine.clone(),
            &vessel_proto::HarnessRef::Added("example".parse().unwrap()),
        )
        .unwrap()
        .confirm(&machine, &mut pages)
        .unwrap();
        let original = pages.get(machine.as_str()).unwrap().pending().unwrap().id;
        let crate::Command::Agents {
            machine: routed,
            request,
            ask,
        } = crate::Command::remove_agent(confirmed)
        else {
            panic!("not an Agents command")
        };
        assert_eq!(routed, machine);
        assert_eq!(request, original);
        let (send, mut receive) = mpsc::channel(1);
        phase("hub command delivery", Hub::default()
            .ask(Some(peer.client.clone()), routed, request, ask, send)).await;
        let Event::Agents {
            machine: replied,
            request: ticket,
            answer: Ok(Answer::Removed { node, name }),
        } = phase("hub event delivery", receive.recv()).await.unwrap()
        else {
            panic!("removal was not acknowledged")
        };
        assert_eq!(replied, machine);
        assert_eq!(ticket, original);
        assert!(pages.removed(&replied, ticket, node, &name));
        assert!(pages.get(machine.as_str()).unwrap().pending().is_none());
        assert!(
            pages
                .get(machine.as_str())
                .unwrap()
                .inventory()
                .unwrap()
                .agents
                .is_empty()
        );
        assert_eq!(peer.removals(), vec![name]);
    }).await;
}

#[tokio::test]
async fn acknowledgement_echoes_the_confirmed_name_and_actual_node_once() {
    safety(async {
        let peer = Peer::new(REMOVE_PROTOCOL, Response::Ack).await;
        let target = target(&peer);
        let node = target.node;
        let name = target.name.clone();
        let answer = phase("removal response", remove(&peer.client, target)).await.unwrap();
        assert!(
            matches!(answer, Answer::Removed { node: actual, name: removed } if actual == node && removed == name)
        );
        peer.barrier().await;
        assert_eq!(peer.removals(), vec![name]);
    }).await;
}

#[tokio::test]
async fn a_replaced_connection_receives_no_removal() {
    safety(async {
        let peer = Peer::new(REMOVE_PROTOCOL, Response::Ack).await;
        let mut target = target(&peer);
        let expected = NodeId::random();
        target.node = expected;
        let error = phase("removal response", remove(&peer.client, target)).await.unwrap_err();
        assert!(
            matches!(error, RemovalError::NodeChanged { expected: kept, actual } if kept == expected && actual == peer.client.node().id)
        );
        peer.barrier().await;
        assert!(peer.removals().is_empty());
    }).await;
}

#[tokio::test]
async fn an_old_peer_receives_no_removal() {
    safety(async {
        let peer = Peer::new(ProtocolVersion::new(10), Response::Ack).await;
        assert!(matches!(
            phase("removal response", remove(&peer.client, target(&peer))).await,
            Err(RemovalError::OlderNode)
        ));
        peer.barrier().await;
        assert!(peer.removals().is_empty());
    })
    .await;
}

#[tokio::test]
async fn admin_and_profile_refusals_retain_the_wire_code_and_diagnostic() {
    safety(async {
        for (code, problem) in [
            (ErrorCode::Forbidden, Problem::Permission),
            (ErrorCode::Unauthorized, Problem::Permission),
            (ErrorCode::Conflict, Problem::ProviderInUse),
        ] {
            let peer = Peer::new(REMOVE_PROTOCOL, Response::Error(code)).await;
            let error = phase("removal response", remove(&peer.client, target(&peer)))
                .await
                .unwrap_err();
            assert_eq!(error.problem(), problem);
            assert!(!error.is_uncertain());
            let RemovalError::Call(CallError::Remote(wire)) = error else {
                panic!("wire source discarded")
            };
            assert_eq!(wire, WireError::new(code, "fixture diagnostic"));
            peer.barrier().await;
            assert_eq!(peer.removals().len(), 1);
        }
    })
    .await;
}

#[tokio::test]
async fn loss_after_dispatch_and_wrong_acknowledgement_are_uncertain_not_replayed() {
    safety(async {
        for response in [Response::Disconnect, Response::WrongMethod] {
            let mut peer = Peer::new(REMOVE_PROTOCOL, response).await;
            let error = phase("removal response", remove(&peer.client, target(&peer)))
                .await
                .unwrap_err();
            assert_eq!(error.problem(), Problem::RemovalUnknown);
            assert!(error.is_uncertain());
            assert!(matches!(
                phase("peer removal observation", peer.seen.recv()).await,
                Some(Call::RemoveAgent(_))
            ));
            assert_eq!(peer.removals().len(), 1);
        }
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn an_unacknowledged_call_releases_the_deadline_without_repeating_mutation() {
    safety(async {
        let mut peer = Peer::new(REMOVE_PROTOCOL, Response::Wait).await;
        let client = peer.client.clone();
        let target = target(&peer);
        let mut call = tokio::spawn(async move { remove(&client, target).await });
        let _cleanup = AbortOnDrop::new(&call);
        assert!(matches!(
            phase("peer removal observation", peer.seen.recv()).await,
            Some(Call::RemoveAgent(_))
        ));
        assert!(!call.is_finished());
        // Observation has finished its phase timer. Start the join phase only
        // after advancing product time; no active short phase can beat it.
        tokio::time::advance(REMOVE_TIMEOUT).await;
        assert!(matches!(
            phase("removal task completion", &mut call).await.unwrap(),
            Err(RemovalError::Timeout)
        ));
        peer.barrier().await;
        assert_eq!(peer.removals().len(), 1);
    })
    .await;
}

#[tokio::test]
async fn hub_busy_and_missing_connection_settle_the_exact_ticket_without_sending() {
    safety(async {
        let mut peer = Peer::new(REMOVE_PROTOCOL, Response::Wait).await;
        let hub = Hub::default();
        let machine = MachineKey::from_book("one");
        let request = RequestId::new(21);
        let (send, mut receive) = mpsc::channel(1);
        for index in 0..crate::agents::calls::MAX_TASKS {
            phase("hub command delivery", hub.ask(
                Some(peer.client.clone()),
                MachineKey::from_book(format!("occupied-{index}")),
                RequestId::ZERO,
                Ask::Remove(target(&peer)),
                send.clone(),
            )).await;
            assert!(matches!(phase("peer removal observation", peer.seen.recv()).await, Some(Call::RemoveAgent(_))));
        }
        phase("hub command delivery", hub.ask(
            Some(peer.client.clone()),
            machine.clone(),
            request,
            Ask::Remove(target(&peer)),
            send.clone(),
        )).await;
        assert!(
            matches!(phase("hub event delivery", receive.recv()).await, Some(Event::Agents {machine: actual,request: ticket, answer: Err(AgentsError::Busy)}) if actual == machine && ticket == request)
        );
        phase("hub command delivery", hub.ask(
            None,
            machine.clone(),
            request,
            Ask::Remove(target(&peer)),
            send,
        )).await;
        assert!(
            matches!(phase("hub event delivery", receive.recv()).await, Some(Event::Agents {machine: actual,request: ticket, answer: Err(AgentsError::Disconnected)}) if actual == machine && ticket == request)
        );
        peer.barrier().await;
        assert_eq!(peer.removals().len(), crate::agents::calls::MAX_TASKS);
    }).await;
}

#[test]
fn every_other_wire_failure_has_an_explicit_mutation_verdict() {
    for (code, problem, uncertain) in [
        (ErrorCode::InvalidRequest, Problem::Call, false),
        (ErrorCode::NotFound, Problem::Call, false),
        (ErrorCode::ProtocolMismatch, Problem::OlderNode, false),
        (ErrorCode::Unsupported, Problem::OlderNode, false),
        (ErrorCode::HarnessNotInstalled, Problem::Call, false),
        (ErrorCode::SpawnFailed, Problem::Call, false),
        (ErrorCode::Unavailable, Problem::RemovalUnknown, true),
        (ErrorCode::Internal, Problem::RemovalUnknown, true),
    ] {
        let error = RemovalError::Call(CallError::Remote(WireError::new(code, "machine prose")));
        assert_eq!(error.problem(), problem);
        assert_eq!(error.is_uncertain(), uncertain);
    }
    let error = RemovalError::Call(CallError::RequestIdsExhausted);
    assert_eq!(error.problem(), Problem::Call);
    assert!(!error.is_uncertain());
}

#[test]
fn an_unassigned_requirement_is_an_unsupported_unsent_call_not_an_uncertain_removal() {
    // RPC admission rejects this category before allocating an id/writer entry;
    // see vessel-rpc/src/client/protocol.rs. It is never a dispatched timeout.
    let error = RemovalError::Call(CallError::UnassignedProtocol {
        method: vessel_proto::Method::RemoveAgent,
    });
    assert_eq!(error.problem(), Problem::OlderNode);
    assert!(!error.is_uncertain());
    assert!(matches!(
        error,
        RemovalError::Call(CallError::UnassignedProtocol {
            method: vessel_proto::Method::RemoveAgent,
        })
    ));
}
