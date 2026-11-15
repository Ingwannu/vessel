//! Calls that need more than the stream tests show: `node.stop`.

use futures_util::{SinkExt as _, StreamExt as _};
use vessel_proto::{ProtocolVersion, ServerFrame, Welcome};

use super::*;

#[tokio::test]
async fn stop_node_reaches_the_node() {
    let node = Arc::new(FakeNode::new(0));
    let client = connect(Arc::clone(&node)).await;
    client.stop_node(StopWhen::WhenIdle).await.unwrap();
    assert_eq!(*node.stopped.lock().unwrap(), Some(StopWhen::WhenIdle));
    client.stop_node(StopWhen::Now).await.unwrap();
    assert_eq!(*node.stopped.lock().unwrap(), Some(StopWhen::Now));
}

/// A node that speaks only protocol 2 is never sent `node.stop`.
#[tokio::test]
async fn stop_node_is_refused_locally_on_protocol_2() {
    let (client_end, mut node_end) = vessel_rpc::memory::pair(NonZeroUsize::new(4).unwrap());
    let old_node = tokio::spawn(async move {
        let _hello = node_end.next().await.unwrap().unwrap();
        let welcome = ServerFrame::Welcome(Welcome {
            protocol: ProtocolVersion::new(2),
            node: node(),
        });
        node_end
            .send(serde_json::to_string(&welcome).unwrap())
            .await
            .unwrap();
        // Anything sent after the handshake would arrive here.
        node_end.next().await
    });
    let client = NodeClient::over(client_end, client_info(), &ClientConfig::default())
        .await
        .unwrap();
    assert_eq!(client.protocol(), ProtocolVersion::new(2));
    let error = client.stop_node(StopWhen::Now).await.unwrap_err();
    assert_eq!(error.code(), Some(ErrorCode::Unsupported));
    drop(client);
    assert!(old_node.await.unwrap().is_none(), "nothing was sent");
}

/// Protocol 12 cannot receive a preview request; it never goes on the wire.
#[tokio::test]
async fn preview_is_refused_locally_before_protocol_13() {
    let (client_end, mut node_end) = vessel_rpc::memory::pair(NonZeroUsize::new(4).unwrap());
    let old_node = tokio::spawn(async move {
        let _hello = node_end.next().await.unwrap().unwrap();
        let welcome = ServerFrame::Welcome(Welcome {
            protocol: ProtocolVersion::new(12),
            node: node(),
        });
        node_end
            .send(serde_json::to_string(&welcome).unwrap())
            .await
            .unwrap();
        node_end.next().await
    });
    let client = NodeClient::over(client_end, client_info(), &ClientConfig::default())
        .await
        .unwrap();
    let error = client
        .preview_schedule(vessel_proto::automation::PreviewSchedule {
            cadence: vessel_proto::automation::Cadence::Every {
                seconds: vessel_proto::automation::EverySeconds::try_from(60).unwrap(),
                anchor: None,
            },
            after: None,
            count: vessel_proto::automation::PreviewCount::default(),
        })
        .await
        .unwrap_err();
    assert_eq!(error.code(), Some(ErrorCode::Unsupported));
    drop(client);
    assert!(old_node.await.unwrap().is_none(), "no request was sent");
}

#[tokio::test]
async fn fleet_controls_are_refused_before_their_declared_protocol() {
    use vessel_proto::Request as _;
    let since = vessel_proto::GetFleetStatus::METHOD.since().unwrap();
    let previous = ProtocolVersion::new(since.get().checked_sub(1).unwrap());
    let (client_end, mut node_end) = vessel_rpc::memory::pair(NonZeroUsize::new(4).unwrap());
    let old_node = tokio::spawn(async move {
        let _hello = node_end.next().await.unwrap().unwrap();
        let welcome = ServerFrame::Welcome(Welcome {
            protocol: previous,
            node: node(),
        });
        node_end
            .send(serde_json::to_string(&welcome).unwrap())
            .await
            .unwrap();
        node_end.next().await
    });
    let client = NodeClient::over(client_end, client_info(), &ClientConfig::default())
        .await
        .unwrap();
    for result in [client.leave_fleet().await, client.sync_fleet().await] {
        assert_eq!(result.unwrap_err().code(), Some(ErrorCode::Unsupported));
    }
    assert_eq!(
        client.fleet_status().await.unwrap_err().code(),
        Some(ErrorCode::Unsupported)
    );
    drop(client);
    assert!(old_node.await.unwrap().is_none(), "no new control was sent");
}

