//! Pending/older admission refuses actual typed client calls before any frame.

use super::*;
use crate::launch_profiles::LaunchProfileClientError;
use futures_util::{SinkExt as _, StreamExt as _};
use std::{future::Future, time::Duration};
use vessel_proto::launch_profiles::*;
use vessel_proto::{Method, ProtocolVersion, ServerFrame, Welcome};

const PHASE: Duration = Duration::from_secs(5);
async fn phase<T>(name: &str, future: impl Future<Output = T>) -> T {
    tokio::time::timeout(PHASE, future)
        .await
        .unwrap_or_else(|_| panic!("phase timed out: {name}"))
}
fn refusal(error: CallError, method: Method, negotiated: ProtocolVersion) {
    match NAMED_LAUNCH_PROFILES {
        None => assert!(
            matches!(error, CallError::UnassignedProtocol { method: actual } if actual == method)
        ),
        Some(needed) => assert!(
            matches!(error, CallError::NotInProtocol { method: actual, needs, negotiated: actual_version } if actual == method && needs == needed && actual_version == negotiated)
        ),
    }
}

#[tokio::test]
async fn launch_profile_methods_and_nested_calls_are_never_sent_before_availability() {
    vessel_testkit::within(
        Duration::from_secs(15),
        Box::pin(async {
            let negotiated = NAMED_LAUNCH_PROFILES
                .map_or(vessel_proto::PROTOCOL_VERSION, |since| {
                    ProtocolVersion::new(since.get().checked_sub(1).unwrap())
                });
            let (client_end, mut node_end) =
                vessel_rpc::memory::pair(NonZeroUsize::new(4).unwrap());
            let peer = tokio::spawn(async move {
                let _hello = phase("client hello", node_end.next())
                    .await
                    .unwrap()
                    .unwrap();
                let welcome = ServerFrame::Welcome(Welcome {
                    protocol: negotiated,
                    node: node(),
                });
                phase(
                    "peer welcome",
                    node_end.send(serde_json::to_string(&welcome).unwrap()),
                )
                .await
                .unwrap();
                phase("no post-handshake frame", node_end.next()).await
            });
            let client = phase(
                "typed client connection",
                NodeClient::over(client_end, client_info(), &ClientConfig::default()),
            )
            .await
            .unwrap();
            refusal(
                phase("list admission", client.launch_profiles())
                    .await
                    .unwrap_err(),
                Method::ListLaunchProfiles,
                negotiated,
            );
            refusal(
                phase(
                    "replace admission",
                    client.replace_launch_profiles(
                        LaunchProfileRevision::ZERO,
                        LaunchProfiles::default(),
                    ),
                )
                .await
                .unwrap_err(),
                Method::ReplaceLaunchProfiles,
                negotiated,
            );
            let selection =
                LaunchSelection::new(HarnessKind::Codex.into(), None, LaunchChoices::default());
            refusal(
                phase(
                    "options admission",
                    client.launch_profile_options(selection),
                )
                .await
                .unwrap_err(),
                Method::GetLaunchProfileOptions,
                negotiated,
            );
            refusal(
                phase(
                    "eligibility admission",
                    client.applicable_launch_profiles(session_id()),
                )
                .await
                .unwrap_err(),
                Method::GetApplicableLaunchProfiles,
                negotiated,
            );
            let LaunchProfileClientError::Call(error) = phase(
                "bundle admission",
                client.apply_session_options(session_id(), LaunchChoices::default()),
            )
            .await
            .unwrap_err() else {
                panic!("not local admission")
            };
            refusal(error, Method::ApplySessionOptions, negotiated);
            refusal(
                phase(
                    "nested admission",
                    client.call_on(NodeId::random(), ListLaunchProfiles {}),
                )
                .await
                .unwrap_err(),
                Method::Forward,
                negotiated,
            );
            drop(client);
            assert!(
                phase("peer sees client closure", peer)
                    .await
                    .unwrap()
                    .is_none()
            );
        }),
    )
    .await
    .unwrap();
}
