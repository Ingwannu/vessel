//! Raw callers cannot bypass pending admission, including fleet envelopes.
use super::*;
use vessel_proto::prompt_index::{ListSessionPrompts, PROMPT_INDEX_PROTOCOL, PromptIndexLimit};
use vessel_proto::{Deliver, Forward, Origin, OriginBy, RelayCall};

#[tokio::test]
async fn pending_prompt_index_is_refused_before_dispatch_and_connection_survives() {
    let negotiated = PROMPT_INDEX_PROTOCOL.map_or(vessel_proto::PROTOCOL_VERSION, |version| {
        ProtocolVersion::new(version.get().checked_sub(1).unwrap())
    });
    let (mut client, mut served, target) = raw_client_with_node(negotiated.get()).await;
    let request = ListSessionPrompts {
        session: session(1),
        after: None,
        epoch: None,
        limit: PromptIndexLimit::default(),
    };
    let origin = Origin {
        node: target,
        by: OriginBy::Owner,
        scopes: vec![],
    };
    for (index, call) in [
        request.clone().into_call(),
        Forward {
            node: target,
            call: Box::new(request.clone().into_call()),
        }
        .into_call(),
        RelayCall {
            node: target,
            call: Box::new(request.clone().into_call()),
            origin: origin.clone(),
        }
        .into_call(),
        Deliver {
            call: Box::new(request.into_call()),
            origin,
        }
        .into_call(),
    ]
    .into_iter()
    .enumerate()
    {
        let method = call.method();
        let error = refusal(&call_raw(&mut client, u64::try_from(index).unwrap(), call).await);
        assert_eq!(error.code, ErrorCode::Unsupported);
        match PROMPT_INDEX_PROTOCOL {
            None => assert_eq!(
                error.message,
                CallError::UnassignedProtocol { method }.to_string()
            ),
            Some(needs) => assert!(error.message.contains(&format!("needs protocol {needs}"))),
        }
    }
    let text = call_raw(&mut client, 5, GetNode.into_call()).await;
    assert!(text.contains(r#""outcome":{"ok""#));
    assert_eq!(served.seen.recv().await.unwrap().method(), GetNode::METHOD);
    assert!(served.seen.try_recv().is_err());
}
