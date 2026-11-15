//! Raw requests prove pending/old admission on the real node service and every wrapper.
use super::*;
use crate::{principal::Principal, service::Connection};
use futures_util::{SinkExt as _, StreamExt as _};
use std::num::NonZeroUsize;
use tokio_util::sync::CancellationToken;
use vessel_proto::launch_profiles::*;
use vessel_proto::{
    Call, ClientFrame, ClientInfo, Deliver, ErrorCode, Forward, Handler as _, Hello, Origin,
    OriginBy, Outcome, ProtocolRange, ProtocolVersion, RelayCall, Request as _, RequestId,
    ServerFrame,
};
use vessel_rpc::{Server, ServerConfig, memory};

fn calls(session: vessel_proto::SessionId) -> Vec<Call> {
    vec![
        ListLaunchProfiles {}.into_call(),
        serde_json::from_str::<ReplaceLaunchProfiles>(
            vessel_testkit::launch_profile_fixtures::REPLACE,
        )
        .unwrap()
        .into_call(),
        GetLaunchProfileOptions {
            selection: LaunchSelection::new(
                vessel_proto::HarnessKind::ClaudeCode.into(),
                None,
                LaunchChoices::default(),
            ),
        }
        .into_call(),
        GetApplicableLaunchProfiles { session }.into_call(),
        ApplySessionOptions {
            session,
            options: LaunchChoices::default(),
        }
        .into_call(),
    ]
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pending_or_old_direct_forward_relay_and_deliver_fail_before_real_dispatch() {
    vessel_testkit::within(
        SAFETY,
        Box::pin(async {
            let fixture = Fixture::new().await;
            let latest_unavailable = NAMED_LAUNCH_PROFILES
                .map_or(vessel_proto::PROTOCOL_VERSION, |since| {
                    ProtocolVersion::new(since.get().checked_sub(1).unwrap())
                });
            for version in [vessel_proto::MIN_PROTOCOL_VERSION, latest_unavailable] {
                let (mut client, stop, serving) = raw_connection(&fixture, version).await;
                check_wrapped_calls(&fixture, &mut client).await;
                stop.cancel();
                phase("raw server shutdown", serving).await.unwrap();
            }
            // None of the raw replacements reached persistence.
            let owner = fixture.context(Principal::Owner).await;
            let saved = phase(
                "unchanged store after admission",
                fixture
                    .service()
                    .list_launch_profiles(&owner.connection, ListLaunchProfiles {}),
            )
            .await
            .unwrap();
            assert_eq!(saved.revision(), LaunchProfileRevision::ZERO);
            owner.finish().await;
            fixture.finish().await;
        }),
    )
    .await
    .unwrap();
}

async fn raw_connection(
    fixture: &Fixture,
    version: ProtocolVersion,
) -> (
    memory::MemoryTransport,
    CancellationToken,
    tokio::task::JoinHandle<()>,
) {
    let stop = CancellationToken::new();
    let (_info, watch) = tokio::sync::watch::channel(fixture.node.as_ref().unwrap().info());
    let server = Server::new(
        fixture.service(),
        watch,
        ServerConfig::default(),
        stop.clone(),
    );
    let (mut client, transport) = memory::pair(NonZeroUsize::new(16).unwrap());
    let serving = tokio::spawn(async move {
        server.serve(transport, Connection::owner).await.unwrap();
    });
    let hello = ClientFrame::Hello(Hello {
        protocol: ProtocolRange {
            min: version,
            max: version,
        },
        client: ClientInfo {
            name: "launch-raw-test".into(),
            version: "0".into(),
        },
    });
    phase(
        "raw hello",
        client.send(serde_json::to_string(&hello).unwrap()),
    )
    .await
    .unwrap();
    let welcome = phase("raw welcome", client.next()).await.unwrap().unwrap();
    assert!(matches!(
        serde_json::from_str::<ServerFrame>(&welcome).unwrap(),
        ServerFrame::Welcome(_)
    ));
    (client, stop, serving)
}

async fn check_wrapped_calls(fixture: &Fixture, client: &mut memory::MemoryTransport) {
    let node = fixture.node.as_ref().unwrap().info().id;
    let origin = Origin {
        node,
        by: OriginBy::Owner,
        scopes: vessel_proto::Scope::ALL.to_vec(),
    };
    let mut seq = 0_u64;
    for call in calls(vessel_proto::SessionId::random()) {
        for wrapped in [
            call.clone(),
            Forward {
                node,
                call: Box::new(call.clone()),
            }
            .into_call(),
            RelayCall {
                node,
                call: Box::new(call.clone()),
                origin: origin.clone(),
            }
            .into_call(),
            Deliver {
                call: Box::new(call.clone()),
                origin: origin.clone(),
            }
            .into_call(),
        ] {
            seq = seq.checked_add(1).unwrap();
            let request = ClientFrame::Request {
                id: RequestId::new(seq),
                call: wrapped,
            };
            phase(
                "raw pending request",
                client.send(serde_json::to_string(&request).unwrap()),
            )
            .await
            .unwrap();
            let response = phase("raw pending refusal", client.next())
                .await
                .unwrap()
                .unwrap();
            let ServerFrame::Response {
                id,
                outcome: Outcome::Err(error),
            } = serde_json::from_str::<ServerFrame>(&response).unwrap()
            else {
                panic!("unexpected response")
            };
            assert_eq!(id, RequestId::new(seq));
            assert_eq!(error.code, ErrorCode::Unsupported);
        }
    }
}