/// Protocol 15 has no `fleet.published`: the client refuses it locally.
#[tokio::test]
async fn published_is_refused_locally_before_protocol_16() {
    let (client_end, mut node_end) = vessel_rpc::memory::pair(NonZeroUsize::new(4).unwrap());
    let old_node = tokio::spawn(async move {
        let _hello = node_end.next().await.unwrap().unwrap();
        let welcome = ServerFrame::Welcome(Welcome {
            protocol: ProtocolVersion::new(15),
            node: node(),
        });
        node_end
            .send(serde_json::to_string(&welcome).unwrap())
            .await
            .unwrap();
        node_end.next().await
    });
    let client = NodeClient::over(client_end, client_info(), &ClientConfig::default())
        .await
        .unwrap();
    assert_eq!(
        client.fleet_published().await.unwrap_err().code(),
        Some(ErrorCode::Unsupported)
    );
    drop(client);
    assert!(old_node.await.unwrap().is_none(), "nothing was sent");
}

/// Protocol 16 has no `agent.history` (track A2 wrote it at 14; it is 17
/// on the node line): the client refuses it locally.
#[tokio::test]
async fn agent_history_is_refused_locally_before_protocol_17() {
    let (client_end, mut node_end) = vessel_rpc::memory::pair(NonZeroUsize::new(4).unwrap());
    let old_node = tokio::spawn(async move {
        let _hello = node_end.next().await.unwrap().unwrap();
        let welcome = ServerFrame::Welcome(Welcome {
            protocol: ProtocolVersion::new(16),
            node: node(),
        });
        node_end
            .send(serde_json::to_string(&welcome).unwrap())
            .await
            .unwrap();
        node_end.next().await
    });
    let client = NodeClient::over(client_end, client_info(), &ClientConfig::default())
        .await
        .unwrap();
    let asked = vessel_proto::history::GetAgentHistory {
        harness: "claude-code".parse().unwrap(),
        profile: None,
        cwd: None,
        cursor: None,
    };
    assert_eq!(
        client.call(asked).await.unwrap_err().code(),
        Some(ErrorCode::Unsupported)
    );
    drop(client);
    assert!(old_node.await.unwrap().is_none(), "nothing was sent");
}

