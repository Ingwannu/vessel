//! Registry requests below their birth are refused before handler dispatch.
//! JSON here is the explicit decoding-edge fixture, never a product domain type.
//! The recording fake proves protocol pre-dispatch, separately from node policy.

use super::*;
use serde_json::json;
use vessel_proto::{Deliver, Forward, Origin, OriginBy, RelayCall, Scope, WorkspaceId};

const CASE_DEADLINE: std::time::Duration = std::time::Duration::from_secs(5);
const FIRST_REQUEST: u64 = 1;

fn requests() -> Vec<Call> {
    let id = WorkspaceId::random().to_string();
    [
        ("workspace.registry", json!({})),
        ("workspace.list", json!({})),
        ("workspace.get", json!({"workspace":id})),
        ("workspace.rename", json!({"workspace":id})),
        ("workspace.pin", json!({"workspace":id,"pin":"unpinned"})),
        (
            "workspace.release_automation",
            json!({
                "workspace":id,"expected":0,"source":{
                    "kind":"change_request","number":1,
                    "head_repository":{"kind":"github","host":"github.com","path":"fork/project"}
                }
            }),
        ),
    ]
    .into_iter()
    .map(|(method, params)| {
        serde_json::from_value(json!({"method":method,"params":params})).unwrap()
    })
    .collect()
}

#[tokio::test]
async fn registry_metadata_raw_direct_and_nested_requests_refuse_before_dispatch_below_birth() {
    let needed = vessel_proto::registry::PROJECT_WORKSPACE_REGISTRY.unwrap();
    let previous = ProtocolVersion::new(needed.get().checked_sub(1).unwrap());
    let mut versions = vec![vessel_proto::MIN_PROTOCOL_VERSION, previous];
    versions.sort();
    versions.dedup();
    for version in versions {
        tokio::time::timeout(CASE_DEADLINE, scenario(version))
            .await
            .unwrap();
    }
}

async fn scenario(version: ProtocolVersion) {
    let (mut client, mut served, target) = raw_client_with_node(version.get()).await;
    let origin = Origin {
        node: target,
        by: OriginBy::Owner,
        scopes: Scope::ALL.to_vec(),
    };
    let mut next = FIRST_REQUEST;
    for inner in requests() {
        for call in [
            inner.clone(),
            Forward {
                node: target,
                call: Box::new(inner.clone()),
            }
            .into_call(),
            RelayCall {
                node: target,
                call: Box::new(inner.clone()),
                origin: origin.clone(),
            }
            .into_call(),
            Deliver {
                call: Box::new(inner),
                origin: origin.clone(),
            }
            .into_call(),
        ] {
            let text = call_raw(&mut client, next, call).await;
            let ServerFrame::Response {
                id,
                outcome: Outcome::Err(error),
            } = serde_json::from_str(&text).unwrap()
            else {
                panic!("expected a correlated protocol refusal");
            };
            assert_eq!(id, RequestId::new(next));
            assert_eq!(error.code, ErrorCode::Unsupported);
            // Even unserved Fake methods record their calls, so empty is a
            // real pre-dispatch check rather than a default Unsupported echo.
            assert!(served.seen.try_recv().is_err());
            next = next.checked_add(1).unwrap();
        }
    }
    let text = call_raw(&mut client, next, GetNode.into_call()).await;
    let ServerFrame::Response {
        id,
        outcome: Outcome::Ok(reply),
    } = serde_json::from_str(&text).unwrap()
    else {
        panic!("expected the supported negative control");
    };
    assert_eq!(id, RequestId::new(next));
    assert_eq!(reply.method(), Method::NodeInfo);
    assert_eq!(served.seen.recv().await.unwrap().method(), Method::NodeInfo);
    assert!(served.seen.try_recv().is_err());
    served.shutdown.cancel();
    drop(client);
    served.task.await.unwrap().unwrap();
    // This generic Fake returns a fresh id for node_info; only the Welcome
    // target was used for envelope identity. Real node checks are separate.
}
