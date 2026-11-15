//! Current authority over authenticated connections; self-inspection must
//! work even when no Read scope was granted, without exposing other devices.
use crate::agents::{as_agent, code, credential, structured};
use crate::secure::{Relay, device_via, dialer, relayed_with};
use crate::support::{Fixture, client, step, within};
use vessel_client::Connector as _;
use vessel_client::secure::{RouteTimeouts, pair_over};
use vessel_proto::{DeviceSelf, ErrorCode, GetDeviceSelf, Scope};

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn every_authenticated_device_can_read_only_its_own_current_authority() {
    within(async {
        let relay = Relay::start().await;
        let fixture = Fixture::new();
        let node = relayed_with(&fixture, &[&relay.url], |config| {
            config.vessel_mcp = Some(crate::agents::vessel_mcp(&fixture));
        })
        .await;
        let owner = fixture.connect(&node).await;
        assert_eq!(
            owner.device_self().await.unwrap(),
            vessel_testkit::device_fixtures::owner_self()
        );
        for grant in vessel_testkit::device_fixtures::SELF_GRANTS {
            let scopes = grant.to_vec();
            let offer = owner
                .invite_with_scopes(None, 600, scopes.clone())
                .await
                .unwrap()
                .link
                .decode()
                .unwrap();
            let (paired, _) = pair_over(&dialer(), &offer, "self reader", RouteTimeouts::default())
                .await
                .unwrap();
            let device = device_via(dialer(), &paired, &[], &[&relay.url])
                .connect(client())
                .await
                .unwrap();
            let own = device.device_self().await.unwrap();
            assert_eq!(
                own,
                DeviceSelf {
                    device: Some(paired.credential.device),
                    scopes
                }
            );
            assert_eq!(code(device.devices().await), ErrorCode::Forbidden);
            if !grant.contains(&Scope::Read) {
                assert_eq!(code(device.node_info().await), ErrorCode::Forbidden);
            }
            if !grant.contains(&Scope::Operate) {
                assert_eq!(
                    code(device.kill(vessel_proto::SessionId::random()).await),
                    ErrorCode::Forbidden
                );
            }
            owner.revoke_device(own.device.unwrap()).await.unwrap();
            step("revoked self-reader closes", device.closed()).await;
            assert!(device.device_self().await.is_err());
        }
        let mut session = structured(&owner, &fixture).await;
        let (_, socket, auth) = credential(&owner, &mut session).await;
        let agent = as_agent(&socket, &auth).await;
        assert_eq!(
            agent.device_self().await.unwrap(),
            DeviceSelf {
                device: None,
                scopes: vec![Scope::Read, Scope::Operate],
            }
        );
        owner.kill(session.id).await.unwrap();
        node.stop().await.unwrap();
        relay.stop().await;
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn delegated_self_reports_intersection_without_the_transport_device_id() {
    within(async {
        let relay = Relay::start().await;
        let sources = tempfile::tempdir().unwrap();
        let home = crate::across::home(&relay, sources.path()).await;
        let member = crate::across::member_granting(
            &relay,
            &home,
            "self-member",
            Scope::ALL.to_vec(),
            vec![Scope::Read],
        )
        .await;
        assert_eq!(
            home.owner
                .call_on(member.id(), GetDeviceSelf {})
                .await
                .unwrap(),
            DeviceSelf {
                device: None,
                scopes: vec![Scope::Read],
            }
        );
        member.node.stop().await.unwrap();
        home.node.stop().await.unwrap();
        relay.stop().await;
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_agent_without_read_reports_only_its_clipped_scope() {
    within(async {
        let relay = Relay::start().await;
        let fixture = Fixture::new();
        let node = relayed_with(&fixture, &[&relay.url], |config| {
            config.vessel_mcp = Some(crate::agents::vessel_mcp(&fixture));
        })
        .await;
        let owner = fixture.connect(&node).await;
        let creator = crate::fleet_control::device(&owner, &relay, vec![Scope::Operate]).await;
        let session = creator
            .create_session(crate::support::structured_spec(&fixture, None))
            .await
            .unwrap();
        let mut running =
            crate::agents::following(&owner, session.id, vessel_proto::EventSeq::ZERO).await;
        let (_, socket, auth) = credential(&owner, &mut running).await;
        let agent = as_agent(&socket, &auth).await;
        assert_eq!(
            agent.device_self().await.unwrap(),
            DeviceSelf {
                device: None,
                scopes: vec![Scope::Operate]
            }
        );
        assert_eq!(code(agent.node_info().await), ErrorCode::Forbidden);
        assert_eq!(code(agent.devices().await), ErrorCode::Forbidden);
        owner.kill(session.id).await.unwrap();
        node.stop().await.unwrap();
        relay.stop().await;
    })
    .await;
}
