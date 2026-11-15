//! Staged Skills SDK refuses before transport work or request-id allocation.
use super::*;
use vessel_proto::skills::{
    ImportSkillsSelection, SaveSkillsSelection, SkillNames, SkillSelection,
};
use vessel_proto::{ClientFrame, Method, Outcome, Reply, RequestId, ServerFrame, Welcome};
use vessel_rpc::{CallError, Transport as _};

#[tokio::test]
async fn staged_skills_wrappers_leave_the_first_real_request_at_id_zero() {
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        let (client_end, mut peer) = vessel_rpc::memory::pair(NonZeroUsize::new(4).unwrap());
        let handshake = async move {
            let hello = peer.next().await.unwrap().unwrap();
            assert!(matches!(
                serde_json::from_str::<ClientFrame>(&hello).unwrap(),
                ClientFrame::Hello(_)
            ));
            peer.send(
                serde_json::to_string(&ServerFrame::Welcome(Welcome {
                    protocol: vessel_proto::PROTOCOL_VERSION,
                    node: node(),
                }))
                .unwrap(),
            )
            .await
            .unwrap();
            peer
        };
        let config = ClientConfig::default();
        let (client, mut peer) = tokio::join!(
            NodeClient::over(client_end, client_info(), &config),
            handshake,
        );
        let client = client.unwrap();
        let errors = [
            client.skills_status().await.unwrap_err(),
            client.reconcile_skills().await.unwrap_err(),
            client.uninstall_skills().await.unwrap_err(),
            client
                .save_skills_selection(SaveSkillsSelection {
                    selection: SkillSelection::All,
                    confirmed_removals: SkillNames::default(),
                })
                .await
                .unwrap_err(),
            client
                .import_skills_selection(ImportSkillsSelection {
                    selection: SkillSelection::All,
                })
                .await
                .unwrap_err(),
        ];
        for (error, method) in errors.into_iter().zip([
            Method::GetSkillsStatus,
            Method::ReconcileSkills,
            Method::UninstallSkills,
            Method::SaveSkillsSelection,
            Method::ImportSkillsSelection,
        ]) {
            assert_eq!(error, CallError::UnassignedProtocol { method });
        }
        // The accepted request is a writer barrier. An early empty-queue
        // observation could miss a frame that was merely delayed by scheduling.
        let first = async {
            let text = peer.next().await.unwrap().unwrap();
            let ClientFrame::Request { id, call } = serde_json::from_str(&text).unwrap() else {
                panic!("expected the first actual request");
            };
            assert_eq!(id, RequestId::new(0));
            assert!(matches!(call, vessel_proto::Call::NodeInfo(_)));
            peer.send(
                serde_json::to_string(&ServerFrame::Response {
                    id,
                    outcome: Outcome::Ok(Reply::NodeInfo(node())),
                })
                .unwrap(),
            )
            .await
            .unwrap();
        };
        let (reply, ()) = tokio::join!(client.call(GetNode), first);
        assert_eq!(reply.unwrap().name, "fake");
    })
    .await
    .unwrap();
}
