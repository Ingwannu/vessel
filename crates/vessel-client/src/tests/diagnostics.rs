//! Below its protocol, diagnostics never sends a frame, including wrapped calls.
use super::*;
use crate::CallError;
use futures_util::{SinkExt as _, StreamExt as _};
use vessel_proto::diagnostics::GetDiagnostics;
use vessel_proto::{Method, Origin, OriginBy, Request as _, ServerFrame, Welcome};

const SAFETY: std::time::Duration = std::time::Duration::from_secs(60);
const PHASE: std::time::Duration = std::time::Duration::from_secs(20);
#[tokio::test]
async fn diagnostics_below_its_protocol_direct_self_and_every_envelope_send_no_frame() {
    let needs = vessel_proto::diagnostics::DIAGNOSTICS_PROTOCOL.unwrap();
    let previous = vessel_proto::ProtocolVersion::new(needs.get().checked_sub(1).unwrap());
    vessel_testkit::within(SAFETY, async {
        for version in [vessel_proto::MIN_PROTOCOL_VERSION, previous] {
            let (client_end, mut node_end) =
                vessel_rpc::memory::pair(NonZeroUsize::new(4).unwrap());
            let info = node();
            let id = info.id;
            let peer = tokio::spawn(async move {
                let _hello = node_end.next().await.unwrap().unwrap();
                node_end
                    .send(
                        serde_json::to_string(&ServerFrame::Welcome(Welcome {
                            protocol: version,
                            node: info,
                        }))
                        .unwrap(),
                    )
                    .await
                    .unwrap();
                node_end.next().await
            });
            let client = tokio::time::timeout(
                PHASE,
                NodeClient::over(client_end, client_info(), &ClientConfig::default()),
            )
            .await
            .expect("diagnostic handshake")
            .unwrap();
            let error = tokio::time::timeout(PHASE, client.diagnostics())
                .await
                .expect("diagnostic client admission")
                .unwrap_err();
            assert!(matches!(
                error,
                crate::DiagnosticError::Call(CallError::NotInProtocol {
                    method: Method::Diagnostics,
                    needs: got,
                    negotiated,
                }) if got == needs && negotiated == version
            ));
            let error = tokio::time::timeout(PHASE, client.call_on(id, GetDiagnostics {}))
                .await
                .expect("diagnostic self wrapper")
                .unwrap_err();
            assert!(matches!(
                error,
                CallError::NotInProtocol {
                    method: Method::Forward,
                    needs: got,
                    negotiated,
                } if got == needs && negotiated == version
            ));
            let origin = Origin {
                node: id,
                by: OriginBy::Owner,
                scopes: Vec::new(),
            };
            for call in [
                GetDiagnostics {}.into_call(),
                vessel_proto::Forward {
                    node: id,
                    call: Box::new(GetDiagnostics {}.into_call()),
                }
                .into_call(),
                vessel_proto::RelayCall {
                    node: id,
                    origin: origin.clone(),
                    call: Box::new(GetDiagnostics {}.into_call()),
                }
                .into_call(),
                vessel_proto::Deliver {
                    origin,
                    call: Box::new(GetDiagnostics {}.into_call()),
                }
                .into_call(),
            ] {
                let method = call.method();
                let call = vessel_rpc::UnaryCall::try_from(call).unwrap();
                let error = tokio::time::timeout(PHASE, client.call_any(call))
                    .await
                    .expect("diagnostic recursive admission")
                    .unwrap_err();
                assert!(matches!(
                    error,
                    CallError::NotInProtocol {
                        method: got,
                        needs: required,
                        negotiated,
                    } if got == method && required == needs && negotiated == version
                ));
            }
            drop(client);
            assert!(
                tokio::time::timeout(PHASE, peer)
                    .await
                    .expect("diagnostic peer closure")
                    .unwrap()
                    .is_none()
            );
        }
    })
    .await
    .unwrap();
}
