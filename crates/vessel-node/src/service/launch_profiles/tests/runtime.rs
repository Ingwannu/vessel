//! Real fake-driver probes and completed partial application, without any prompt.
use super::*;
use crate::principal::Principal;
use vessel_proto::launch_profiles::*;
use vessel_proto::{ErrorCode, Handler as _, HarnessKind, Origin, OriginBy, Scope};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn selected_probe_and_live_bundle_use_actual_acp_choices_and_refusals() {
    vessel_testkit::within(
        SAFETY,
        Box::pin(async {
            let fixture = Fixture::new().await;
            let service = fixture.service();
            let owner = fixture.context(Principal::Owner).await;
            let read = fixture
                .context(Principal::Device {
                    id: vessel_proto::DeviceId::random(),
                    scopes: vec![Scope::Read],
                })
                .await;
            let selection = LaunchSelection::new(
                HarnessKind::ClaudeCode.into(),
                None,
                bundle(&[("model", "fake-large"), ("mode", "plan")]),
            );
            let catalogue = phase(
                "actual selected probe",
                service.launch_profile_options(
                    &read.connection,
                    GetLaunchProfileOptions {
                        selection: selection.clone(),
                    },
                ),
            )
            .await
            .unwrap();
            assert_eq!(catalogue.selection(), &selection);
            assert_eq!(catalogue.account().kind(), LaunchAccountKind::Default);
            assert!(
                catalogue
                    .options()
                    .as_slice()
                    .iter()
                    .any(|option| option.id.as_str() == "model"
                        && option.current_name() == Some("Fake-large"))
            );
            assert!(
                phase(
                    "probe makes no persistent session",
                    service.list_sessions(&owner.connection, vessel_proto::ListSessions {})
                )
                .await
                .unwrap()
                .is_empty()
            );
            let missing = LaunchSelection::new(
                HarnessKind::ClaudeCode.into(),
                Some("missing".parse().unwrap()),
                LaunchChoices::default(),
            );
            assert_eq!(
                phase(
                    "missing account never falls back",
                    service.launch_profile_options(
                        &owner.connection,
                        GetLaunchProfileOptions { selection: missing }
                    )
                )
                .await
                .unwrap_err()
                .code,
                ErrorCode::InvalidRequest
            );
            let preset: ReplaceLaunchProfiles =
                serde_json::from_str(vessel_testkit::launch_profile_fixtures::REPLACE).unwrap();
            let stored = phase(
                "save real fixture file",
                service.replace_launch_profiles(&owner.connection, preset),
            )
            .await
            .unwrap();
            let handle = fixture.running(&owner.connection).await;
            let id = handle.id();
            let eligible = phase(
                "node filters applicable account",
                service.applicable_launch_profiles(
                    &read.connection,
                    GetApplicableLaunchProfiles { session: id },
                ),
            )
            .await
            .unwrap();
            assert_eq!(eligible.profiles(), stored.profiles());

            let request = ApplySessionOptions {
                session: id,
                options: bundle(&[
                    ("model", "fake-large"),
                    ("mode", "locked"),
                    ("effort", "low"),
                ]),
            };
            assert_eq!(
                phase(
                    "Read cannot apply",
                    service.apply_session_options(&read.connection, request.clone())
                )
                .await
                .unwrap_err()
                .code,
                ErrorCode::Forbidden
            );
            let agent = fixture
                .context(Principal::delegated(
                    vessel_proto::DeviceId::random(),
                    &Scope::ALL,
                    Origin {
                        node: vessel_proto::NodeId::random(),
                        by: OriginBy::Agent {
                            session: vessel_proto::SessionId::random(),
                            depth: 0,
                        },
                        scopes: Scope::ALL.to_vec(),
                    },
                    vessel_proto::NodeId::random(),
                ))
                .await;
            assert_eq!(
                phase(
                    "unrelated agent cannot apply",
                    service.apply_session_options(&agent.connection, request.clone())
                )
                .await
                .unwrap_err()
                .code,
                ErrorCode::Forbidden
            );
            let result = phase(
                "actual ACP prefix and failure",
                service.apply_session_options(&owner.connection, request),
            )
            .await
            .unwrap();
            assert_eq!(
                result.applied().as_slice(),
                bundle(&[("model", "fake-large")]).as_slice()
            );
            assert_eq!(
                result.failure().unwrap().choice.as_choice().id.as_str(),
                "mode"
            );
            assert_eq!(result.failure().unwrap().error.code, ErrorCode::Conflict);
            let options = phase("read actual process options", handle.options())
                .await
                .unwrap();
            assert!(options.iter().any(|option| option.id.as_str() == "model"
                && option.current_name() == Some("Fake-large")));
            assert!(
                options.iter().any(|option| option.id.as_str() == "effort"
                    && option.current_name() == Some("High"))
            );
            phase("stop actual session", service.0.sessions.kill(id))
                .await
                .unwrap();
            assert!(
                phase(
                    "ended process does not accept later bundle",
                    service.apply_session_options(
                        &owner.connection,
                        ApplySessionOptions {
                            session: id,
                            options: bundle(&[("mode", "plan")])
                        }
                    )
                )
                .await
                .is_err()
            );
            agent.finish().await;
            read.finish().await;
            owner.finish().await;
            fixture.finish().await;
        }),
    )
    .await
    .unwrap();
}
