//! Real memory-transport calls; no node process, package runner or network.
//! Root testkit's safety helper builds fakes in another workspace first.
//! These tests need no fake executable; retain finite outer/named phases
//! through the native workspace's existing Tokio dev dependency instead.
use super::*;
use crate::agents::{Answer, Ask, CatalogueFetch, Hub, MachineKey};
use tokio::sync::mpsc;
use vessel_proto::{Call, ProtocolVersion, RequestId};

use crate::agents::testing as peer;
use peer::{Abort, Mode, Peer, phase, safety};

#[tokio::test]
async fn registry_reads_echo_the_real_node_and_original_hub_ticket_without_installing() {
    safety(async {
        for fetch in [CatalogueFetch::Cached, CatalogueFetch::Refresh] {
            let mut peer = Peer::new(vessel_proto::PROTOCOL_VERSION, Mode::Catalogue).await;
            let node = peer.client.node().id;
            let machine = MachineKey::from_book("registry-route");
            let ticket = RequestId::ZERO.next().unwrap();
            let (send, mut receive) = mpsc::channel(1);
            phase(
                "registry hub dispatch",
                Hub::default().ask(
                    Some(peer.client.clone()),
                    machine.clone(),
                    ticket,
                    Ask::Catalogue { node, fetch },
                    send,
                ),
            )
            .await;
            let crate::Event::Agents {
                machine: replied,
                request,
                answer:
                    Ok(Answer::Catalogue {
                        node: actual,
                        catalogue,
                    }),
            } = phase("registry hub answer", receive.recv()).await.unwrap()
            else {
                panic!("registry read did not return its typed catalogue")
            };
            assert_eq!(replied, machine);
            assert_eq!(request, ticket);
            assert_eq!(actual, node);
            assert_eq!(catalogue.fetched_at, Some(chrono::DateTime::UNIX_EPOCH));
            assert_eq!(catalogue.skipped, 1);
            let Call::GetCatalogue(call) = phase("registry request observation", peer.seen.recv())
                .await
                .unwrap()
            else {
                panic!("a registry read must not start installation or another operation")
            };
            assert_eq!(call.refresh, matches!(fetch, CatalogueFetch::Refresh));
            peer.barrier().await;
            assert!(matches!(
                phase("registry ordered read", peer.seen.recv()).await,
                Some(Call::NodeInfo(_))
            ));
            assert!(matches!(
                peer.seen.try_recv(),
                Err(mpsc::error::TryRecvError::Empty)
            ));
        }
    })
    .await;
}

#[tokio::test]
async fn changed_identity_and_older_protocol_are_refused_before_any_registry_frame() {
    safety(async {
        for protocol in [vessel_proto::PROTOCOL_VERSION, ProtocolVersion::new(10)] {
            let mut peer = Peer::new(protocol, Mode::Catalogue).await;
            let actual = peer.client.node().id;
            let expected = if protocol == vessel_proto::PROTOCOL_VERSION {
                "00000000-0000-4000-8000-000000000002".parse().unwrap()
            } else { actual };
            let error = phase("registry preflight refusal", read(&peer.client, expected, Fetch::Refresh))
                .await.unwrap_err();
            if protocol == vessel_proto::PROTOCOL_VERSION {
                assert!(matches!(error, CatalogueError::NodeChanged { expected: retained, actual: found }
                    if retained == expected && found == actual));
            } else {
                assert!(matches!(error, CatalogueError::Call(CallError::NotInProtocol { .. })));
            }
            peer.barrier().await;
            assert!(matches!(phase("registry no-frame barrier", peer.seen.recv()).await, Some(Call::NodeInfo(_))));
            assert!(matches!(peer.seen.try_recv(), Err(mpsc::error::TryRecvError::Empty)));
        }
    }).await;
}

#[tokio::test]
async fn registry_refusals_preserve_their_wire_source_instead_of_becoming_empty() {
    safety(async {
        for (code, problem) in [
            (ErrorCode::Forbidden, Problem::Permission),
            (ErrorCode::Unauthorized, Problem::Permission),
            (ErrorCode::Unsupported, Problem::Unsupported),
            (ErrorCode::InvalidRequest, Problem::Read),
            (ErrorCode::Unavailable, Problem::Read),
        ] {
            let peer = Peer::new(vessel_proto::PROTOCOL_VERSION, Mode::RefuseCatalogue(code)).await;
            let error = phase(
                "registry node refusal",
                read(&peer.client, peer.client.node().id, Fetch::Refresh),
            )
            .await
            .unwrap_err();
            assert_eq!(error.problem(), problem);
            let CatalogueError::Call(CallError::Remote(source)) = error else {
                panic!("original node refusal was discarded")
            };
            assert_eq!(source.code, code);
            assert_eq!(source.message, peer::DIAGNOSTIC);
        }
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn registry_deadline_ends_the_wait_without_replaying_the_request() {
    safety(async {
        let mut peer = Peer::new(vessel_proto::PROTOCOL_VERSION, Mode::WaitCatalogue).await;
        let client = peer.client.clone();
        let node = client.node().id;
        let pending = tokio::spawn(async move { read(&client, node, Fetch::Refresh).await });
        let _abort = Abort::of(&pending);
        assert!(matches!(
            phase("registry request before time advance", peer.seen.recv()).await,
            Some(Call::GetCatalogue(_))
        ));
        // No short observation phase remains active across this intentional
        // advance. The peer idle and outer budgets include this product time.
        tokio::time::advance(CATALOGUE_TIMEOUT).await;
        assert!(matches!(
            phase("registry completion after deadline", pending)
                .await
                .unwrap(),
            Err(CatalogueError::Timeout)
        ));
        peer.barrier().await;
        assert!(matches!(
            phase("registry no-resend barrier", peer.seen.recv()).await,
            Some(Call::NodeInfo(_))
        ));
        assert!(matches!(
            peer.seen.try_recv(),
            Err(mpsc::error::TryRecvError::Empty)
        ));
    })
    .await;
}
