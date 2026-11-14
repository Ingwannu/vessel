//! Host settings keep their non-forwardable authority; paired relay links work.
use super::{fixture, policy};
use crate::agents::code;
use crate::support::{client, within};
use futures_util::{SinkExt as _, StreamExt as _};
use tokio_tungstenite::tungstenite::{Message, client::IntoClientRequest as _};
use vessel_client::secure::{RouteTimeouts, pair_over};
use vessel_client::{Connector as _, NodeClient};
use vessel_proto::restart::{RestartRecovery, SetRestartRecovery};
use vessel_proto::settings::{GetNodeSettings, SetNodeSettings};
use vessel_proto::{
    Call, ClientFrame, ErrorCode, Hello, Outcome, ProtocolRange, ProtocolVersion, Reply,
    Request as _, RequestId, Scope, ServerFrame,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn old_direct_hops_refuse_the_explicit_read_patch_and_nested_fields() {
    within(async {
        let f = fixture();
        let node = f.start().await;
        let owner = f.connect(&node).await;
        policy(&owner, true, Some("Continue verbatim.\n")).await;
        let mut request = format!("ws://{}/rpc", node.address().unwrap())
            .into_client_request()
            .unwrap();
        let mut bearer = format!("Bearer {}", f.token().expose())
            .parse::<tokio_tungstenite::tungstenite::http::HeaderValue>()
            .unwrap();
        bearer.set_sensitive(true);
        let _old = request.headers_mut().insert("authorization", bearer);
        let (mut socket, _) = tokio_tungstenite::connect_async(request).await.unwrap();
        let version = ProtocolVersion::new(
            vessel_proto::restart::RESTART_RECOVERY_PROTOCOL
                .get()
                .checked_sub(1)
                .unwrap(),
        );
        socket
            .send(Message::text(
                serde_json::to_string(&ClientFrame::Hello(Hello {
                    protocol: ProtocolRange {
                        min: version,
                        max: version,
                    },
                    client: client(),
                }))
                .unwrap(),
            ))
            .await
            .unwrap();
        let received = socket.next().await.unwrap().unwrap().into_text().unwrap();
        let ServerFrame::Welcome(welcome) = serde_json::from_str(&received).unwrap() else {
            panic!("welcome")
        };
        assert_eq!(welcome.protocol, version);
        let read = GetNodeSettings {
            include_restart_recovery: true,
        }
        .into_call();
        let empty = SetNodeSettings {
            restart_recovery: Some(SetRestartRecovery::default()),
            ..SetNodeSettings::default()
        }
        .into_call();
        let nested = Call::Forward(vessel_proto::Forward {
            node: node.info().id,
            call: Box::new(read.clone()),
        });
        for (id, call) in [
            (1, GetNodeSettings::default().into_call()),
            (2, read),
            (3, empty),
            (4, nested),
        ] {
            socket
                .send(Message::text(
                    serde_json::to_string(&ClientFrame::Request {
                        id: RequestId::new(id),
                        call,
                    })
                    .unwrap(),
                ))
                .await
                .unwrap();
            let received = socket.next().await.unwrap().unwrap().into_text().unwrap();
            let ServerFrame::Response { outcome, .. } = serde_json::from_str(&received).unwrap()
            else {
                panic!("response")
            };
            if id == 1 {
                let Outcome::Ok(Reply::GetNodeSettings(shown)) = outcome else {
                    panic!("settings")
                };
                assert_eq!(shown.restart_recovery, RestartRecovery::default());
            } else {
                let Outcome::Err(error) = outcome else {
                    panic!("old hop must refuse")
                };
                assert_eq!(error.code, ErrorCode::Unsupported);
            }
        }
        socket.close(None).await.unwrap();
        assert!(
            owner
                .call(GetNodeSettings {
                    include_restart_recovery: true
                })
                .await
                .unwrap()
                .restart_recovery
                .enabled
        );
        node.stop().await.unwrap();
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn real_fleet_member_envelopes_refuse_host_settings_even_with_admin() {
    within(async {
        let relay = crate::secure::Relay::start().await;
        let sources = tempfile::tempdir().unwrap();
        let home = crate::across::home(&relay, sources.path()).await;
        let member = crate::across::member_granting(
            &relay,
            &home,
            "settings-member",
            vec![Scope::Sync, Scope::Relay, Scope::Read, Scope::Admin],
            vec![Scope::Read, Scope::Admin],
        )
        .await;
        policy(&home.owner, true, Some("Continue verbatim.\n")).await;
        let read = GetNodeSettings {
            include_restart_recovery: true,
        };
        let before = home.owner.call(read).await.unwrap();
        assert_eq!(
            code(member.owner.call_on(home.id(), read).await),
            ErrorCode::InvalidRequest
        );
        assert_eq!(
            code(
                member
                    .owner
                    .call_on(
                        home.id(),
                        SetNodeSettings {
                            restart_recovery: Some(SetRestartRecovery {
                                enabled: Some(false),
                                prompt: None
                            }),
                            ..SetNodeSettings::default()
                        }
                    )
                    .await
            ),
            ErrorCode::InvalidRequest
        );
        assert_eq!(home.owner.call(read).await.unwrap(), before);
        member.node.stop().await.unwrap();
        home.node.stop().await.unwrap();
        relay.stop().await;
    })
    .await;
}

async fn relay_device(
    home: &crate::across::Member,
    relay: &crate::secure::Relay,
    scopes: Vec<Scope>,
) -> NodeClient {
    let mut offer = home
        .owner
        .invite_with_scopes(None, 600, scopes)
        .await
        .unwrap()
        .link
        .decode()
        .unwrap();
    offer.direct.clear();
    assert!(!offer.relays.is_empty());
    let (paired, _) = pair_over(
        &crate::secure::dialer(),
        &offer,
        "settings device",
        RouteTimeouts::default(),
    )
    .await
    .unwrap();
    crate::secure::device_via(crate::secure::dialer(), &paired, &[], &[&relay.url])
        .connect(client())
        .await
        .unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_hosts_paired_relay_devices_keep_recovery_fields_and_enforce_read_admin() {
    within(async {
        let relay = crate::secure::Relay::start().await;
        let sources = tempfile::tempdir().unwrap();
        let home = crate::across::home(&relay, sources.path()).await;
        policy(&home.owner, true, Some("Continue verbatim.\n")).await;
        let admin = relay_device(&home, &relay, vec![Scope::Read, Scope::Admin]).await;
        let read = GetNodeSettings {
            include_restart_recovery: true,
        };
        let shown = admin.call(read).await.unwrap();
        assert!(shown.restart_recovery.enabled);
        assert_eq!(
            shown.restart_recovery.prompt.as_str(),
            "Continue verbatim.\n"
        );
        let patch = SetNodeSettings {
            restart_recovery: Some(SetRestartRecovery {
                enabled: Some(false),
                prompt: None,
            }),
            ..SetNodeSettings::default()
        };
        let changed = admin.call(patch.clone()).await.unwrap();
        assert!(!changed.restart_recovery.enabled);
        assert_eq!(
            changed.restart_recovery.prompt.as_str(),
            "Continue verbatim.\n"
        );
        let reader = relay_device(&home, &relay, vec![Scope::Read]).await;
        assert_eq!(reader.call(read).await.unwrap(), changed);
        assert_eq!(code(reader.call(patch).await), ErrorCode::Forbidden);
        assert_eq!(home.owner.call(read).await.unwrap(), changed);
        drop(reader);
        drop(admin);
        home.node.stop().await.unwrap();
        relay.stop().await;
    })
    .await;
}
