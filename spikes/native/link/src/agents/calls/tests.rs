//! An old machine alias cannot redirect a credential check or installation.
use super::*;
use crate::agents::testing::{Mode, Peer, phase, safety};
use vessel_proto::{
    Call, HarnessKind,
    agents::{InstallAgent, InstallPart},
};
#[tokio::test]
async fn checks_and_installers_refuse_a_different_actual_node_before_any_action_frame() {
    safety(async {
        for installer in [false, true] {
            let mut peer = Peer::new(vessel_proto::PROTOCOL_VERSION, Mode::Catalogue).await;
            let expected = "00000000-0000-4000-8000-000000000002".parse().unwrap();
            let actual = peer.client.node().id;
            let ask = if installer {
                Ask::Install {
                    node: expected,
                    install: InstallAgent {
                        agent: HarnessKind::Codex,
                        part: InstallPart::Adapter,
                    },
                }
            } else {
                Ask::Check {
                    node: expected,
                    agent: HarnessKind::ClaudeCode.into(),
                }
            };
            let (send, mut receive) = mpsc::channel(1);
            let machine = MachineKey::from_book("original-machine");
            let ticket = RequestId::ZERO.next().unwrap();
            phase(
                "guarded action admission",
                Hub::default().ask(
                    Some(peer.client.clone()),
                    machine.clone(),
                    ticket,
                    ask,
                    send,
                ),
            )
            .await;
            let Event::Agents {
                machine: reply_machine,
                request,
                answer:
                    Err(AgentsError::TargetChanged {
                        expected: retained,
                        actual: found,
                    }),
            } = phase("guarded action refusal", receive.recv())
                .await
                .unwrap()
            else {
                panic!("stale action did not return a typed node refusal")
            };
            assert_eq!(
                (reply_machine, request, retained, found),
                (machine, ticket, expected, actual)
            );
            peer.barrier().await;
            assert!(matches!(
                phase("no action frame barrier", peer.seen.recv()).await,
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
