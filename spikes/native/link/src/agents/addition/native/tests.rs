//! Memory RPC only: no actual file mutation, permission grant or agent program.
use super::*;
use crate::agents::{
    Ask, Hub, MachineKey,
    testing::{self, Abort, Mode, Peer, phase, safety},
};
use tokio::sync::mpsc;
use vessel_proto::{
    Call, ProtocolVersion, RequestId,
    agents::{AddAgent, AgentToAdd},
};

fn target(peer: &Peer) -> Result<Addition, Box<dyn std::error::Error>> {
    Ok(Addition {
        node: peer.client.node().id,
        call: AddAgent {
            name: "fixture-added".parse()?,
            agent: AgentToAdd::Registry {
                id: "fixture".parse()?,
                version: "1.0.0".parse()?,
                mcp: Vec::new(),
            },
            replace: false,
        },
    })
}
#[tokio::test]
async fn hub_add_echoes_route_ticket_and_actual_identity_without_discovery_or_install() {
    Box::pin(safety(async {
        let mut peer=Peer::new(vessel_proto::PROTOCOL_VERSION,Mode::Add).await;
        let expected=target(&peer).unwrap();let node=expected.node;let call=expected.call.clone();
        let machine=MachineKey::from_book("original-add-route");let ticket=RequestId::ZERO.next().unwrap();
        let (send,mut receive)=mpsc::channel(1);
        phase("addition hub admission",Hub::default().ask(Some(peer.client.clone()),machine.clone(),ticket,Ask::Add(expected),send)).await;
        let crate::Event::Agents {machine:actual,request,answer:Ok(Answer::Added {node:actual_node,info})}=phase("addition hub event",receive.recv()).await.unwrap() else {panic!("addition did not return its typed ACK")};
        assert_eq!((actual,request,actual_node),(machine,ticket,node));assert!(matches_ack(&call,&info));
        assert!(matches!(phase("addition observed frame",peer.seen.recv()).await,Some(Call::AddAgent(actual)) if actual==call && !actual.replace));
        peer.barrier().await;
        assert!(matches!(phase("addition ordered barrier",peer.seen.recv()).await,Some(Call::NodeInfo(_))));
        assert!(matches!(peer.seen.try_recv(),Err(mpsc::error::TryRecvError::Empty)));
    })).await;
}
#[tokio::test]
async fn changed_node_and_older_protocol_emit_no_add_frame() {
    safety(async {
        for version in [vessel_proto::PROTOCOL_VERSION, ProtocolVersion::new(10)] {
            let mut peer = Peer::new(version, Mode::Add).await;
            let mut request = target(&peer).unwrap();
            if version == vessel_proto::PROTOCOL_VERSION {
                request.node = "00000000-0000-4000-8000-000000000002".parse().unwrap();
            }
            let error = phase("addition preflight", add(&peer.client, request))
                .await
                .unwrap_err();
            if version == vessel_proto::PROTOCOL_VERSION {
                assert!(matches!(error, AdditionError::NodeChanged { .. }));
            } else {
                assert!(matches!(
                    error,
                    AdditionError::Call(CallError::NotInProtocol { .. })
                ));
            }
            assert!(!error.is_uncertain());
            peer.barrier().await;
            assert!(matches!(
                phase("addition no-frame barrier", peer.seen.recv()).await,
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
async fn wire_refusals_keep_the_original_code_and_distinguish_uncertain_writes() {
    safety(async {
        for (code, problem) in [
            (ErrorCode::Forbidden, Failure::Permission),
            (ErrorCode::Unauthorized, Failure::Permission),
            (ErrorCode::Conflict, Failure::Conflict),
            (ErrorCode::InvalidRequest, Failure::Rejected),
            (ErrorCode::Unsupported, Failure::Unsupported),
            (ErrorCode::Unavailable, Failure::Unknown),
            (ErrorCode::Internal, Failure::Unknown),
        ] {
            let mut peer = Peer::new(vessel_proto::PROTOCOL_VERSION, Mode::RefuseAdd(code)).await;
            let error = phase(
                "addition node refusal",
                add(&peer.client, target(&peer).unwrap()),
            )
            .await
            .unwrap_err();
            assert_eq!(error.problem(), problem);
            assert_eq!(error.is_uncertain(), problem == Failure::Unknown);
            let AdditionError::Call(CallError::Remote(source)) = error else {
                panic!("original addition refusal discarded")
            };
            assert_eq!(source.code, code);
            assert_eq!(source.message, testing::DIAGNOSTIC);
            assert!(matches!(
                phase("refused addition observation", peer.seen.recv()).await,
                Some(Call::AddAgent(_))
            ));
            peer.barrier().await;
            assert!(matches!(
                phase("refusal no-resend barrier", peer.seen.recv()).await,
                Some(Call::NodeInfo(_))
            ));
            assert!(matches!(
                peer.seen.try_recv(),
                Err(mpsc::error::TryRecvError::Empty)
            ));
        }
        let peer = Peer::new(vessel_proto::PROTOCOL_VERSION, Mode::WrongAdd).await;
        assert!(matches!(
            phase(
                "wrong addition ACK",
                add(&peer.client, target(&peer).unwrap())
            )
            .await,
            Err(AdditionError::Acknowledgement)
        ));
    })
    .await;
}
#[tokio::test(start_paused = true)]
async fn deadline_ends_the_wait_and_never_replays_or_probes() {
    safety(async {
        let mut peer = Peer::new(vessel_proto::PROTOCOL_VERSION, Mode::WaitAdd).await;
        let client = peer.client.clone();
        let request = target(&peer).unwrap();
        let task = tokio::spawn(async move { add(&client, request).await });
        let _abort = Abort::of(&task);
        assert!(matches!(
            phase("addition before time advance", peer.seen.recv()).await,
            Some(Call::AddAgent(_))
        ));
        // No short phase remains active across the intentional product deadline.
        tokio::time::advance(ADD_TIMEOUT).await;
        assert!(matches!(
            phase("addition after deadline", task).await.unwrap(),
            Err(AdditionError::Timeout)
        ));
        peer.barrier().await;
        assert!(matches!(
            phase("deadline no-resend barrier", peer.seen.recv()).await,
            Some(Call::NodeInfo(_))
        ));
        assert!(matches!(
            peer.seen.try_recv(),
            Err(mpsc::error::TryRecvError::Empty)
        ));
    })
    .await;
}

#[test]
fn local_admission_failures_do_not_blame_the_machine_or_imply_a_write() {
    for (call, expected) in [
        (
            CallError::UnassignedProtocol {
                method: vessel_proto::Method::AddAgent,
            },
            Failure::BuildUnavailable,
        ),
        (CallError::RequestIdsExhausted, Failure::Exhausted),
        (
            CallError::NotInProtocol {
                method: vessel_proto::Method::AddAgent,
                needs: ProtocolVersion::new(11),
                negotiated: ProtocolVersion::new(10),
            },
            Failure::Unsupported,
        ),
    ] {
        let error = AdditionError::Call(call);
        assert_eq!(error.problem(), expected);
        assert!(!error.is_uncertain());
        assert!(matches!(error, AdditionError::Call(_)));
    }
}