#[tokio::test]
async fn self_authority_is_refused_locally_on_an_older_direct_or_routed_connection() {
    use vessel_proto::{GetDeviceSelf, Request as _};
    let previous = ProtocolVersion::new(
        GetDeviceSelf::METHOD
            .since()
            .unwrap()
            .get()
            .checked_sub(1)
            .unwrap(),
    );
    let (client_end, mut node_end) = vessel_rpc::memory::pair(NonZeroUsize::new(4).unwrap());
    let old_node = tokio::spawn(async move {
        let _hello = node_end.next().await.unwrap().unwrap();
        node_end
            .send(
                serde_json::to_string(&ServerFrame::Welcome(Welcome {
                    protocol: previous,
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
    assert!(matches!(
        client.device_self().await,
        Err(vessel_rpc::CallError::NotInProtocol { .. })
    ));
    assert!(matches!(
        client.call_on(NodeId::random(), GetDeviceSelf {}).await,
        Err(vessel_rpc::CallError::NotInProtocol { .. })
    ));
    drop(client);
    assert!(
        old_node.await.unwrap().is_none(),
        "no query was sent and no authority was guessed"
    );
}

/// The old hop itself cannot decode a new inner call, even for its own id.
#[tokio::test]
async fn usage_direct_and_every_envelope_are_refused_locally_on_an_old_hop() {
    use vessel_proto::usage::{QueryUsage, UsagePeriod, UsageScope};
    use vessel_proto::{Deliver, Forward, Origin, OriginBy, RelayCall, Request as _};
    use vessel_rpc::{CallError, UnaryCall};
    let needed = QueryUsage::METHOD.since().unwrap();
    let previous = ProtocolVersion::new(needed.get().checked_sub(1).unwrap());
    let hop = node();
    let target = hop.id;
    let (client_end, mut node_end) = vessel_rpc::memory::pair(NonZeroUsize::new(4).unwrap());
    let old_node = tokio::spawn(async move {
        let _hello = node_end.next().await.unwrap().unwrap();
        node_end
            .send(
                serde_json::to_string(&ServerFrame::Welcome(Welcome {
                    protocol: previous,
                    node: hop,
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
    let query = QueryUsage {
        scope: UsageScope::Machine,
        period: UsagePeriod::Day,
        after: None,
    };
    let direct = client.usage(query.clone()).await.unwrap_err();
    assert!(
        matches!(direct, CallError::NotInProtocol { method: vessel_proto::Method::QueryUsage, needs, negotiated } if needs == needed && negotiated == previous)
    );
    let forwarded = client.call_on(target, query.clone()).await.unwrap_err();
    assert!(
        matches!(forwarded, CallError::NotInProtocol { method: vessel_proto::Method::Forward, needs, negotiated } if needs == needed && negotiated == previous)
    );
    let origin = Origin {
        node: target,
        by: OriginBy::Owner,
        scopes: vec![],
    };
    for call in [
        Forward {
            node: target,
            call: Box::new(query.clone().into_call()),
        }
        .into_call(),
        RelayCall {
            node: target,
            call: Box::new(query.clone().into_call()),
            origin: origin.clone(),
        }
        .into_call(),
        Deliver {
            call: Box::new(query.into_call()),
            origin,
        }
        .into_call(),
    ] {
        let method = call.method();
        let error = client
            .call_any(UnaryCall::try_from(call).unwrap())
            .await
            .unwrap_err();
        assert!(
            matches!(error, CallError::NotInProtocol { method: actual, needs, negotiated } if actual == method && needs == needed && negotiated == previous)
        );
    }
    drop(client);
    assert!(
        old_node.await.unwrap().is_none(),
        "no direct or wrapped usage frame was sent"
    );
}

#[tokio::test]
async fn limits_policy_and_reads_send_no_frames_to_an_old_hop() {
    use vessel_proto::{Deliver, Forward, Origin, OriginBy, RelayCall, Request as _, limits::*};
    use vessel_rpc::{CallError, UnaryCall};
    let needed = GetUsageLimits::METHOD.since().unwrap();
    assert_eq!(Some(needed), SetUsageLookup::METHOD.since());
    let previous = ProtocolVersion::new(needed.get().checked_sub(1).unwrap());
    let hop = node();
    let target = hop.id;
    let (client_end, mut node_end) = vessel_rpc::memory::pair(NonZeroUsize::new(4).unwrap());
    let old_node = tokio::spawn(async move {
        let _hello = node_end.next().await.unwrap().unwrap();
        node_end
            .send(
                serde_json::to_string(&ServerFrame::Welcome(Welcome {
                    protocol: previous,
                    node: hop,
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
    let query = GetUsageLimits {
        scope: LimitsScope::Account {
            account: LimitAccount {
                harness: HarnessKind::Codex.into(),
                profile: None,
            },
        },
        refresh: LimitRefresh::Refresh,
    };
    let policy = SetUsageLookup {
        policy: ExternalLookup::On,
    };
    // Cover both typed conveniences and the typed call_on-self seam.
    for error in [
        client.usage_limits(query.clone()).await.unwrap_err(),
        client.usage_lookup(policy).await.unwrap_err(),
        client.call_on(target, query.clone()).await.unwrap_err(),
        client.call_on(target, policy).await.unwrap_err(),
    ] {
        assert!(
            matches!(error,CallError::NotInProtocol {needs,negotiated,..} if needs==needed && negotiated==previous)
        );
    }
    let origin = Origin {
        node: target,
        by: OriginBy::Owner,
        scopes: vec![],
    };
    for inner in [query.into_call(), policy.into_call()] {
        for call in [
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
            let method = call.method();
            let error = client
                .call_any(UnaryCall::try_from(call).unwrap())
                .await
                .unwrap_err();
            assert!(
                matches!(error,CallError::NotInProtocol {method:actual,needs,negotiated} if actual==method && needs==needed && negotiated==previous)
            );
        }
    }
    drop(client);
    assert!(
        old_node.await.unwrap().is_none(),
        "no direct or enveloped limit frame was sent"
    );
}

mod inbox;

/// Quality B pending calls are refused before a frame is sent, even to a
/// current peer. This preserves older clients while the package is integrated.
#[tokio::test]
async fn pending_git_comparison_never_reaches_the_peer() {
    use vessel_proto::workspace::{CompareGit, GitComparison, GitWhitespace, WorkspaceRoot};
    let (client_end, mut node_end) = vessel_rpc::memory::pair(NonZeroUsize::new(4).unwrap());
    let peer = tokio::spawn(async move {
        let _hello = node_end.next().await.unwrap().unwrap();
        node_end
            .send(
                serde_json::to_string(&ServerFrame::Welcome(Welcome {
                    protocol: ProtocolRange::CURRENT.max,
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
    let error = client
        .call(CompareGit {
            root: WorkspaceRoot::Project {
                project: vessel_proto::ProjectId::random(),
            },
            comparison: GitComparison::Committed { base: None },
            whitespace: GitWhitespace::Show,
            reason: vessel_proto::workspace::GitReadReason::View,
        })
        .await
        .unwrap_err();
    assert_eq!(error.code(), Some(ErrorCode::Unsupported));
    assert!(matches!(
        error,
        crate::CallError::UnassignedProtocol {
            method: vessel_proto::Method::GitCompare
        }
    ));
    let history = client
        .call(vessel_proto::workspace::ReadGitHistory {
            root: WorkspaceRoot::Project {
                project: vessel_proto::ProjectId::random(),
            },
        })
        .await
        .unwrap_err();
    assert!(matches!(
        history,
        crate::CallError::UnassignedProtocol {
            method: vessel_proto::Method::GitHistory
        }
    ));
    drop(client);
    assert!(
        peer.await.unwrap().is_none(),
        "no pending request frame was sent"
    );
}
mod file_panels;
