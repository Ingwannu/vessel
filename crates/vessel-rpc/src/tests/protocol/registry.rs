//! Registry requests below their birth are refused before handler dispatch.
//! JSON here is the explicit decoding-edge fixture, never a product domain type.
//! The recording fake proves protocol pre-dispatch, separately from node policy.

use super::*;
use serde_json::json;
use std::future::Future;
use std::panic::{AssertUnwindSafe, resume_unwind};
use vessel_proto::{Deliver, Forward, Origin, OriginBy, RelayCall, Scope, WorkspaceId};

const CASE_DEADLINE: std::time::Duration = std::time::Duration::from_secs(5);
const PHASE_DEADLINE: std::time::Duration = std::time::Duration::from_secs(1);

async fn phase<T>(name: &str, future: impl Future<Output = T>) -> T {
    tokio::time::timeout(PHASE_DEADLINE, future)
        .await
        .unwrap_or_else(|_elapsed| panic!("registry raw phase: {name}"))
}

enum RawCase {
    Old,
    Current,
}

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
        run(version, RawCase::Old).await;
    }
}

async fn scenario(client: &mut MemoryTransport, served: &mut Served, target: NodeId) {
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
            let text = request(client, next, call).await;
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
    let text = request(client, next, GetNode.into_call()).await;
    let ServerFrame::Response {
        id,
        outcome: Outcome::Ok(reply),
    } = serde_json::from_str(&text).unwrap()
    else {
        panic!("expected the supported negative control");
    };
    assert_eq!(id, RequestId::new(next));
    assert_eq!(reply.method(), Method::NodeInfo);
    assert_eq!(
        phase("recorded dispatch", served.seen.recv())
            .await
            .unwrap()
            .method(),
        Method::NodeInfo
    );
    assert!(served.seen.try_recv().is_err());

    // This generic Fake returns a fresh id for node_info; only the Welcome
    // target was used for envelope identity. Real node checks are separate.
}

#[tokio::test]
async fn registry_metadata_current_raw_calls_ignore_future_params_and_reach_dispatch() {
    run(vessel_proto::PROTOCOL_VERSION, RawCase::Current).await;
}

async fn current(client: &mut MemoryTransport, served: &mut Served) {
    let mut next = FIRST_REQUEST;
    for expected in requests() {
        // Explicit wire decoding edge: future request fields remain forward compatible.
        let frame = ClientFrame::Request {
            id: RequestId::new(next),
            call: expected.clone(),
        };
        let mut raw = serde_json::to_value(frame).unwrap();
        let params = raw
            .get_mut("call")
            .unwrap()
            .get_mut("params")
            .unwrap()
            .as_object_mut()
            .unwrap();
        assert!(
            params
                .insert("future_optional".to_owned(), json!({"nested":true}))
                .is_none()
        );
        phase(
            "future-field request frame",
            client.send(serde_json::to_string(&raw).unwrap()),
        )
        .await
        .unwrap();
        let response = phase("future-field response frame", client.next())
            .await
            .unwrap()
            .unwrap();
        let ServerFrame::Response {
            id,
            outcome: Outcome::Err(error),
        } = serde_json::from_str(&response).unwrap()
        else {
            panic!("the recording fake has no metadata implementation")
        };
        assert_eq!(id, RequestId::new(next));
        assert_eq!(error.code, ErrorCode::Unsupported);
        assert_eq!(
            phase("recorded dispatch", served.seen.recv())
                .await
                .unwrap(),
            expected
        );
        next = next.checked_add(1).unwrap();
    }
}

// The task and shutdown token outlive all assertions and the case timeout.
// Cleanup is awaited before rethrowing a failure, so a bad frame cannot leave
// an in-memory server detached for the rest of the test binary.
async fn run(version: ProtocolVersion, case: RawCase) {
    let (mut client, transport) = pair();
    let mut served = serve(transport);
    let result = vessel_testkit::within(
        CASE_DEADLINE,
        AssertUnwindSafe(async {
            let target = handshake(&mut client, version).await;
            match case {
                RawCase::Old => scenario(&mut client, &mut served, target).await,
                RawCase::Current => current(&mut client, &mut served).await,
            }
        })
        .catch_unwind(),
    )
    .await;
    served.shutdown.cancel();
    drop(client);
    let joined = tokio::time::timeout(PHASE_DEADLINE, &mut served.task).await;
    let joined = match joined {
        Ok(joined) => joined,
        Err(_elapsed) => {
            served.task.abort();
            let _cancelled = phase("aborted server join", &mut served.task).await;
            panic!("registry raw phase: server close/join");
        }
    };
    if let Err(panic) = result.expect("registry raw case deadline") {
        resume_unwind(panic);
    }
    joined.unwrap().unwrap();
}

async fn handshake(client: &mut MemoryTransport, version: ProtocolVersion) -> NodeId {
    let hello = ClientFrame::Hello(Hello {
        protocol: ProtocolRange {
            min: version,
            max: version,
        },
        client: client_info(),
    });
    phase(
        "Hello send",
        client.send(serde_json::to_string(&hello).unwrap()),
    )
    .await
    .unwrap();
    let welcome = phase("Welcome receive", client.next())
        .await
        .unwrap()
        .unwrap();
    let ServerFrame::Welcome(welcome) = serde_json::from_str(&welcome).unwrap() else {
        panic!("expected a Welcome");
    };
    assert_eq!(welcome.protocol, version);
    welcome.node.id
}

async fn request(client: &mut MemoryTransport, id: u64, call: Call) -> String {
    let request = ClientFrame::Request {
        id: RequestId::new(id),
        call,
    };
    phase(
        "request send",
        client.send(serde_json::to_string(&request).unwrap()),
    )
    .await
    .unwrap();
    phase("response receive", client.next())
        .await
        .unwrap()
        .unwrap()
}
