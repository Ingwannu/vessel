//! A forced call below its protocol is refused on the welcomed connection before dispatch.
use super::*;
use vessel_proto::diagnostics::GetDiagnostics;
use vessel_proto::{Origin, OriginBy};
const SAFETY: std::time::Duration = std::time::Duration::from_secs(60);
const PHASE: std::time::Duration = std::time::Duration::from_secs(20);
#[tokio::test]
async fn diagnostics_below_their_protocol_refuse_correlated_direct_and_nested_before_dispatch() {
    let needs = vessel_proto::diagnostics::DIAGNOSTICS_PROTOCOL.unwrap();
    let previous = ProtocolVersion::new(needs.get().checked_sub(1).unwrap());
    vessel_testkit::within(SAFETY, async {
        for version in [vessel_proto::MIN_PROTOCOL_VERSION, previous] {
            let (mut client, mut served, node) = tokio::time::timeout(PHASE, raw_client_with_node(version.get())).await.expect("raw diagnostic welcome");
            let origin = Origin { node, by: OriginBy::Owner, scopes: Vec::new() };
            let cases = [
                GetDiagnostics {}.into_call(),
                vessel_proto::Forward { node, call: Box::new(GetDiagnostics {}.into_call()) }.into_call(),
                vessel_proto::RelayCall { node, origin: origin.clone(), call: Box::new(GetDiagnostics {}.into_call()) }.into_call(),
                vessel_proto::Deliver { origin, call: Box::new(GetDiagnostics {}.into_call()) }.into_call(),
            ];
            for (index, call) in cases.into_iter().enumerate() {
                let id = u64::try_from(index).unwrap().checked_add(1).unwrap();
                let text = tokio::time::timeout(PHASE, call_raw(&mut client, id, call)).await.expect("raw diagnostic refusal");
                let frame = serde_json::from_str::<ServerFrame>(&text).unwrap();
                let ServerFrame::Response { id: got, outcome: Outcome::Err(error) } = frame else { panic!("expected correlated refusal") };
                assert_eq!(got, RequestId::new(id));
                assert_eq!(error.code, ErrorCode::Unsupported);
                assert!(served.seen.try_recv().is_err(), "a refused handler must not run");
            }
            let text = tokio::time::timeout(PHASE, call_raw(&mut client, u64::MAX, GetNode.into_call())).await.expect("supported call after refusals");
            assert!(matches!(serde_json::from_str::<ServerFrame>(&text).unwrap(), ServerFrame::Response { id, outcome: Outcome::Ok(Reply::NodeInfo(info)) } if id==RequestId::new(u64::MAX) && info.state==NodeState::Serving));
            assert_eq!(tokio::time::timeout(PHASE, served.seen.recv()).await.expect("supported dispatch observed").unwrap(), GetNode.into_call());
            served.shutdown.cancel();
            drop(client);
            tokio::time::timeout(PHASE, served.task).await.expect("diagnostic server shutdown").unwrap().unwrap();
        }
    }).await.unwrap();
}
