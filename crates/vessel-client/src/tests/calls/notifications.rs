//! Real sender refusal: pending calls and a field added to push.presence
//! must not emit any request frame to a peer lacking the feature.
use super::*;
use vessel_proto::notifications::{
    ATTENTION_NOTIFICATIONS_PROTOCOL, ClearTerminalAttention, NotificationSource,
    ReportTerminalActivity, TerminalActivitySignal, TerminalAttentionRevision,
};
#[tokio::test]
async fn attention_contract_and_presence_field_are_refused_before_any_wire_request() {
    let protocol = ATTENTION_NOTIFICATIONS_PROTOCOL
        .map_or(vessel_proto::PROTOCOL_VERSION, |since| {
            ProtocolVersion::new(since.get().checked_sub(1).unwrap())
        });
    let (client_end, mut node_end) = vessel_rpc::memory::pair(NonZeroUsize::new(8).unwrap());
    let serving = tokio::spawn(async move {
        let _hello = node_end.next().await.unwrap().unwrap();
        node_end
            .send(
                serde_json::to_string(&ServerFrame::Welcome(Welcome {
                    protocol,
                    node: node(),
                }))
                .unwrap(),
            )
            .await
            .unwrap();
        node_end.next().await
    });
    let client = NodeClient::over(client_end, client_info(), &ClientConfig::default())
        .await
        .unwrap();
    let terminal = vessel_proto::TerminalId::random();
    let revision = TerminalAttentionRevision::new(1);
    let failures = [
        client
            .claim_notification(NotificationSource::Terminal { terminal, revision })
            .await
            .map(|_result| ())
            .unwrap_err(),
        client
            .clear_terminal_attention(ClearTerminalAttention { terminal, revision })
            .await
            .map(|_attention| ())
            .unwrap_err(),
        client
            .report_terminal_activity(ReportTerminalActivity {
                terminal,
                revision: Some(revision),
                signal: TerminalActivitySignal::Working,
            })
            .await
            .map(|_attention| ())
            .unwrap_err(),
        client
            .report_push_presence(vessel_proto::push::ReportPushPresence {
                idle_age: None,
                focus: vessel_proto::push::PushFocus::Terminal { terminal },
                activity: vessel_proto::push::PushActivity::Unchanged,
            })
            .await
            .unwrap_err(),
    ];
    for error in failures {
        match ATTENTION_NOTIFICATIONS_PROTOCOL {
            None => assert!(matches!(
                error,
                vessel_rpc::CallError::UnassignedProtocol { .. }
            )),
            Some(_) => assert!(matches!(error, vessel_rpc::CallError::NotInProtocol { .. })),
        }
    }
    drop(client);
    assert!(
        serving.await.unwrap().is_none(),
        "no unsupported call reached the transport"
    );
}
