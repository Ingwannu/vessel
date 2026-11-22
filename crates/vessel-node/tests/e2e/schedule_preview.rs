//! Read-only previews through the real node and device authentication.
use crate::agents::{as_agent, code, credential, structured, vessel_mcp};
use crate::secure::{Relay, device_via, dialer, relayed_with};
use crate::support::{Fixture, client, structured_spec, within};
use chrono::{DateTime, Utc};
use vessel_client::Connector as _;
use vessel_client::secure::{RouteTimeouts, pair_over};
use vessel_proto::automation::{
    AfterRun, Cadence, CatchUp, EverySeconds, PreviewCount, PreviewSchedule, ScheduleDefinition,
    SchedulePreviewError, ScheduleState, ScheduleTarget,
};
use vessel_proto::{ErrorCode, ErrorDetail, Scope};

fn request(after: Option<DateTime<Utc>>) -> PreviewSchedule {
    PreviewSchedule {
        cadence: Cadence::Every {
            seconds: EverySeconds::try_from(3600).unwrap(),
            anchor: None,
        },
        after,
        count: PreviewCount::default(),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_preview_writes_nothing_and_saved_cadence_keeps_its_anchor() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        for &(expr, zone, after, next) in vessel_testkit::schedule_fixtures::CRON_CASES {
            let result = client
                .preview_schedule(PreviewSchedule {
                    cadence: Cadence::Cron {
                        expr: expr.parse().unwrap(),
                        zone: zone.parse().unwrap(),
                    },
                    after: Some(after.parse().unwrap()),
                    count: PreviewCount::try_from(1).unwrap(),
                })
                .await
                .unwrap();
            assert_eq!(result.runs, [next.parse::<DateTime<Utc>>().unwrap()]);
        }
        let preview = client.preview_schedule(request(None)).await.unwrap();
        assert_eq!(preview.runs.len(), 5);
        assert!(client.schedules().await.unwrap().is_empty());
        assert!(client.sessions().await.unwrap().is_empty());
        let saved = client
            .create_schedule(
                ScheduleDefinition {
                    name: None,
                    cadence: preview.cadence.clone(),
                    target: ScheduleTarget::NewSession {
                        isolation: None,
                        spec: Box::new(structured_spec(&fixture, None)),
                        after: AfterRun::Archive,
                    },
                    prompt: "plain".parse().unwrap(),
                    catch_up: CatchUp::Once,
                    max_runs: None,
                    ends_at: None,
                },
                false,
            )
            .await
            .unwrap();
        assert_eq!(saved.definition.cadence, preview.cadence);
        assert_eq!(
            saved.state,
            ScheduleState::Active {
                next_due: *preview.runs.first().unwrap()
            }
        );
        let mut invalid = request(None);
        invalid.cadence = Cadence::Cron {
            expr: "0 9 * * *".parse().unwrap(),
            zone: "Not/AZone".parse().unwrap(),
        };
        let Err(vessel_rpc::CallError::Remote(error)) = client.preview_schedule(invalid).await
        else {
            panic!("expected typed unknown-zone error");
        };
        assert_eq!(error.code, ErrorCode::InvalidRequest);
        assert_eq!(
            error.detail,
            Some(ErrorDetail::SchedulePreview {
                reason: SchedulePreviewError::UnknownZone
            })
        );
        for (cadence, after, expected) in [
            (
                Cadence::Unknown,
                "2026-01-01T00:00:00Z",
                SchedulePreviewError::UnknownCadence,
            ),
            (
                Cadence::Cron {
                    expr: "0 0 30 2 *".parse().unwrap(),
                    zone: "UTC".parse().unwrap(),
                },
                "2026-01-01T00:00:00Z",
                SchedulePreviewError::NoFutureRun,
            ),
            (
                preview.cadence.clone(),
                "+010000-01-01T00:00:00Z",
                SchedulePreviewError::TimeOutOfRange,
            ),
        ] {
            let mut invalid = request(Some(after.parse().unwrap()));
            invalid.cadence = cadence;
            let Err(vessel_rpc::CallError::Remote(error)) = client.preview_schedule(invalid).await
            else {
                panic!("expected preview refusal");
            };
            assert_eq!(error.code, ErrorCode::InvalidRequest);
            assert_eq!(
                error.detail,
                Some(ErrorDetail::SchedulePreview { reason: expected })
            );
        }
        node.stop().await.unwrap();
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_read_only_device_and_agent_can_preview_but_a_device_without_read_cannot() {
    within(async {
        let relay = Relay::start().await;
        let fixture = Fixture::new();
        let node = relayed_with(&fixture, &[&relay.url], |config| {
            config.vessel_mcp = Some(vessel_mcp(&fixture));
        })
        .await;
        let owner = fixture.connect(&node).await;
        for (scopes, allowed) in [(vec![Scope::Read], true), (vec![Scope::Sync], false)] {
            let offer = owner
                .invite_with_scopes(None, 600, scopes)
                .await
                .unwrap()
                .link
                .decode()
                .unwrap();
            let (paired, _) = pair_over(
                &dialer(),
                &offer,
                "preview reader",
                RouteTimeouts::default(),
            )
            .await
            .unwrap();
            let device = device_via(dialer(), &paired, &[], &[&relay.url])
                .connect(client())
                .await
                .unwrap();
            let result = device.preview_schedule(request(None)).await;
            if allowed {
                assert_eq!(result.unwrap().runs.len(), 5);
            } else {
                assert_eq!(code(result), ErrorCode::Forbidden);
            }
        }
        let mut session = structured(&owner, &fixture).await;
        let (_, socket, auth) = credential(&owner, &mut session).await;
        let agent = as_agent(&socket, &auth).await;
        assert_eq!(
            agent
                .preview_schedule(request(None))
                .await
                .unwrap()
                .runs
                .len(),
            5
        );
        owner.kill(session.id).await.unwrap();
        node.stop().await.unwrap();
        relay.stop().await;
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn delegated_preview_uses_the_targets_calculation_and_effective_read_scope() {
    within(async {
        let relay = Relay::start().await;
        let sources = tempfile::tempdir().unwrap();
        let home = crate::across::home(&relay, sources.path()).await;
        let readable = crate::across::member_granting(
            &relay,
            &home,
            "preview-readable",
            vec![Scope::Sync, Scope::Relay],
            vec![Scope::Read],
        )
        .await;
        let unreadable = crate::across::member_granting(
            &relay,
            &home,
            "preview-unreadable",
            vec![Scope::Sync, Scope::Relay],
            vec![Scope::Operate],
        )
        .await;
        let explicit = request(Some("2026-01-01T00:00:00Z".parse().unwrap()));
        let expected = readable
            .owner
            .preview_schedule(explicit.clone())
            .await
            .unwrap();
        let received = home
            .owner
            .call_on(readable.id(), explicit.clone())
            .await
            .unwrap();
        assert_eq!(received, expected);
        assert_eq!(
            code(home.owner.call_on(unreadable.id(), explicit).await),
            ErrorCode::Forbidden
        );
        unreadable.node.stop().await.unwrap();
        readable.node.stop().await.unwrap();
        home.node.stop().await.unwrap();
        relay.stop().await;
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_old_connection_gets_no_timezone_and_cannot_run_preview() {
    use futures_util::{SinkExt as _, StreamExt as _};
    use tokio_tungstenite::tungstenite::{Message, client::IntoClientRequest as _};
    use vessel_proto::{
        ClientFrame, GetNode, Hello, Outcome, ProtocolRange, ProtocolVersion, Request as _,
        RequestId, ServerFrame,
    };
    within(async {
        let fixture = Fixture::new();
        let node = fixture.start().await;
        let endpoint = format!("ws://{}/rpc", node.address().unwrap());
        let mut upgrade = endpoint.into_client_request().unwrap();
        let mut bearer = format!("Bearer {}", fixture.token().expose())
            .parse::<tokio_tungstenite::tungstenite::http::HeaderValue>()
            .unwrap();
        bearer.set_sensitive(true);
        let _previous = upgrade.headers_mut().insert("authorization", bearer);
        let (mut socket, _) = tokio_tungstenite::connect_async(upgrade).await.unwrap();
        let version = ProtocolVersion::new(12);
        let hello = ClientFrame::Hello(Hello {
            protocol: ProtocolRange {
                min: version,
                max: version,
            },
            client: client(),
        });
        socket
            .send(Message::text(serde_json::to_string(&hello).unwrap()))
            .await
            .unwrap();
        let welcome = socket.next().await.unwrap().unwrap().into_text().unwrap();
        let ServerFrame::Welcome(welcome) = serde_json::from_str(&welcome).unwrap() else {
            panic!("expected welcome");
        };
        assert_eq!(welcome.protocol, version);
        assert_eq!(welcome.node.timezone, None);
        for (id, call) in [(1, GetNode.into_call()), (2, request(None).into_call())] {
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
            let response = socket.next().await.unwrap().unwrap().into_text().unwrap();
            let ServerFrame::Response { outcome, .. } = serde_json::from_str(&response).unwrap()
            else {
                panic!("expected reply");
            };
            if id == 1 {
                let Outcome::Ok(vessel_proto::Reply::NodeInfo(info)) = outcome else {
                    panic!("expected node info");
                };
                assert_eq!(info.timezone, None);
            } else {
                let Outcome::Err(error) = outcome else {
                    panic!("expected protocol refusal");
                };
                assert_eq!(error.code, ErrorCode::Unsupported);
            }
        }
        socket.close(None).await.unwrap();
        node.stop().await.unwrap();
    })
    .await;
}
