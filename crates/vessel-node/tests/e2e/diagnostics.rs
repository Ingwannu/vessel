//! Assigned diagnostics over owner, paired-device, agent and fleet transports.
//! The fixtures use fake agents and temporary stores, never physical machines.
use crate::agents::{as_agent, credential, structured};
use crate::fleet_control::device;
use crate::secure::{Relay, relayed_with};
use crate::support::{Fixture, step, structured_spec, within};
use vessel_client::{CallError, DiagnosticError, NodeClient};
use vessel_proto::diagnostics::{DiagnosticReport, DiagnosticSection, GetDiagnostics};
use vessel_proto::{ErrorCode, GetNode, NodeId, Origin, OriginBy, Request as _, Scope};

async fn report(client: &NodeClient, id: NodeId, count: u64) -> DiagnosticReport {
    let report = step("assigned selected-node diagnostics", client.diagnostics())
        .await
        .unwrap();
    assert_eq!(report.fields().node.id, id);
    assert_eq!(client.node().id, id);
    let DiagnosticSection::Available { value, .. } = &report.fields().store else {
        panic!("fixture store is readable")
    };
    assert_eq!(value.sessions.total.get(), count);
    let DiagnosticSection::Available { value, .. } = &report.fields().sessions else {
        panic!("fixture runtime is readable")
    };
    assert_eq!(value.live.get(), count);
    report
}
fn forbidden(error: DiagnosticError) {
    assert!(
        matches!(error, DiagnosticError::Call(CallError::Remote(error)) if error.code == ErrorCode::Forbidden)
    );
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn diagnostics_owner_and_two_paired_readers_follow_two_real_nodes_and_revocation() {
    within(async {
        let relay = step("diagnostic relay starts", Relay::start()).await;
        let first = Fixture::new();
        let second = Fixture::new();
        let a = step(
            "first diagnostic node",
            relayed_with(&first, &[&relay.url], |_| {}),
        )
        .await;
        let b = step(
            "second diagnostic node",
            relayed_with(&second, &[&relay.url], |_| {}),
        )
        .await;
        let owner_a = step("first owner admitted", first.connect(&a)).await;
        let owner_b = step("second owner admitted", second.connect(&b)).await;
        assert_ne!(a.info().id, b.info().id);
        let session = step(
            "one real fake session on first node",
            owner_a.create_session(structured_spec(&first, None)),
        )
        .await
        .unwrap();
        assert!(session.status.is_live());
        let reader_a = step(
            "pair first Read device",
            device(&owner_a, &relay, vec![Scope::Read]),
        )
        .await;
        let reader_b = step(
            "pair second Read device",
            device(&owner_b, &relay, vec![Scope::Read]),
        )
        .await;
        for (reader, owner, id, count) in [
            (&reader_a, &owner_a, a.info().id, 1),
            (&reader_b, &owner_b, b.info().id, 0),
        ] {
            let authority = step(
                "paired caller reads its admitted scope",
                reader.device_self(),
            )
            .await
            .unwrap();
            assert_eq!(authority.scopes, [Scope::Read]);
            let paired_id = authority
                .device
                .expect("a genuine paired caller has its own id");
            let _owner_report = report(owner, id, count).await;
            let report = report(reader, id, count).await;
            let json = serde_json::to_string(&report).unwrap();
            for private in [
                first.data_path(),
                second.data_path(),
                first.cwd().as_path(),
                second.cwd().as_path(),
            ] {
                assert!(!json.contains(private.to_str().unwrap()));
            }
            step(
                "canonical owner revokes paired reader",
                owner.revoke_device(paired_id),
            )
            .await
            .unwrap();
            step("revoked diagnostic connection closes", reader.closed()).await;
            assert!(
                step(
                    "revoked reader cannot obtain another report",
                    reader.diagnostics()
                )
                .await
                .is_err()
            );
        }
        step("stop first diagnostic node", a.stop()).await.unwrap();
        step("stop second diagnostic node", b.stop()).await.unwrap();
        step("stop diagnostic relay", relay.stop()).await;
    })
    .await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn diagnostics_wire_authority_keeps_read_agents_and_refuses_no_read_devices_and_agents() {
    within(async {
        let relay = step("agent diagnostic relay", Relay::start()).await;
        let fixture = Fixture::new();
        let node = step(
            "agent diagnostic node",
            relayed_with(&fixture, &[&relay.url], |config| {
                config.vessel_mcp = Some(crate::agents::vessel_mcp(&fixture));
            }),
        )
        .await;
        let owner = step("diagnostic owner admitted", fixture.connect(&node)).await;
        let creator = step(
            "pair Operate-only diagnostic caller",
            device(&owner, &relay, vec![Scope::Operate]),
        )
        .await;
        let own = step("read actual Operate-only grant", creator.device_self())
            .await
            .unwrap();
        assert_eq!(own.scopes, [Scope::Operate]);
        forbidden(
            step("no-Read device diagnostic refusal", creator.diagnostics())
                .await
                .unwrap_err(),
        );
        let mut allowed = step(
            "create Read-agent fake session",
            structured(&owner, &fixture),
        )
        .await;
        let (_, socket, auth) = step(
            "Read-agent actual credential",
            credential(&owner, &mut allowed),
        )
        .await;
        let agent = step(
            "Read-agent authenticated local socket",
            as_agent(&socket, &auth),
        )
        .await;
        let report = step("effective-Read agent assigned report", agent.diagnostics())
            .await
            .unwrap();
        assert_eq!(report.fields().node.id, node.info().id);
        let mut restricted = step(
            "create scope-clipped fake session",
            structured(&creator, &fixture),
        )
        .await;
        let (_, socket, auth) = step(
            "clipped agent actual credential",
            credential(&owner, &mut restricted),
        )
        .await;
        let agent = step(
            "clipped agent authenticated local socket",
            as_agent(&socket, &auth),
        )
        .await;
        assert_eq!(
            step("clipped agent actual scopes", agent.device_self())
                .await
                .unwrap()
                .scopes,
            [Scope::Operate]
        );
        forbidden(
            step("no-Read agent diagnostic refusal", agent.diagnostics())
                .await
                .unwrap_err(),
        );
        step("stop scoped diagnostic node", node.stop())
            .await
            .unwrap();
        step("stop scoped diagnostic relay", relay.stop()).await;
    })
    .await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn diagnostics_supported_fleet_routes_refuse_forwarding_instead_of_substituting_a_home_report()
 {
    within(async {
        let relay = step("fleet diagnostic relay", Relay::start()).await;
        let sources = tempfile::tempdir().unwrap();
        let home = step("diagnostic fleet home", crate::across::home(&relay, sources.path())).await;
        let member = step("diagnostic fleet member", crate::across::member_granting(
            &relay, &home, "diagnostic-member", Scope::ALL.to_vec(), Scope::ALL.to_vec(),
        )).await;
        let info = step("same fleet route admits existing Read call", home.owner.call_on(member.id(), GetNode)).await.unwrap();
        assert_eq!(info.id, member.id());
        let origin = Origin { node: home.id(), by: OriginBy::Owner, scopes: Scope::ALL.to_vec() };
        let calls = [
            vessel_proto::Forward { node: member.id(), call: Box::new(GetDiagnostics {}.into_call()) }.into_call(),
            vessel_proto::RelayCall { node: member.id(), origin: origin.clone(), call: Box::new(GetDiagnostics {}.into_call()) }.into_call(),
            vessel_proto::Deliver { origin, call: Box::new(GetDiagnostics {}.into_call()) }.into_call(),
        ];
        for call in calls {
            let error = step("supported envelope refuses nonforwardable diagnostics", home.owner.call_any(vessel_rpc::UnaryCall::try_from(call).unwrap())).await.unwrap_err();
            assert!(matches!(error, CallError::Remote(error) if error.code == ErrorCode::InvalidRequest && error.message.contains("node.diagnostics")));
        }
        // Directly selected connections still answer after all three refusals.
        let _home = report(&home.owner, home.id(), 0).await;
        let _member = report(&member.owner, member.id(), 0).await;
        step("stop diagnostic fleet member", member.node.stop()).await.unwrap();
        step("stop diagnostic fleet home", home.node.stop()).await.unwrap();
        step("stop diagnostic fleet relay", relay.stop()).await;
    }).await;
}
